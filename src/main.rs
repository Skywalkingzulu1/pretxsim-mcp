use rmcp::{
    handler::server::{ServerHandler, tool::Parameters, tool::ToolCallContext},
    model::{
        CallToolRequestParam, Content, CallToolResult, ListToolsResult, PaginatedRequestParam,
    },
    schemars::JsonSchema,
    service::RequestContext,
    tool, tool_router, ErrorData as McpError, RoleServer,
};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::net::SocketAddr;
use tracing::info;

// --- Input Parameter Schemas ---

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentCheckoutParams {
    #[schemars(description = "Original user prompt intent, e.g. 'Buy a replacement water filter'")]
    pub user_prompt: String,

    #[schemars(description = "SKU or item title returned by merchant")]
    pub sku_title: String,

    #[schemars(description = "Unit price of the item in cents")]
    pub unit_price_cents: u64,

    #[schemars(description = "Baseline expected market price in cents")]
    pub baseline_price_cents: u64,

    #[schemars(description = "Quantity requested")]
    pub quantity: u32,

    #[schemars(description = "Merchant or platform identifier")]
    pub merchant: String,

    #[schemars(description = "Region code for tax calculation, e.g. 'ZA' or 'US-CA'")]
    pub region_code: String,

    #[schemars(description = "Unique session ID for cumulative spend tracking")]
    pub session_id: String,

    #[schemars(description = "Cumulative spend in cents across this session prior to this call")]
    pub cumulative_spend_cents: u64,

    #[schemars(description = "Session spend limit in cents")]
    pub session_cap_cents: u64,
}

// --- Output Verdict Schema ---

#[derive(Debug, Serialize)]
pub struct VerdictDetails {
    pub subtotal_cents: u64,
    pub tax_fee_cents: u64,
    pub total_cents: u64,
    pub cap_cents: u64,
}

#[derive(Debug, Serialize)]
pub struct CheckoutVerdict {
    pub verdict: String, // "ALLOW" | "WARN" | "BLOCK"
    pub reason: String,  // "OK" | "INTENT_MISMATCH" | "PRICE_ANOMALY" | "SESSION_CAP" | "TAX_OVERAGE"
    pub details: VerdictDetails,
}

// --- PreTxSim Firewall Server ---

#[derive(Clone, Default)]
pub struct PreTxSimFirewall;

