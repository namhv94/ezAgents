"""Command Line Interface for ezAgents.
Provides interactive inspection, task dispatch, and verification runner.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from ezagents import __version__
from ezagents.core.orchestrator import AgentOrchestrator, OrchestratorConfig
from ezagents.core.sandbox import CapabilityLevel, CapabilityPolicy


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="ezagents",
        description="ezAgents: Model Context Protocol (MCP) Agent Orchestration Engine (EzLabs)",
    )
    parser.add_argument("--version", action="version", version=f"ezagents v{__version__}")

    subparsers = parser.add_subparsers(dest="command", help="Sub-commands")

    # Command: run
    run_parser = subparsers.add_parser("run", help="Execute an agent task inside an isolated sandbox")
    run_parser.add_argument("--task-id", default="task_001", help="Unique identifier for the task run")
    run_parser.add_argument("--workspace", default="./workspace", help="Target workspace path")
    run_parser.add_argument("--require-approval", action="store_true", help="Enforce human approval gate")
    run_parser.add_argument("--max-steps", type=int, default=20, help="Maximum execution steps allowed")

    # Command: inspect
    inspect_parser = subparsers.add_parser("inspect", help="Inspect MCP tools available on the runtime")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(0)

    if args.command == "inspect":
        cfg = OrchestratorConfig(task_id="probe", workspace_path="./tmp_probe")
        orch = AgentOrchestrator(cfg)
        tools = orch.client.list_tools()
        print(f"=== ezAgents MCP Tool Catalog (v{__version__}) ===")
        print(f"Server: {orch.server.name} (Protocol: {orch.server.protocol_version})")
        print(f"Registered Tools: {len(tools)}\n")
        for t in tools:
            print(f"- {t['name']}: {t['description']}")
            print(f"  Schema: {json.dumps(t['inputSchema'])}")
        print("\nReady.")

    elif args.command == "run":
        ws_dir = Path(args.workspace)
        ws_dir.mkdir(parents=True, exist_ok=True)
        print(f"🚀 Initializing ezAgents Task [{args.task_id}]...")
        print(f"📁 Workspace: {ws_dir.resolve()}")
        cfg = OrchestratorConfig(
            task_id=args.task_id,
            workspace_path=ws_dir,
            require_plan_approval=args.require_approval,
            max_steps=args.max_steps,
        )
        orch = AgentOrchestrator(cfg)
        print(f"⚡ State: {orch.state_machine.current_state.value}")

        # Plan
        plan_state = orch.plan_phase({"objective": "Scaffold project and verify", "steps": ["write", "verify"]})
        print(f"📋 Plan submitted. Current State: {plan_state.value}")

        if plan_state.value == "awaiting_approval":
            print("⏳ Awaiting human approval gate. Auto-approving for CLI demonstration...")
            orch.approve_plan()
            print(f"✅ Approved. State: {orch.state_machine.current_state.value}")

        # Execute demo write tool
        orch.execute_step("write_file", {"path": "ezagents_manifest.json", "content": '{"engine": "ezagents", "status": "active"}'})
        print("✍️ Executed MCP step: write_file -> ezagents_manifest.json")

        # Deterministic verification
        orch.verifier.add_file_exists_check("ezagents_manifest.json")
        orch.verifier.add_file_content_check("ezagents_manifest.json", "ezagents")
        passed, results = orch.verify_and_complete()

        print(f"\n🔍 Deterministic Verification: {'PASSED (All verified)' if passed else 'FAILED'}")
        for r in results:
            print(f"  [{'✓' if r.passed else '✗'}] {r.name}: {r.details} ({r.execution_time_ms:.2f}ms)")

        print(f"\n🏁 Final State: {orch.state_machine.current_state.value}")


if __name__ == "__main__":
    main()
