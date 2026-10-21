# PreTxSim — Voice Commerce Pre-Execution Firewall

**Amazon Developer Hackathon 2026 — Alexa+ Track**
A self-hosted [Model Context Protocol](https://modelcontextprotocol.io) server that acts as a zero-trust firewall for voice commerce transactions, validating every checkout **before** a payment signature is created.

- **Repo:** https://github.com/Skywalkingzulu1/pretxsim-mcp
- **License:** MIT
- **Language:** Rust
- **MCP SDK:** [`rmcp` 0.3](https://crates.io/crates/rmcp) (official Rust MCP SDK)
- **Transport:** Streamable HTTP (MCP spec 2025-11-25) via Axum
- **Demo video:** [`pretxsim_demo_final.mp4`](pretxsim_demo_final.mp4)

---

## Why

Voice assistants can be manipulated into purchasing the wrong item, at an inflated
price, or beyond a budget the user intended. PreTxSim sits between the assistant and
the merchant: the agent must pass a deterministic validation gate before any
transaction is signed or submitted.

```
User voice command
        │
        ▼
  Alexa+ / LLM Agent
        │  tools/call → agent_checkout
        ▼
┌──────────────────────────────┐
│   PreTxSim MCP Server        │
│  1. Intent Guard             │  Does the SKU match what the user asked for?
│  2. Price Oracle             │  Is the price within a sane band vs. baseline?
│  3. State Ledger             │  Does this exceed the session spend cap?
│  4. Tax Surface              │  Does tax/fee push the total over the cap?
└──────────────────────────────┘
        │  ALLOW | WARN | BLOCK
        ▼
   Merchant / Payment Signer
```

---

## The `agent_checkout` tool

### Input

| Field | Type | Description |
|---|---|---|
| `user_prompt` | string | Original user intent, e.g. `"Buy a replacement water filter"` |
| `sku_title` | string | SKU or item title returned by the merchant |
| `unit_price_cents` | u64 | Unit price in cents |
| `baseline_price_cents` | u64 | Baseline expected market price in cents |
| `quantity` | u32 | Quantity requested |
| `merchant` | string | Merchant or platform identifier |
| `region_code` | string | Region code for tax calculation (`ZA`, `US-CA`, …) |
| `session_id` | string | Unique session ID for cumulative spend tracking |
| `cumulative_spend_cents` | u64 | Spend in this session prior to this call |
| `session_cap_cents` | u64 | Session spend limit in cents |

### Output

```json
{
  "verdict": "ALLOW",
  "reason": "OK",
  "details": {
    "subtotal_cents": 2999,
    "tax_fee_cents": 239,
    "total_cents": 3238,
    "cap_cents": 10000
  }
}
```

**Verdicts:** `ALLOW` · `WARN` · `BLOCK`
**Reasons:** `OK` · `INTENT_MISMATCH` · `PRICE_ANOMALY` · `SESSION_CAP` · `TAX_OVERAGE`

### The 4 layers

1. **Intent Guard** — the prompt mentions a product class the returned SKU does not
   belong to (e.g. asked for a *water filter*, merchant returned an *air purifier*).
   → `BLOCK` / `INTENT_MISMATCH`
2. **Price Oracle** — unit price exceeds 2× the baseline market price.
   → `BLOCK` / `PRICE_ANOMALY`
3. **State Ledger** — projected session total exceeds the session cap.
   → `BLOCK` / `SESSION_CAP`
4. **Tax Surface** — subtotal fits the cap, but tax and fees push the total over it.
   → `BLOCK` / `TAX_OVERAGE`

All money is handled in integer cents. Tax rate is 15% for `ZA`, 8% otherwise.

---

## Run it

```bash
cargo run
```

The server binds to `0.0.0.0:3000` and exposes:

| Endpoint | Purpose |
|---|---|
| `POST /mcp` | Streamable HTTP MCP endpoint (the actual server) |
| `GET /health` | Liveness check → `OK` |

Transport runs in **stateless mode**, so every MCP request is independent — no
session headers required.

### Try it

```bash
# Health check
curl http://localhost:3000/health

# Initialize
curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d @init.json

# List tools
curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d @list_tools.json

# Call the tool — ALLOW
curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d @checkout_allow.json

# Call the tool — BLOCK
curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d @checkout_block_intent.json
```

Sample request payloads are in the repo root: `init.json`, `list_tools.json`,
`checkout_allow.json`, `checkout_block_intent.json`, `checkout_block_price.json`,
`checkout_block_cap.json`.

---

## Test

```bash
cargo test
```

9 tests pass — 7 covering every verdict path of the firewall (including ZA VAT
math and the tax-overage edge case), 1 covering the `/health` endpoint over a real
HTTP server, and 2 legacy adapter tests.

```
running 7 tests
test tests::test_agent_checkout_allow ... ok
test tests::test_agent_checkout_allow_za_vat ... ok
test tests::test_agent_checkout_intent_mismatch ... ok
test tests::test_agent_checkout_price_anomaly ... ok
test tests::test_agent_checkout_session_cap ... ok
test tests::test_agent_checkout_tax_overage ... ok
test tests::test_health_endpoint ... ok
```

---

## Architecture

```
src/
├── main.rs            # rmcp MCP server, PreTxSimFirewall, 4-layer validation, tests
├── lib.rs             # library root
└── adapters/          # legacy air-gapped EVM inspection adapter
    ├── mcp.rs
    ├── mod.rs
    └── registry.rs
```

The MCP server implements `rmcp::ServerHandler` directly. `#[tool_router]` generates
the route table and `#[tool]` generates the JSON Schema for the tool input; the
`call_tool` / `list_tools` handlers dispatch through the generated router.

```rust
impl ServerHandler for PreTxSimFirewall {
    async fn call_tool(&self, request: CallToolRequestParam, context: RequestContext<RoleServer>) -> Result<CallToolResult, McpError> {
        let ctx = ToolCallContext::new(self, request, context);
        Self::tool_router().call(ctx).await
    }
    // list_tools -> Self::tool_router().list_all()
}
```

Logging is written to **stderr only** so stdout stays clean for JSON-RPC framing.

---

## Demo assets

| File | Purpose |
|---|---|
| `pretxsim_demo_final.mp4` | Final submission video (2m 47s, with audio) |
| `architecture.jpg` | Architecture slide used as the video intro |
| `screen-capture (1)_2.mp4` | Raw terminal screen capture |
| `build_demo.py` | Splices slide + capture into the final video |
| `record_demo.py` | Local screen recorder (mss + OpenCV) |

---

## Dependencies

| Crate | Purpose |
|---|---|
| `rmcp` 0.3 | Official Rust MCP SDK — server handler, `#[tool]`/`#[tool_router]` macros, Streamable HTTP transport |
| `tokio` | Async runtime |
| `axum` 0.7 | HTTP server hosting the MCP endpoint |
| `tower-http` | CORS layer |
| `schemars` 1 | JSON Schema generation for the tool input schema |
| `serde` / `serde_json` | Serialization of params and verdicts |
| `tracing` | Structured logging |

---

## License

MIT — see [LICENSE](LICENSE).
