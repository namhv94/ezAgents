//! ezAgents: Production-grade Model Context Protocol (MCP) agent orchestration engine.
//! A Product of EzLabs (https://ezlabs.tech)

pub mod mcp;
pub mod orchestrator;
pub mod sandbox;
pub mod state;
pub mod verifier;

pub use mcp::{MCPClient, MCPServer, ToolDefinition};
pub use orchestrator::{AgentOrchestrator, OrchestratorConfig, OrchestratorError};
pub use sandbox::{SandboxError, SandboxPolicy, ScopedSandbox};
pub use state::{RunState, StateCheckpoint, StateError, StateMachine};
pub use verifier::{DeterministicVerifier, VerificationResult};
