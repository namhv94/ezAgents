use clap::{Parser, Subcommand};
use ezagents::mcp::MCPClient;
use ezagents::orchestrator::{AgentOrchestrator, OrchestratorConfig};
use serde_json::json;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "ezagents",
    version = "0.1.0",
    about = "ezAgents: Model Context Protocol (MCP) Agent Orchestration Engine (EzLabs)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inspect available Model Context Protocol (MCP) tools
    Inspect,

    /// Execute a bounded agent task inside an isolated sandbox
    Run {
        #[arg(long, default_value = "task_001")]
        task_id: String,

        #[arg(long, default_value = "./workspace")]
        workspace: PathBuf,

        #[arg(long, default_value_t = false)]
        require_approval: bool,

        #[arg(long, default_value_t = 20)]
        max_steps: usize,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect => {
            let tmp_dir = std::env::temp_dir().join("ezagents_probe");
            let config = OrchestratorConfig {
                task_id: "probe".to_string(),
                workspace_path: tmp_dir,
                require_plan_approval: false,
                max_steps: 10,
                sandbox_policy: None,
            };
            let orch = AgentOrchestrator::new(config)?;
            let mut client = MCPClient::new(&orch.mcp_server);
            let tools = client.list_tools().map_err(|e| e.to_string())?;

            println!("=== ezAgents MCP Tool Catalog (Rust v0.1.0) ===");
            println!(
                "Server: {} (Protocol: {})",
                orch.mcp_server.name, orch.mcp_server.protocol_version
            );
            println!("Registered Tools: {}\n", tools.len());
            for t in tools {
                println!("- {}: {}", t.name, t.description);
                println!("  Schema: {}", t.input_schema);
            }
            println!("\nEngine: Rust Tokio / Compiled Binary Ready.");
        }
        Commands::Run {
            task_id,
            workspace,
            require_approval,
            max_steps,
        } => {
            println!("🚀 Initializing ezAgents Task [{}]...", task_id);
            println!("📁 Workspace: {}", workspace.display());

            let config = OrchestratorConfig {
                task_id: task_id.clone(),
                workspace_path: workspace,
                require_plan_approval: require_approval,
                max_steps,
                sandbox_policy: None,
            };

            let mut orch = AgentOrchestrator::new(config)?;
            println!("⚡ State: {:?}", orch.state_machine.current_state);

            // Plan
            let plan_state = orch.plan_phase(json!({
                "objective": "Scaffold manifest and verify",
                "steps": ["write_file", "verify"]
            }))?;
            println!("📋 Plan submitted. Current State: {:?}", plan_state);

            if plan_state == ezagents::RunState::AwaitingApproval {
                println!(
                    "⏳ Awaiting human approval gate. Auto-approving for CLI demonstration..."
                );
                orch.approve_plan()?;
                println!("✅ Approved. State: {:?}", orch.state_machine.current_state);
            }

            // Execute MCP tool step
            let res = orch.execute_step(
                "write_file",
                json!({
                    "path": "ezagents_manifest.json",
                    "content": "{\"engine\":\"ezagents-rust\",\"status\":\"active\"}"
                }),
            )?;
            println!(
                "✍️ Executed MCP step: write_file -> ezagents_manifest.json (result: {})",
                res
            );

            // Deterministic verification
            orch.verifier
                .add_file_exists_check("ezagents_manifest.json");
            orch.verifier
                .add_content_match_check("ezagents_manifest.json", "ezagents-rust");

            let (passed, results) = orch.verify_and_complete()?;

            println!(
                "\n🔍 Deterministic Verification: {}",
                if passed {
                    "PASSED (All verified)"
                } else {
                    "FAILED"
                }
            );
            for r in results {
                println!(
                    "  [{}] {}: {} ({:.2}ms)",
                    if r.passed { "✓" } else { "✗" },
                    r.name,
                    r.details,
                    r.duration_ms
                );
            }

            println!("\n🏁 Final State: {:?}", orch.state_machine.current_state);
        }
    }

    Ok(())
}
