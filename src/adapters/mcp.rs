//! Amazon MCP server / Agent Skill proxy (Amazon Developer Hackathon —
//! Alexa+ MCP server / Agent Skill track).

use crate::adapters::{Adapter, AdapterResult, AdapterVerdict};
use serde_json::Value;

pub struct McpAdapter;

impl Adapter for McpAdapter {
    fn name(&self) -> &'static str {
        "mcp"
    }


    fn inspect(&self, tx: &Value) -> AdapterResult {
        let mut reasons: Vec<String> = Vec::new();
        let method = tx.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let params = tx
            .get("params")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let arguments = params
            .get("arguments")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        let details = serde_json::json!({
            "method": method,
            "skill": name,
            "arguments": arguments,
        });

        if method != "tools/call" {
            reasons.push(format!("non-skill MCP method: {}", method));
        }

        if name.is_empty() {
            reasons.push("agent skill name missing — cannot enforce allowlist".to_string());
        }

        let destructive = arguments.get("destructive").and_then(|v| v.as_bool()).unwrap_or(false);
        if destructive {
            reasons.push("destructive agent action — requires explicit confirmation".to_string());
        }

        let spend = arguments.get("spend_usd").and_then(|v| v.as_u64()).unwrap_or(0);
        if spend > 100 {
            reasons.push(format!("agent spend ${} exceeds default agent skill cap", spend));
        }

        let verdict = if reasons.is_empty() { AdapterVerdict::Allow } else { AdapterVerdict::Block };

        AdapterResult { target: "mcp".to_string(), verdict, reasons, details }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn benign_skill_allowed() {
        let tx = json!({
            "method": "tools/call",
            "params": {"name": "read_file", "arguments": {"path": "/tmp/x"}}
        });
        let r = McpAdapter.inspect(&tx);
        assert_eq!(r.verdict, AdapterVerdict::Allow);
    }

    #[test]
    fn destructive_spend_blocked() {
        let tx = json!({
            "method": "tools/call",
            "params": {"name": "pay_vendor", "arguments": {"destructive": true, "spend_usd": 250}}
        });
        let r = McpAdapter.inspect(&tx);
        assert_eq!(r.verdict, AdapterVerdict::Block);
    }
}
