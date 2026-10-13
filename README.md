<div align="center">

# 🦀 ezAgents

**High-Performance Model Context Protocol (MCP) Agent Orchestration Engine in Rust**  
*Scoped tool boundaries, state checkpointing, human-in-the-loop gates, and deterministic verifiers.*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust 1.80+](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)
[![MCP Spec](https://img.shields.io/badge/MCP-2024--11--05-cyan.svg)](https://modelcontextprotocol.io/)
[![Product of EzLabs](https://img.shields.io/badge/Product%20of-EzLabs-0ea5e9.svg)](https://ezlabs.tech)

[**Website**](https://ezlabs.tech) • [**ezRouter**](https://github.com/namhv94/ezRouter) • [**ezEval**](https://github.com/namhv94/ezEval) • [**Issues**](https://github.com/namhv94/ezAgents/issues)

</div>

---

## 💡 The Problem

Most autonomous AI agents suffer from **unbounded tool execution, hallucinated success, and brittle state recovery**:
* Agents loop indefinitely, exceeding token and time budgets without bounded termination.
* LLMs declare *"All tests passed successfully"* while running zero actual assertions.
* Path traversal bugs allow agents to escape workspaces and mutate host environments.
* A single tool crash corrupts the entire context window with no clean rollback.

## ⚡ The Solution: ezAgents

**ezAgents** is an open-source execution harness built in **Rust** by **[EzLabs](https://ezlabs.tech)**. It decouples the model's intent from the execution environment:
* **The model proposes actions** via standard **Model Context Protocol (MCP)** JSON-RPC 2.0 schemas.
* **Deterministic, compiled Rust application code enforces permissions, state transitions, filesystem bounds, and physical verification.**

> **Core Philosophy:** *Give agents a task. Get an auditable change, not a promise.*

---

## 🏛️ System Architecture

```text
 ┌────────────────────────────────────────────────────────┐
 │                   Agent Task Prompt                    │
 └───────────────────────────┬────────────────────────────┘
                             │
                             ▼
 ┌────────────────────────────────────────────────────────┐
 │            ezAgents Rust State Machine                 │
 │   Draft ➔ Planning ➔ [Human Approval] ➔ Running       │
 └───────────────────────────┬────────────────────────────┘
                             │
              ┌──────────────┴──────────────┐
              ▼                             ▼
   ┌───────────────────────┐   ┌──────────────────────────┐
   │    MCP JSON-RPC 2.0   │   │     Scoped Sandbox       │
   │  Standard Tool Layer  │   │   Workspace Isolation    │
   │  - tools/list         │   │   Path Traversal Barrier │
   │  - tools/call         │   │   Max Step & Size Bounds │
   └──────────┬────────────┘   └────────────┬─────────────┘
              │                             │
              └──────────────┬──────────────┘
                             ▼
 ┌────────────────────────────────────────────────────────┐
 │             Deterministic Verification Gate            │
 │   - Physical File Existence Checks                     │
 │   - Exact Content / Regex Assertions                   │
 │   - Real Subprocess Exit Codes (Zero LLM Verdicts)     │
 └───────────────────────────┬────────────────────────────┘
                             │
                    ┌────────┴────────┐
                    ▼                 ▼
             [ All Passed ]    [ Any Failure ]
                    │                 │
                    ▼                 ▼
                Completed         RolledBack / Failed
```

---

## ✨ Key Features

- **Blazing Fast Native Rust**: Compiled binary with sub-millisecond execution overhead and zero garbage collection pauses.
- **Standard MCP Protocol (2024-11-05)**: Full client and server implementation of Anthropic's Model Context Protocol over clean JSON-RPC 2.0.
- **Finite State Machine & Checkpointing**: Transitions through immutable steps with instant SHA-256 cryptographic rollback on step failures.
- **Scoped Sandbox**: Restricts agent disk access strictly to designated workspaces. Canonicalized path isolation blocks `../` path traversal.
- **Human-in-the-Loop Approval Gates**: Interactive approval checkpoints before destructive actions or branch publications.
- **Deterministic Verifiers**: Verifies output with real test executions and file assertions. Never accepts simulated LLM verdicts.

---

## 🚀 Quick Start (60 Seconds)

### 1. Build from Source

```bash
git clone https://github.com/namhv94/ezAgents.git
cd ezAgents
cargo build --release
```

### 2. Inspect Registered MCP Tools

```bash
cargo run --release -- inspect
```

*Output:*
```text
=== ezAgents MCP Tool Catalog (Rust v0.1.0) ===
Server: ezagents-rust-core (Protocol: 2024-11-05)
Registered Tools: 3

- list_files: List relative file paths within the workspace
- read_file: Read content from a file inside the isolated sandbox
- write_file: Write content to a file inside the isolated sandbox

Engine: Rust Tokio / Compiled Binary Ready.
```

### 3. Run a Bounded Agent Task via CLI

```bash
cargo run --release -- run --task-id demo_01 --workspace ./my_workspace --require-approval
```

---

## 💻 Rust API Usage

```rust
use ezagents::{
    AgentOrchestrator, OrchestratorConfig, RunState,
};
use serde_json::json;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Configure the bounded execution environment
    let config = OrchestratorConfig {
        task_id: "fix-auth-header".to_string(),
        workspace_path: PathBuf::from("./sandboxed_repo"),
        require_plan_approval: true,
        max_steps: 15,
        sandbox_policy: None,
    };
    let mut orchestrator = AgentOrchestrator::new(config)?;

    // 2. Planning phase with human approval gate
    orchestrator.plan_phase(json!({
        "goal": "Refactor auth parser",
        "target": "auth.rs"
    }))?;
    orchestrator.approve_plan()?;

    // 3. Model executes scoped MCP tool calls
    orchestrator.execute_step(
        "write_file",
        json!({
            "path": "auth.rs",
            "content": "pub fn parse_auth(h: &str) -> &str { h.trim() }"
        }),
    )?;

    // 4. Attach deterministic verification checks (not LLM assertions!)
    orchestrator.verifier.add_file_exists_check("auth.rs");
    orchestrator.verifier.add_content_match_check("auth.rs", "parse_auth");

    let (passed, results) = orchestrator.verify_and_complete()?;
    println!("Task Status: {:?}", if passed { "SUCCESS" } else { "FAILED" });
    Ok(())
}
```

---

## 🧪 Running Tests

```bash
cargo test
```

Output:
```text
running 4 tests
test test_mcp_protocol_flow ... ok
test test_state_machine_transitions_and_rollback ... ok
test test_sandbox_path_traversal_prevention ... ok
test test_orchestrator_end_to_end ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

---

## 🌐 EzLabs Ecosystem

| Project | Description | Stack | Status |
| :--- | :--- | :---: | :--- |
| **[ezRouter](https://github.com/namhv94/ezRouter)** | Ultra-fast LLM routing proxy with Token Saver & multi-provider failover | **Rust** | `v0.1.0` Open Source |
| **[ezAgents](https://github.com/namhv94/ezAgents)** | MCP-native Agent Orchestration Engine with deterministic verifiers | **Rust** | `v0.1.0` Active |
| **[ezEval](https://github.com/namhv94/ezEval)** | Boundary fuzzing, JSON Schema validation & safety guardrails | **Rust** | `v0.1.0` Active |

---

## 📄 License

Distributed under the MIT License. See [LICENSE](LICENSE) for more information.

Developed with ❤️ by **[EzLabs](https://ezlabs.tech)**.
