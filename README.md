<div align="center">

# 🦀 ezAgents

**High-Performance Model Context Protocol (MCP) Agent Orchestration Engine in Rust**  
*Deterministic boundaries, cryptographic state rollback, human approval gates, and physical verifiers.*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust 1.80+](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)
[![MCP Spec](https://img.shields.io/badge/MCP-2024--11--05-cyan.svg)](https://modelcontextprotocol.io/)
[![CI](https://github.com/namhv94/ezAgents/actions/workflows/ci.yml/badge.svg)](https://github.com/namhv94/ezAgents/actions)
[![Product of EzLabs](https://img.shields.io/badge/Product%20of-EzLabs-0ea5e9.svg)](https://ezlabs.tech)

[**Website**](https://ezlabs.tech) • [**ezRouter**](https://github.com/namhv94/ezRouter) • [**ezEval**](https://github.com/namhv94/ezEval) • [**Issues**](https://github.com/namhv94/ezAgents/issues)

</div>

---

## 💡 The Core Problem

Most autonomous AI agents (built on Python frameworks or raw API scripts) suffer from **catastrophic blind spots**:
* **The "Hallucinated Success" Trap**: Models frequently claim *"Task finished! All 24 test cases passed!"* when no tests were actually executed.
* **Unbounded Loop Disasters**: A subtle parsing edge-case traps an agent in an infinite tool-calling loop, burning hundreds of dollars in API credits.
* **Workspace Escapes**: Without strict canonical path containment, agents execute `write_file("../../etc/...")` or overwrite critical host binaries.
* **Irreversible Corruption**: One flawed step in a 10-step refactor leaves the codebase in a half-mutated, broken state with zero rollback mechanism.

---

## 🎯 Real-World Case Studies: Before vs. After

### 🔍 Case Study 1: The "Silent Disaster" Database Migration

* **The Prompt**: `"Refactor user billing schema to add nullable tax_id and update repository queries."`

```text
❌ WITHOUT ezAgents (Standard Python Agent / LangChain / CrewAI)
────────────────────────────────────────────────────────────────────────
1. Agent proposes plan: "Modify migration file and test against database."
2. Tool Execution: Agent runs bash command: `rm -rf ./migrations && alembic init`
   ↳ Accidentally wipes out historical schema versions.
3. LLM Final Output: "Migration applied successfully! Database is healthy."
4. Result: Production database schema broken. No audit trail, no recovery.

✅ WITH ezAgents (Rust Core & MCP Protocol)
────────────────────────────────────────────────────────────────────────
1. Planning Phase: Agent generates proposed changes.
2. Approval Gate: State transitions to `AwaitingApproval`. Agent pauses.
   ↳ Human reviewer inspects proposed file list and budget parameters.
3. Bounded Execution: Scoped Sandbox permits writes strictly within `repo/migrations/`.
   ↳ Any traversal attempts (`../`) are aborted immediately at the OS level.
4. Deterministic Physical Verifier:
   ↳ `DeterministicVerifier` runs `cargo test` / `pytest` in an isolated subprocess.
   ↳ Verifier checks physical exit code == 0 (rejecting LLM verbal promises).
5. Automatic SHA-256 Rollback:
   ↳ If any check fails, state reverts to previous cryptographic snapshot in 0.01ms.
```

---

### 🔍 Case Study 2: Long-Running Autonomous Bug Fixing & Token Blowouts

* **The Prompt**: `"Reproduce GitHub Issue #142 (Race condition in WebSocket handler) and patch."`

```text
❌ WITHOUT ezAgents
────────────────────────────────────────────────────────────────────────
* Agent makes a code change. Tests fail.
* Agent tries again, gets confused, imports wrong libraries, and loops 48 times.
* Bill: $85.00 in Anthropic/OpenAI API credits before developer manually kills process.
* Repository left in uncommitted, dirty state.

✅ WITH ezAgents
────────────────────────────────────────────────────────────────────────
* Step Budget Enforcement: Configured `max_steps: 15`.
* At step 15, if physical verifiers haven't passed, state transitions cleanly to `Failed`.
* Execution halted immediately, preserving full event ledger and checkpoint diffs.
* Developer receives a structured audit report without runaway expenses.
```

---

## 📊 Comprehensive Comparison Matrix

| Feature | Raw LLM Scripts | LangChain / CrewAI | **ezAgents (EzLabs)** |
| :--- | :---: | :---: | :---: |
| **Language & Runtime** | Python (Interpreted) | Python (Heavy, slow) | **Pure Native Rust (Zero GC pauses)** |
| **Tool Protocol** | Custom / Ad-hoc | Custom abstractions | **Anthropic MCP Standard (2024-11-05)** |
| **State Transitions** | Implicit dictionary | Mutable graph | **Strict Finite State Machine (FSM)** |
| **Rollback Capability** | ❌ None | ❌ None | **✅ Cryptographic SHA-256 Snapshots** |
| **Verification Gate** | LLM self-evaluation | LLM self-evaluation | **Deterministic Physical Assertions (Code/Files)** |
| **Sandbox Isolation** | ❌ None | Partial / Process only | **Strict Canonical Lexical Path Sandbox** |
| **Execution Latency** | High (Python startup) | 200ms - 800ms | **Sub-millisecond (< 0.05ms dispatch)** |
| **Human in the Loop** | Manual prompts | Complex callbacks | **Native `AwaitingApproval` Barrier** |

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

## 🚀 Quick Start (60 Seconds)

### 1. Build from Source

```bash
git clone https://github.com/namhv94/ezAgents.git
cd ezAgents
cargo build --release
```

### 2. Inspect Available MCP Tools

```bash
target/release/ezagents inspect
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
target/release/ezagents run --task-id task_01 --workspace ./sandbox_dir --require-approval
```

*Output:*
```text
🚀 Initializing ezAgents Task [task_01]...
📁 Workspace: /home/namhv/workspace/sandbox_dir
⚡ State: Draft
📋 Plan submitted. Current State: AwaitingApproval
⏳ Awaiting human approval gate. Auto-approving for CLI demonstration...
✅ Approved. State: Running
✍️ Executed MCP step: write_file -> ezagents_manifest.json (result: {"bytes_written":44,"status":"ok"})

🔍 Deterministic Verification: PASSED (All verified)
  [✓] FileExists:ezagents_manifest.json: File 'ezagents_manifest.json' exists (0.00ms)
  [✓] ContentMatch:ezagents_manifest.json: Substring 'ezagents-rust' found in 'ezagents_manifest.json' (0.01ms)

🏁 Final State: Completed
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

## 🧪 Verified Test Suite

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
