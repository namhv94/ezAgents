"""ezAgents - Production-grade Model Context Protocol (MCP) agent orchestration engine.
A Product of EzLabs (https://ezlabs.tech)
"""

__version__ = "0.1.0"
__author__ = "EzLabs Core Team"

from ezagents.core.mcp import MCPClient, MCPServer, ToolDefinition
from ezagents.core.state import StateMachine, RunState, StateCheckpoint
from ezagents.core.sandbox import ScopedSandbox, CapabilityPolicy
from ezagents.core.verifier import DeterministicVerifier, VerificationResult
from ezagents.core.orchestrator import AgentOrchestrator, OrchestratorConfig

__all__ = [
    "MCPClient",
    "MCPServer",
    "ToolDefinition",
    "StateMachine",
    "RunState",
    "StateCheckpoint",
    "ScopedSandbox",
    "CapabilityPolicy",
    "DeterministicVerifier",
    "VerificationResult",
    "AgentOrchestrator",
    "OrchestratorConfig",
]
