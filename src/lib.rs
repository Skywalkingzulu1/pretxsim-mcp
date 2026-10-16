//! PreTxSim adapter for Amazon Developer Hackathon - Alexa+ MCP server / Agent Skill.
//!
//! Air-gapped structural inspection for the Amazon Developer Hackathon hackathon.
//! Exposes the McpAdapter adapter plus the shared Adapter core.

pub mod adapters;
pub use adapters::{Adapter, AdapterResult, AdapterVerdict};
pub use adapters::mcp::McpAdapter;
