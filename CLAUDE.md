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

MyAnonamouse uses a **session cookie** named `mam_id`. There is no login endpoint or token exchange — the user must obtain the cookie value from the Security tab of their Preferences on MyAnonamouse and provide it to the server.

- **Supply via:** `--mam-session <value>` CLI arg or `MAM_SESSION` environment variable
- **How to obtain:** Log into MyAnonamouse, go to Preferences → Security tab, copy the `mam_id` value
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
        ↕ shared HTTP client
  MAM HTTP API (www.myanonamouse.net)
```

A single HTTP client is built once at startup with the `mam_id` cookie and User-Agent pre-configured, then shared across all tool calls. The `MamServer` struct holds this shared client plus the set of enabled tool names, and is cloned per HTTP session when using the HTTP transport.

## File Structure

| Path | Purpose |
|---|---|
| `Cargo.toml` | Manifest — dependencies, metadata, binary definition |
| `Cargo.lock` | Exact dependency versions (kept for binaries) |
| `src/main.rs` | Entry point — CLI arg parsing, `--list-tools`, `--test-connection`, transport selection, server startup, HTTP auth middleware |
| `src/mam/mod.rs` | MAM HTTP client — builds the shared client, `get_ip_info` helper, `enrich_error` with LLM hints |
| `src/tools/mod.rs` | `MamServer` struct + all MCP tool implementations + server handler |
| `tests/` | Integration tests |
| `api-docs/` | MAM API documentation (HTML) |

## Tool Implementation Pattern

Tools are defined as async methods on `MamServer` using three rmcp procedural macros: `#[tool_router]` on the impl block, `#[tool]` on each tool method, and `#[tool_handler]` on the `ServerHandler` impl.

- Each tool method's doc comment becomes the tool description visible to the LLM — write it as a complete sentence
- Tool methods always return `Result<String, String>` — the Ok value is the result text sent to the LLM, the Err value is the error text
- Parameters are defined as structs deriving `Deserialize` and `schemars::JsonSchema` — each field's doc comment becomes its JSON Schema description visible to the LLM
- Optional parameters use `Option<T>`; boolean parameters that default to false use `#[serde(default)]`

## CLI Args

The server accepts these flags:

- `--mam-session` / `MAM_SESSION` env — the `mam_id` session cookie (required)
- `--transport` — `stdio` (default) or `http`
- `--http-bind` — bind address for HTTP transport (default `0.0.0.0:8080`)
- `--api-token` / `MAM_API_TOKEN` env — Bearer token for HTTP transport authentication
- `--test-connection` — verify the session cookie works, then exit

## Error Handling

- `anyhow` is used for all internal error propagation
- `thiserror` is used for any custom error enums
- Tool methods convert internal errors to plain strings at the boundary
- Known HTTP error codes get `[Hint: ...]` suffixes appended to their error messages so the LLM knows how to recover — for example, a 401 tells the user to refresh their `mam_id` from Preferences → Security, and a 429 tells the LLM to wait before retrying

## Logging

All log output goes to **stderr** via `tracing` — never stdout, which is reserved for MCP JSON-RPC framing. Logs use structured key-value fields rather than inline format strings. Log level defaults to INFO and respects the `RUST_LOG` environment variable. Never use `println!`.

## Transport

**Stdio** (default): the server speaks MCP over stdin/stdout. This is the transport used by Claude Desktop.

**HTTP**: the server listens on the configured bind address and exposes the MCP endpoint at `/mcp`. Each connection gets its own `MamServer` instance. Requests are authenticated via a Bearer token if `--api-token` is set. The server applies CORS and HTTP tracing middleware.
