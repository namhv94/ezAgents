use crate::mcp::{MCPClient, MCPServer};
use crate::sandbox::{SandboxError, SandboxPolicy, ScopedSandbox};
use crate::state::{RunState, StateError, StateMachine};
use crate::verifier::{DeterministicVerifier, VerificationResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Sandbox error: {0}")]
    Sandbox(#[from] SandboxError),
    #[error("State error: {0}")]
    State(#[from] StateError),
    #[error("MCP error: {0}")]
    Mcp(String),
    #[error("Execution budget exceeded (max {0} steps)")]
    BudgetExceeded(usize),
    #[error("Invalid execution state: {0}")]
    InvalidState(String),
}

pub struct OrchestratorConfig {
    pub task_id: String,
    pub workspace_path: PathBuf,
    pub require_plan_approval: bool,
    pub max_steps: usize,
    pub sandbox_policy: Option<SandboxPolicy>,
}

pub struct AgentOrchestrator {
    pub config: OrchestratorConfig,
    pub sandbox: Arc<Mutex<ScopedSandbox>>,
    pub state_machine: StateMachine,
    pub mcp_server: MCPServer,
    pub verifier: DeterministicVerifier,
}

impl AgentOrchestrator {
    pub fn new(config: OrchestratorConfig) -> Result<Self, OrchestratorError> {
        let sb = ScopedSandbox::new(&config.workspace_path, config.sandbox_policy.clone())?;
        let verifier = DeterministicVerifier::new(sb.workspace_root());
        let sandbox_arc = Arc::new(Mutex::new(sb));

        let mut mcp_server = MCPServer::new("ezagents-rust-core", "0.1.0");

        // Register default tools
        let sb_read = Arc::clone(&sandbox_arc);
        mcp_server.register_tool(
            "read_file",
            "Read content from a file inside the isolated sandbox",
            json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"]
            }),
            Arc::new(move |args: Value| {
                let path = args
                    .get("path")
                    .and_then(|p| p.as_str())
                    .ok_or("Missing path")?;
                let sb = sb_read.lock().map_err(|e| e.to_string())?;
                let content = sb.read_file(path).map_err(|e| e.to_string())?;
                Ok(Value::String(content))
            }),
        );

        let sb_write = Arc::clone(&sandbox_arc);
        mcp_server.register_tool(
            "write_file",
            "Write content to a file inside the isolated sandbox",
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"]
            }),
            Arc::new(move |args: Value| {
                let path = args
                    .get("path")
                    .and_then(|p| p.as_str())
                    .ok_or("Missing path")?;
                let content = args
                    .get("content")
                    .and_then(|c| c.as_str())
                    .ok_or("Missing content")?;
                let sb = sb_write.lock().map_err(|e| e.to_string())?;
                let written = sb.write_file(path, content).map_err(|e| e.to_string())?;
                Ok(json!({ "bytes_written": written, "status": "ok" }))
            }),
        );

        let sb_list = Arc::clone(&sandbox_arc);
        mcp_server.register_tool(
            "list_files",
            "List relative file paths within the workspace",
            json!({ "type": "object", "properties": {} }),
            Arc::new(move |_args: Value| {
                let sb = sb_list.lock().map_err(|e| e.to_string())?;
                let files = sb.list_files().map_err(|e| e.to_string())?;
                Ok(json!(files))
            }),
        );

        let state_machine =
            StateMachine::new(&config.task_id, json!({ "task_id": config.task_id }));

        Ok(Self {
            config,
            sandbox: sandbox_arc,
            state_machine,
            mcp_server,
            verifier,
        })
    }

    pub fn plan_phase(&mut self, proposed_plan: Value) -> Result<RunState, OrchestratorError> {
        self.state_machine
            .transition_to(RunState::Planning, Some(json!({ "plan": proposed_plan })))?;

        if self.config.require_plan_approval {
            self.state_machine
                .transition_to(RunState::AwaitingApproval, None)?;
            Ok(RunState::AwaitingApproval)
        } else {
            self.state_machine.transition_to(RunState::Running, None)?;
            Ok(RunState::Running)
        }
    }

    pub fn approve_plan(&mut self) -> Result<RunState, OrchestratorError> {
        if self.state_machine.current_state != RunState::AwaitingApproval {
            return Err(OrchestratorError::InvalidState(format!(
                "Cannot approve from state {:?}",
                self.state_machine.current_state
            )));
        }
        self.state_machine.transition_to(RunState::Running, None)?;
        Ok(RunState::Running)
    }

    pub fn execute_step(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> Result<String, OrchestratorError> {
        if self.state_machine.current_state != RunState::Running {
            return Err(OrchestratorError::InvalidState(format!(
                "Must be in Running state to execute steps, current: {:?}",
                self.state_machine.current_state
            )));
        }

        if self.state_machine.step_counter >= self.config.max_steps {
            let _ = self.state_machine.transition_to(RunState::Failed, None);
            return Err(OrchestratorError::BudgetExceeded(self.config.max_steps));
        }

        let mut client = MCPClient::new(&self.mcp_server);
        let res = client
            .call_tool(tool_name, arguments)
            .map_err(OrchestratorError::Mcp)?;

        self.state_machine.transition_to(
            RunState::Running,
            Some(json!({
                "last_tool": tool_name,
                "last_result": res.chars().take(200).collect::<String>()
            })),
        )?;

        Ok(res)
    }

    pub fn verify_and_complete(
        &mut self,
    ) -> Result<(bool, Vec<VerificationResult>), OrchestratorError> {
        self.state_machine
            .transition_to(RunState::Verifying, None)?;

        let results = self.verifier.run_all();
        let all_passed = results.iter().all(|r| r.passed);

        if all_passed {
            self.state_machine.transition_to(
                RunState::Completed,
                Some(json!({ "verification": results })),
            )?;
        } else {
            self.state_machine
                .transition_to(RunState::Failed, Some(json!({ "verification": results })))?;
        }

        Ok((all_passed, results))
    }
}
