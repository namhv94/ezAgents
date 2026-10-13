use ezagents::{
    AgentOrchestrator, MCPClient, MCPServer, OrchestratorConfig, RunState, SandboxError,
    ScopedSandbox, StateMachine,
};
use serde_json::json;
use std::sync::Arc;

#[test]
fn test_mcp_protocol_flow() {
    let mut server = MCPServer::new("test-server", "0.1.0");
    server.register_tool(
        "echo",
        "Echo input message",
        json!({
            "type": "object",
            "properties": { "msg": { "type": "string" } }
        }),
        Arc::new(|args| {
            let msg = args.get("msg").and_then(|m| m.as_str()).unwrap_or("");
            Ok(json!(format!("echo: {}", msg)))
        }),
    );

    let mut client = MCPClient::new(&server);
    let init_res = client.initialize().expect("Failed to initialize");
    assert_eq!(init_res["protocolVersion"], "2024-11-05");

    let tools = client.list_tools().expect("Failed to list tools");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");

    let output = client
        .call_tool("echo", json!({ "msg": "rust-power" }))
        .expect("Tool call failed");
    assert!(output.contains("rust-power"));
}

#[test]
fn test_state_machine_transitions_and_rollback() {
    let mut sm = StateMachine::new("test_sm", json!({ "count": 1 }));
    assert_eq!(sm.current_state, RunState::Draft);

    sm.transition_to(RunState::Planning, Some(json!({ "count": 2 })))
        .unwrap();
    assert_eq!(sm.current_state, RunState::Planning);

    sm.transition_to(RunState::Running, Some(json!({ "count": 3 })))
        .unwrap();
    assert_eq!(sm.current_state, RunState::Running);

    // Rollback to step 0
    sm.rollback_to_step(0).unwrap();
    assert_eq!(sm.current_state, RunState::RolledBack);
    assert_eq!(sm.data["count"], 1);
}

#[test]
fn test_sandbox_path_traversal_prevention() {
    let temp_dir = std::env::temp_dir().join("ezagents_sandbox_test");
    let sandbox = ScopedSandbox::new(&temp_dir, None).expect("Failed to create sandbox");

    sandbox
        .write_file("nested/test.txt", "safe-data")
        .expect("Safe write failed");
    let content = sandbox
        .read_file("nested/test.txt")
        .expect("Safe read failed");
    assert_eq!(content, "safe-data");

    // Path traversal should fail
    let bad_read = sandbox.read_file("../../etc/passwd");
    assert!(matches!(bad_read, Err(SandboxError::PathTraversal(_))));
}

#[test]
fn test_orchestrator_end_to_end() {
    let temp_dir = std::env::temp_dir().join("ezagents_orch_test");
    let config = OrchestratorConfig {
        task_id: "test_task".to_string(),
        workspace_path: temp_dir,
        require_plan_approval: false,
        max_steps: 10,
        sandbox_policy: None,
    };

    let mut orch = AgentOrchestrator::new(config).expect("Failed to init orchestrator");
    orch.plan_phase(json!({ "objective": "write file" }))
        .expect("Plan failed");

    orch.execute_step(
        "write_file",
        json!({
            "path": "code.rs",
            "content": "fn main() { println!(\"verified\"); }"
        }),
    )
    .expect("Step failed");

    orch.verifier.add_file_exists_check("code.rs");
    orch.verifier
        .add_content_match_check("code.rs", "fn main()");

    let (passed, results) = orch.verify_and_complete().expect("Verify failed");
    assert!(passed);
    assert_eq!(results.len(), 2);
    assert_eq!(orch.state_machine.current_state, RunState::Completed);
}
