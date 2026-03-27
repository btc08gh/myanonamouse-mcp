# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

This is an MCP (Model Context Protocol) server for interacting with the MyAnonamouse private torrent tracker. The project is written in Rust.

## Commands

```bash
# Build the project
cargo build
# Run the project
cargo run
# Run tests
cargo test
# Build documentation
cargo doc --open
```

## Tech Stack

Rust for the code. Cargo for package management and build system.

## Dependencies

| Crate | Purpose |
|---|---|
| `rmcp` | Official MCP Rust SDK — features: `server`, `transport-io`, `transport-streamable-http-server`, `schemars` |
| `tokio` | Async runtime — features: `full` |
| `reqwest` | HTTP client for MAM API calls — features: `json`, `cookies` |
| `serde` | Serialization — features: `derive` |
| `serde_json` | JSON support |
| `anyhow` | Error propagation |
| `thiserror` | Custom error types |
| `axum` | HTTP server for HTTP/SSE transport |
| `tower-http` | Middleware — features: `cors`, `trace` |
| `tracing` | Logging |
| `tracing-subscriber` | Log output formatting — **must be configured to write to stderr or a file, never stdout**. Any output on stdout corrupts the JSON-RPC framing used by the MCP stdio transport. — features: `env-filter` |
| `clap` | CLI args — features: `derive`, `env` |

## Authentication

MyAnonamouse uses a **session cookie** named `mam_id`. There is no login endpoint or token exchange — the user must obtain the cookie value manually by logging into the site in a browser, then provide it to the server.

- **Supply via:** `--mam-session <value>` CLI arg or `MAM_SESSION` environment variable
- **Transmission:** Injected as a `Cookie: mam_id=<value>` header on every outbound HTTP request
- **Headers set on every request:**
  - `Cookie: mam_id=<value>`
  - `Content-Type: application/json`
  - `User-Agent: Mozilla/5.0` (browser spoof required by MAM)
- **No session management:** The cookie is assumed to be valid; there is no refresh or re-auth logic
- **Base URL:** `https://www.myanonamouse.net`

## Architecture

```
MCP Client (Claude Desktop, etc.)
        ↕ MCP (stdio or HTTP/SSE)
  MamServer (src/tools/mod.rs)
        ↕ Arc<reqwest::Client>
  MAM HTTP API (www.myanonamouse.net)
```

- `MamServer` holds an `Arc<reqwest::Client>` (pre-configured with the `mam_id` cookie and User-Agent) and a `HashSet<String>` of enabled tool names
- `MamServer` derives `Clone` (required for HTTP session factory)
- A single `reqwest::Client` is shared across all tool handlers; it is built once at startup

## File Structure

| Path | Purpose |
|---|---|
| `Cargo.toml` | Manifest — dependencies, metadata, binary definition |
| `Cargo.lock` | Exact dependency versions (kept for binaries) |
| `src/main.rs` | Entry point — CLI arg parsing, transport selection, server startup, HTTP auth middleware |
| `src/tools/mod.rs` | All MCP tool implementations + `ServerHandler` impl |
| `src/mam/mod.rs` | MAM HTTP client — `reqwest::Client` construction, shared request logic |
| `tests/` | Integration tests |
| `api-docs/` | MAM API documentation (HTML) |

## Tool Implementation Pattern (rmcp macros)

Three procedural macros from `rmcp` drive tool registration:

```rust
// tools/mod.rs

#[derive(Clone)]
pub struct MamServer {
    client: Arc<reqwest::Client>,
    enabled_tools: std::collections::HashSet<String>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl MamServer {
    /// Tool description visible to the LLM — write it as a complete sentence.
    #[tool]
    async fn my_tool(&self, Parameters(p): Parameters<MyParams>) -> Result<String, String> {
        self.tool_gate("my_tool")?;
        // ...
        Ok(result_string)
    }
}

#[tool_handler]
impl ServerHandler for MamServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::default().with_server_info(Implementation::new(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
    }
}
```

### Parameter structs

