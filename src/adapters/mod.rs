//! Shared Adapter core (vendored into each per-hackathon repo).
pub mod registry;
use serde_json::Value;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterVerdict { Allow, Block, Warn, }
impl AdapterVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            AdapterVerdict::Allow => "ALLOW",
            AdapterVerdict::Block => "BLOCK",
            AdapterVerdict::Warn => "WARN",
        }
    }
}
#[derive(Debug, Clone)]
pub struct AdapterResult {
    pub target: String, pub verdict: AdapterVerdict,
    pub reasons: Vec<String>, pub details: Value,
}
pub trait Adapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn inspect(&self, tx: &Value) -> AdapterResult;
}
pub mod mcp;