#[tool_router]
impl PreTxSimFirewall {
    #[tool(description = "Interacts as a zero-trust pre-execution firewall to validate voice commerce transactions before payment signature.")]
    async fn agent_checkout(
        &self,
        params: Parameters<AgentCheckoutParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;

        let subtotal = p.unit_price_cents * (p.quantity as u64);
        let tax_rate = if p.region_code == "ZA" { 0.15 } else { 0.08 };
        let tax_fee = (subtotal as f64 * tax_rate) as u64;
        let total = subtotal + tax_fee;
        let projected_session_total = p.cumulative_spend_cents + total;

        let details = VerdictDetails {
            subtotal_cents: subtotal,
            tax_fee_cents: tax_fee,
            total_cents: total,
            cap_cents: p.session_cap_cents,
        };

        if p.user_prompt.to_lowercase().contains("filter")
            && !p.sku_title.to_lowercase().contains("filter")
        {
            let verdict = CheckoutVerdict {
                verdict: "BLOCK".into(),
                reason: "INTENT_MISMATCH".into(),
                details,
            };
            let json = serde_json::to_string_pretty(&verdict).unwrap();
            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }

        if p.unit_price_cents > (p.baseline_price_cents * 2) {
            let verdict = CheckoutVerdict {
                verdict: "BLOCK".into(),
                reason: "PRICE_ANOMALY".into(),
                details,
            };
            let json = serde_json::to_string_pretty(&verdict).unwrap();
            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }

        if projected_session_total > p.session_cap_cents {
            let verdict = CheckoutVerdict {
                verdict: "BLOCK".into(),
                reason: if (p.cumulative_spend_cents + subtotal) <= p.session_cap_cents {
                    "TAX_OVERAGE"
                } else {
                    "SESSION_CAP"
                }
                .into(),
                details,
            };
            let json = serde_json::to_string_pretty(&verdict).unwrap();
            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }

        let verdict = CheckoutVerdict {
            verdict: "ALLOW".into(),
            reason: "OK".into(),
            details,
        };
        let json = serde_json::to_string_pretty(&verdict).unwrap();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

impl ServerHandler for PreTxSimFirewall {
    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let ctx = ToolCallContext::new(self, request, context);
        Self::tool_router().call(ctx).await
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = Self::tool_router().list_all();
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    info!("Initializing PreTxSim MCP Streamable HTTP Firewall...");

    let addr: SocketAddr = "0.0.0.0:3000".parse()?;
    info!("Starting server on http://{} /mcp", addr);

    let service: rmcp::transport::streamable_http_server::tower::StreamableHttpService<
        PreTxSimFirewall,
        rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
    > = rmcp::transport::streamable_http_server::tower::StreamableHttpService::new(
        || Ok(PreTxSimFirewall),
        Default::default(),
        rmcp::transport::streamable_http_server::StreamableHttpServerConfig {
            stateful_mode: false,
            sse_keep_alive: None,
        },
    );

    let app = axum::Router::new()
        .route("/health", axum::routing::get(|| async { "OK" }))
        .nest_service("/mcp", service)
        .layer(tower_http::cors::CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use reqwest::Client;
    use tower_http::cors::CorsLayer;

    async fn spawn_test_server() -> SocketAddr {
        let service: rmcp::transport::streamable_http_server::tower::StreamableHttpService<
            PreTxSimFirewall,
            rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
        > = rmcp::transport::streamable_http_server::tower::StreamableHttpService::new(
            || Ok(PreTxSimFirewall),
            Default::default(),
            rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default(),
        );

        let app: Router = Router::new()
            .route("/health", axum::routing::get(|| async { "OK" }))
            .nest_service("/mcp", service)
            .layer(CorsLayer::permissive());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        addr
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let addr = spawn_test_server().await;
        let client = Client::new();
        let resp = client.get(&format!("http://{}/health", addr)).send().await.unwrap();
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.text().await.unwrap(), "OK");
    }

    fn base_params() -> AgentCheckoutParams {
        AgentCheckoutParams {
            user_prompt: "Buy a water filter".into(),
            sku_title: "Replacement water filter".into(),
            unit_price_cents: 2999,
            baseline_price_cents: 2500,
            quantity: 1,
            merchant: "amazon".into(),
            region_code: "US".into(),
            session_id: "test-session".into(),
            cumulative_spend_cents: 0,
            session_cap_cents: 10000,
        }
    }

    async fn run_checkout(params: AgentCheckoutParams) -> serde_json::Value {
        let firewall = PreTxSimFirewall;
        let result = firewall.agent_checkout(Parameters(params)).await.unwrap();
        let text = result.content[0].as_text().unwrap().text.clone();
        serde_json::from_str(&text).unwrap()
    }

    #[tokio::test]
    async fn test_agent_checkout_allow() {
        let verdict = run_checkout(base_params()).await;
        assert_eq!(verdict["verdict"], "ALLOW");
        assert_eq!(verdict["reason"], "OK");
        assert_eq!(verdict["details"]["subtotal_cents"], 2999);
        assert_eq!(verdict["details"]["total_cents"], 2999 + 239);
    }

    #[tokio::test]
    async fn test_agent_checkout_intent_mismatch() {
        let mut p = base_params();
        p.user_prompt = "Buy a replacement water filter".into();
        p.sku_title = "Replacement air purifier".into();
        let verdict = run_checkout(p).await;
        assert_eq!(verdict["verdict"], "BLOCK");
        assert_eq!(verdict["reason"], "INTENT_MISMATCH");
    }

    #[tokio::test]
    async fn test_agent_checkout_price_anomaly() {
        let mut p = base_params();
        p.unit_price_cents = 6000;
        let verdict = run_checkout(p).await;
        assert_eq!(verdict["verdict"], "BLOCK");
        assert_eq!(verdict["reason"], "PRICE_ANOMALY");
    }

    #[tokio::test]
    async fn test_agent_checkout_session_cap() {
        let mut p = base_params();
        p.unit_price_cents = 5000;
        p.cumulative_spend_cents = 8000;
        let verdict = run_checkout(p).await;
        assert_eq!(verdict["verdict"], "BLOCK");
        assert_eq!(verdict["reason"], "SESSION_CAP");
    }

    #[tokio::test]
    async fn test_agent_checkout_tax_overage() {
        let mut p = base_params();
        p.unit_price_cents = 4500;
        p.baseline_price_cents = 3000;
        p.region_code = "ZA".into();
        p.cumulative_spend_cents = 5000;
        let verdict = run_checkout(p).await;
        assert_eq!(verdict["verdict"], "BLOCK");
        assert_eq!(verdict["reason"], "TAX_OVERAGE");
    }

    #[tokio::test]
    async fn test_agent_checkout_allow_za_vat() {
        let mut p = base_params();
        p.unit_price_cents = 8000;
        p.baseline_price_cents = 5000;
        p.region_code = "ZA".into();
        p.merchant = "takealot".into();
        let verdict = run_checkout(p).await;
        assert_eq!(verdict["verdict"], "ALLOW");
        assert_eq!(verdict["reason"], "OK");
        assert_eq!(verdict["details"]["tax_fee_cents"], 1200);
        assert_eq!(verdict["details"]["total_cents"], 9200);
    }
}
