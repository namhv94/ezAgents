<div align="center">

# 🤖 ezAgents

**Production-grade Model Context Protocol (MCP) Agent Orchestration Engine**  
*Scoped tool boundaries, state checkpointing, human-in-the-loop gates, and deterministic verifiers.*

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Python Version](https://img.shields.io/badge/python-3.9+-blue.svg)](https://www.python.org/)
[![MCP Spec](https://img.shields.io/badge/MCP-2024--11--05-cyan.svg)](https://modelcontextprotocol.io/)
[![Product of EzLabs](https://img.shields.io/badge/Product%20of-EzLabs-0ea5e9.svg)](https://ezlabs.tech)

[**Website**](https://ezlabs.tech) • [**ezRouter**](https://github.com/namhv94/ezRouter) • [**Documentation**](#quick-start) • [**Issues**](https://github.com/namhv94/ezAgents/issues)

</div>

---

## 💡 The Problem

Most autonomous AI agents suffer from **unbounded tool execution, hallucinated success, and brittle state recovery**:
* Agents loop indefinitely, exceeding token and time budgets without bounded termination.
* LLMs declare *"All tests passed successfully"* while running zero actual assertions.
* Path traversal bugs allow agents to escape workspaces and mutate host environments.
* A single tool crash corrupts the entire context window with no clean rollback.

## ⚡ The Solution: ezAgents

**ezAgents** is an open-source execution harness built by **[EzLabs](https://ezlabs.tech)**. It decouples the model's intent from the execution environment:
* **The model proposes actions** via standard **Model Context Protocol (MCP)** JSON-RPC 2.0 schemas.
* **Deterministic application code enforces permissions, state transitions, filesystem bounds, and physical verification.**

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
 │               ezAgents State Machine                   │
 │   DRAFT ➔ PLANNING ➔ [HUMAN APPROVAL] ➔ RUNNING       │
 └───────────────────────────┬────────────────────────────┘
                             │
              ┌──────────────┴──────────────┐
              ▼                             ▼
   ┌───────────────────────┐   ┌──────────────────────────┐
   │    MCP JSON-RPC 2.0   │   │     Scoped Sandbox       │
   │  Standard Tool Layer  │   │   Workspace Isolation    │
   │  - tools/list         │   │   Path Traversal Barrier │
   │  - tools/call         │   │   Max Step & Time Budget │
   └──────────┬────────────┘   └────────────┬─────────────┘
              │                             │
              └──────────────┬──────────────┘
                             ▼
 ┌────────────────────────────────────────────────────────┐
 │             Deterministic Verification Gate            │
 │   - Physical File Existence Checks                     │
 │   - Exact Content / Regex Assertions                   │
 │   - Real Exit Code Validations (No LLM Verdicts)       │
 └───────────────────────────┬────────────────────────────┘
                             │
                    ┌────────┴────────┐
                    ▼                 ▼
             [ All Passed ]    [ Any Failure ]
                    │                 │
                    ▼                 ▼
                COMPLETED         ROLLBACK / FAILED
```

---

## ✨ Key Features

- **Standard MCP Protocol (2024-11-05)**: Full client and server implementation of Anthropic's Model Context Protocol over clean JSON-RPC 2.0.
- **Finite State Machine & Checkpointing**: Transitions through immutable steps with instant cryptographic rollback on step failures.
- **Scoped Sandbox**: Restricts agent disk access strictly to designated workspaces. Completely blocks `../` path traversal and unauthorized escalation.
- **Human-in-the-Loop Approval Gates**: Interactive approval checkpoints before destructive actions or branch publications.
- **Deterministic Verifiers**: Verifies output with real test executions and file assertions. Never accepts simulated LLM verdicts.
- **Zero Heavy Dependencies**: Lightweight Python core running on standard libraries with blazing fast cold starts.

---

## 🚀 Quick Start (60 Seconds)

### 1. Installation

```bash
git clone https://github.com/namhv94/ezAgents.git
cd ezAgents
pip install -e .
```

### 2. Inspect Registered MCP Tools

```bash
ezagents inspect
```

*Output:*
```text
=== ezAgents MCP Tool Catalog (v0.1.0) ===
Server: ezagents-core (Protocol: 2024-11-05)
Registered Tools: 3

- read_file: Read content from a file inside the isolated sandbox
- write_file: Write content to a file inside the isolated sandbox
- list_files: List relative file paths within the workspace
```

### 3. Run a Bounded Agent Task via CLI

```bash
ezagents run --task-id demo_01 --workspace ./my_workspace --require-approval
```

---

## 💻 Python API Usage

```python
from ezagents import AgentOrchestrator, OrchestratorConfig, CapabilityPolicy

# 1. Configure the bounded execution environment
config = OrchestratorConfig(
    task_id="fix-issue-42",
    workspace_path="./sandboxed_repo",
    require_plan_approval=True,
    max_steps=15,
)
orchestrator = AgentOrchestrator(config)

# 2. Planning phase with human approval gate
orchestrator.plan_phase({
    "goal": "Refactor auth header parser",
    "target_files": ["auth.py"]
})
orchestrator.approve_plan()

# 3. Model executes scoped MCP tool calls
orchestrator.execute_step("write_file", {
    "path": "auth.py",
    "content": "def parse_auth(header): return header.strip()"
})

# 4. Attach deterministic verification checks (not LLM assertions!)
orchestrator.verifier.add_file_exists_check("auth.py")
orchestrator.verifier.add_file_content_check("auth.py", "def parse_auth")

passed, results = orchestrator.verify_and_complete()
print(f"Task Status: {'VERIFIED SUCCESS' if passed else 'VERIFICATION FAILED'}")
```

---

## 🧪 Running Tests

```bash
python3 -m unittest discover tests
```

Output:
```text
.........
----------------------------------------------------------------------
Ran 9 tests in 0.002s

OK
```

---

## 🌐 EzLabs Ecosystem

| Project | Description | Status |
| :--- | :--- | :--- |
| **[ezRouter](https://github.com/namhv94/ezRouter)** | Ultra-fast LLM routing proxy in Rust with Token Saver & multi-provider failover | `v0.1.0` Open Source |
| **[ezAgents](https://github.com/namhv94/ezAgents)** | MCP-native Agent Orchestration Engine with deterministic verifiers | `v0.1.0` Active |
| **[ezEval](https://github.com/namhv94/ezEval)** | Boundary fuzzing, JSON Schema validation & safety guardrails | `v0.1.0` Active |

---

## 📄 License

Distributed under the MIT License. See [LICENSE](LICENSE) for more information.

Developed with ❤️ by **[EzLabs](https://ezlabs.tech)**.