```rust
#[derive(Deserialize, schemars::JsonSchema)]
struct MyParams {
    /// Doc comment becomes the JSON Schema description visible to the LLM.
    required_field: String,
    /// Optional fields use Option<T>.
    optional_field: Option<String>,
    /// Bool fields that default false use #[serde(default)].
    #[serde(default)]
    some_flag: bool,
}
```

- Tool methods always return `Result<String, String>` — `Ok(String)` is the result text, `Err(String)` is the error returned to the LLM
- Call `self.tool_gate("tool_name")?` at the top of every tool to enforce enable/disable filtering
- Append `[Hint: ...]` to error messages where LLM guidance is valuable (e.g. how to recover)

## CLI Args Pattern

```rust
#[derive(Parser, Debug)]
#[command(name = "myanonamouse-mcp", about = "MCP server for MyAnonamouse")]
struct Cli {
    #[arg(long, env = "MAM_SESSION")]
    mam_session: String,

    #[arg(long = "enable-tool", alias = "enable-tools", value_name = "PATTERN",
          value_delimiter = ',', action = clap::ArgAction::Append)]
    enable: Vec<String>,

    #[arg(long = "disable-tool", alias = "disable-tools", value_name = "PATTERN",
          value_delimiter = ',', action = clap::ArgAction::Append)]
    disable: Vec<String>,

    #[arg(long, default_value = "stdio")]
    transport: Transport,

    #[arg(long, default_value = "0.0.0.0:8080")]
    http_bind: String,

    #[arg(long, env = "MAM_API_TOKEN")]
    api_token: Option<String>,

    #[arg(long, default_value_t = false)]
    list_tools: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum Transport { Stdio, Http }
```

- `--list-tools` is handled by scanning `std::env::args()` directly before full parsing, so it exits without requiring credentials
- `--enable-tool`/`--disable-tool` ordering is preserved by a raw args scan (`parse_tool_flags_in_order`) producing `Vec<(bool, String)>`, then `resolve_enabled_tools` applies them with last-wins semantics

## Error Handling

- **`anyhow`** — internal propagation: `Result<T>` with `?`, `bail!()`, `anyhow!()`
- **`thiserror`** — custom error enums for domain-specific error types
- Tool methods convert `anyhow::Error` to `String` at the boundary via `.map_err(|e| e.to_string())` or an enrichment function
- Known error cases get `[Hint: ...]` suffixes so the LLM knows whether/how to recover:

```rust
fn enrich_mam_error(status: u16, body: &str) -> String {
    let hint = match status {
        401 => Some("The mam_id session cookie is invalid or expired. Ask the user to provide a fresh cookie."),
        429 => Some("Rate limited. Wait before retrying."),
        _ => None,
    };
    match hint {
        Some(h) => format!("HTTP {status}: {body}\n[Hint: {h}]"),
        None => format!("HTTP {status}: {body}"),
    }
}
```

## Logging

```rust
tracing_subscriber::fmt()
    .with_writer(std::io::stderr)   // CRITICAL: never stdout — corrupts MCP JSON-RPC framing
    .with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()),
    )
    .init();
```

Use structured fields, not format strings in the message:
```rust
info!(host = %url, "Starting myanonamouse-mcp");
warn!(tool = %name, "Tool disabled");
```

Never use `println!`. All output goes to stderr via `tracing`.

## Transport Setup

**Stdio (default):**
```rust
let service = server.serve(rmcp::transport::stdio()).await?;
service.waiting().await?;
```

**HTTP:**
```rust
let mcp_service = StreamableHttpService::new(
    move || Ok(MamServer::new(client.clone(), enabled_tools.clone())),
    Arc::new(LocalSessionManager::default()),
    StreamableHttpServerConfig::default(),
);
// Mount at /mcp with Bearer token auth middleware, CORS, and tracing layers
```

MCP endpoint is mounted at `/mcp`. Layers: `TraceLayer` → `CorsLayer::permissive()` → auth middleware → `mcp_service`.
