"""Agent Orchestrator Loop.
Coordinates MCP Tool Execution, State Machine Transitions, Human Approval, and Deterministic Verifiers.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional

from ezagents.core.mcp import MCPClient, MCPServer
from ezagents.core.sandbox import CapabilityPolicy, ScopedSandbox
from ezagents.core.state import RunState, StateMachine
from ezagents.core.verifier import DeterministicVerifier, VerificationResult


@dataclass
class OrchestratorConfig:
    task_id: str
    workspace_path: str | Path
    require_plan_approval: bool = False
    max_steps: int = 15
    policy: CapabilityPolicy = field(default_factory=CapabilityPolicy)


class AgentOrchestrator:
    """Core orchestration engine managing the execution lifecycle of autonomous agents."""

    def __init__(self, config: OrchestratorConfig):
        self.config = config
        self.sandbox = ScopedSandbox(config.workspace_path, policy=config.policy)
        self.state_machine = StateMachine(run_id=config.task_id)
        self.server = MCPServer(name="ezagents-core", version="0.1.0")
        self._register_default_tools()
        self.client = MCPClient(self.server)
        self.verifier = DeterministicVerifier(self.sandbox.workspace_root)
        self.logger = logging.getLogger("ezagents.orchestrator")

    def _register_default_tools(self) -> None:
        """Expose standard sandboxed filesystem tools through MCP."""
        self.server.register_tool(
            name="read_file",
            description="Read content from a file inside the isolated sandbox",
            input_schema={
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"],
            },
            handler=lambda args: self.sandbox.read_file(args["path"]),
        )
        self.server.register_tool(
            name="write_file",
            description="Write content to a file inside the isolated sandbox",
            input_schema={
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"},
                },
                "required": ["path", "content"],
            },
            handler=lambda args: self.sandbox.write_file(args["path"], args["content"]),
        )
        self.server.register_tool(
            name="list_files",
            description="List relative file paths within the workspace",
            input_schema={
                "type": "object",
                "properties": {"path": {"type": "string", "default": "."}},
            },
            handler=lambda args: self.sandbox.list_files(args.get("path", ".")),
        )

    def plan_phase(self, proposed_plan: Dict[str, Any]) -> RunState:
        """Begin planning phase and transition state machine."""
        self.state_machine.transition_to(RunState.PLANNING, {"plan": proposed_plan})
        if self.config.require_plan_approval:
            self.state_machine.transition_to(RunState.AWAITING_APPROVAL)
            return RunState.AWAITING_APPROVAL
        self.state_machine.transition_to(RunState.RUNNING)
        return RunState.RUNNING

    def approve_plan(self) -> RunState:
        """Approve a pending plan to proceed into running state."""
        if self.state_machine.current_state != RunState.AWAITING_APPROVAL:
            raise RuntimeError(f"Cannot approve plan from state '{self.state_machine.current_state}'")
        self.state_machine.transition_to(RunState.RUNNING)
        return RunState.RUNNING

    def execute_step(self, tool_name: str, arguments: Dict[str, Any]) -> Any:
        """Execute a single bounded MCP tool call with state tracking."""
        if self.state_machine.current_state != RunState.RUNNING:
            raise RuntimeError(f"Agent must be in RUNNING state to execute tools, current: {self.state_machine.current_state}")

        if self.state_machine.step_counter >= self.config.max_steps:
            self.state_machine.transition_to(RunState.FAILED, {"reason": "Max step budget exhausted"})
            raise RuntimeError("Maximum allowed execution step budget exceeded")

        res = self.client.call_tool(tool_name, arguments)
        self.state_machine.transition_to(
            RunState.RUNNING,
            {"last_tool": tool_name, "last_result": str(res)[:500]},
        )
        return res

    def verify_and_complete(self) -> tuple[bool, List[VerificationResult]]:
        """Run all registered deterministic checks before marking task as completed."""
        self.state_machine.transition_to(RunState.VERIFYING)
        results = self.verifier.run_all()
        all_passed = all(r.passed for r in results) if results else True

        if all_passed:
            self.state_machine.transition_to(RunState.COMPLETED, {"verification_results": [r.__dict__ for r in results]})
        else:
            self.state_machine.transition_to(RunState.FAILED, {"verification_results": [r.__dict__ for r in results]})

        return all_passed, results
