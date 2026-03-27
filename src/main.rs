use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::bail;
use clap::Parser;
use rmcp::ServiceExt;
use tracing::{info, warn};

mod mam;
mod tools;

/// All tool names exposed by this server, in a stable order used for logging.
const ALL_TOOLS: &[&str] = &[
    "search_torrents",
    "get_user_data",
    "get_user_bonus_history",
    "get_ip_info",
    "update_seedbox_ip",
];

/// Tools that are disabled unless explicitly enabled via --enable-tool.
const DEFAULT_DISABLED: &[&str] = &[
    "update_seedbox_ip",
];

#[derive(Parser, Debug)]
#[command(name = "myanonamouse-mcp", about = "MCP server for MyAnonamouse", version)]
struct Cli {
    /// MyAnonamouse session cookie value (mam_id). Obtain from the Security tab of your Preferences on MyAnonamouse.
    #[arg(long, env = "MAM_SESSION")]
    mam_session: String,

    /// Enable tools matching a pattern (min 3 chars, substring of tool name).
    /// Comma-separated or repeated: --enable-tool=update_seedbox  or  --enable-tool update_seedbox
    #[arg(long = "enable-tool", alias = "enable-tools", value_name = "PATTERN", value_delimiter = ',', action = clap::ArgAction::Append)]
    enable: Vec<String>,

    /// Disable tools matching a pattern (min 3 chars, substring of tool name).
    /// Disabled by default: update_seedbox_ip
    #[arg(long = "disable-tool", alias = "disable-tools", value_name = "PATTERN", value_delimiter = ',', action = clap::ArgAction::Append)]
    disable: Vec<String>,

    /// MCP transport to use
    #[arg(long, default_value = "stdio")]
    transport: Transport,

    /// Bind address for HTTP transport (e.g. 0.0.0.0:8080)
    #[arg(long, default_value = "0.0.0.0:8080")]
    http_bind: String,

    /// Bearer token required for HTTP transport requests (recommended)
    #[arg(long, env = "MAM_API_TOKEN")]
    api_token: Option<String>,

    /// List all tools with their default enabled/disabled state and exit
    #[arg(long, default_value_t = false)]
    list_tools: bool,

    /// Verify the session cookie is valid and exit
    #[arg(long, default_value_t = false)]
    test_connection: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum Transport {
    Stdio,
    Http,
}

/// Scan raw CLI args in order and return `(is_enable, pattern)` pairs.
/// Clap cannot preserve relative ordering between two different repeated flags,
/// so we read `std::env::args()` directly for this purpose.
fn parse_tool_flags_in_order() -> anyhow::Result<Vec<(bool, String)>> {
    let args: Vec<String> = std::env::args().collect();
    let mut result = Vec::new();
    let mut i = 1usize;
    while i < args.len() {
        let arg = &args[i];
        let (is_enable, patterns_str) =
            if let Some(v) = arg.strip_prefix("--enable-tool=").or_else(|| arg.strip_prefix("--enable-tools=")) {
                (true, v.to_string())
            } else if arg == "--enable-tool" || arg == "--enable-tools" {
                if i + 1 < args.len() {
                    i += 1;
                    (true, args[i].clone())
                } else {
                    bail!("{} requires a value", arg);
                }
            } else if let Some(v) = arg.strip_prefix("--disable-tool=").or_else(|| arg.strip_prefix("--disable-tools=")) {
                (false, v.to_string())
            } else if arg == "--disable-tool" || arg == "--disable-tools" {
                if i + 1 < args.len() {
                    i += 1;
                    (false, args[i].clone())
                } else {
                    bail!("{} requires a value", arg);
                }
            } else {
                i += 1;
                continue;
            };

        for pattern in patterns_str.split(',') {
            let pattern = pattern.trim();
            if pattern.is_empty() {
                continue;
            }
            if pattern.len() < 3 {
                bail!(
                    "Tool pattern '{}' is too short (minimum 3 characters). \
                     Available tools: {}",
                    pattern,
                    ALL_TOOLS.join(", ")
                );
            }
            let matches: Vec<&str> = ALL_TOOLS
                .iter()
                .filter(|&&t| t.contains(pattern))
                .copied()
                .collect();
            if matches.is_empty() {
                bail!(
                    "No tools match pattern '{}'. Available tools: {}",
                    pattern,
                    ALL_TOOLS.join(", ")
                );
            }
            result.push((is_enable, pattern.to_string()));
        }
        i += 1;
    }
    Ok(result)
}

/// Apply ordered enable/disable flags to the default tool state.
/// Last flag wins per tool.
fn resolve_enabled_tools(ordered_flags: Vec<(bool, String)>) -> HashSet<String> {
    let mut state: HashMap<&str, bool> = ALL_TOOLS
        .iter()
        .map(|&t| (t, !DEFAULT_DISABLED.contains(&t)))
        .collect();

    for (is_enable, pattern) in &ordered_flags {
        for &tool in ALL_TOOLS.iter().filter(|&&t| t.contains(pattern.as_str())) {
            state.insert(tool, *is_enable);
        }
    }

    state
        .into_iter()
        .filter(|(_, enabled)| *enabled)
        .map(|(t, _)| t.to_string())
        .collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Logging must go to stderr — stdout is reserved for MCP JSON-RPC framing
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Handle --list-tools before clap parsing so the session cookie isn't required
    if std::env::args().any(|a| a == "--list-tools") {
        eprintln!("{:<26} {}", "TOOL", "DEFAULT");
        eprintln!("{}", "-".repeat(38));
        for &tool in ALL_TOOLS {
            let default = if DEFAULT_DISABLED.contains(&tool) { "disabled" } else { "enabled" };
            eprintln!("{:<26} {}", tool, default);
        }
        return Ok(());
    }

    let ordered_flags = parse_tool_flags_in_order()?;
    let enabled_tools = resolve_enabled_tools(ordered_flags);

    let cli = Cli::parse();

    // Log effective tool permissions at startup
    let enabled_list: Vec<&str> = ALL_TOOLS.iter().copied().filter(|&t| enabled_tools.contains(t)).collect();
    let disabled_list: Vec<&str> = ALL_TOOLS.iter().copied().filter(|&t| !enabled_tools.contains(t)).collect();
    info!(enabled = %enabled_list.join(", "), "Enabled tools");
    if !disabled_list.is_empty() {
        warn!(disabled = %disabled_list.join(", "), "Disabled tools");
    }

    info!("Starting myanonamouse-mcp");

    let client = Arc::new(mam::build_client(&cli.mam_session)?);

    if cli.test_connection {
        let ip_info = mam::get_ip_info(&client).await?;
        eprintln!("Connection OK. IP: {}, ASN: {}", ip_info.ip, ip_info.asn_string());
        return Ok(());
    }

    match cli.transport {
        Transport::Stdio => {
            info!("Starting MCP server on stdio");
            let server = tools::MamServer::new(client, enabled_tools);
            let service = server.serve(rmcp::transport::stdio()).await?;
            service.waiting().await?;
        }

        Transport::Http => {
            use axum::Router;
            use axum::extract::{Request, State};
            use axum::http::StatusCode;
            use axum::middleware::{self, Next};
            use axum::response::Response;
            use rmcp::transport::streamable_http_server::{
                StreamableHttpService, StreamableHttpServerConfig,
            };
            use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
            use tower_http::cors::CorsLayer;
            use tower_http::trace::TraceLayer;

            if cli.api_token.is_none() {
                warn!(
                    "HTTP transport started without --api-token. \
                     Anyone who can reach this port can use this server."
                );
            }

            info!(bind = %cli.http_bind, "Starting MCP server on HTTP");

            let mcp_service = {
                let client = client.clone();
                let enabled_tools = enabled_tools.clone();
                StreamableHttpService::new(
                    move || Ok(tools::MamServer::new(client.clone(), enabled_tools.clone())),
                    Arc::new(LocalSessionManager::default()),
                    StreamableHttpServerConfig::default(),
                )
            };

            let api_token = cli.api_token.clone();
            let auth_middleware = middleware::from_fn_with_state(
                api_token,
                |State(token): State<Option<String>>,
                 request: Request,
                 next: Next| async move {
                    if let Some(expected) = token {
                        let authorized = request
                            .headers()
                            .get(axum::http::header::AUTHORIZATION)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.strip_prefix("Bearer "))
                            .map(|t| t == expected)
                            .unwrap_or(false);

                        if !authorized {
                            return Response::builder()
                                .status(StatusCode::UNAUTHORIZED)
                                .body(axum::body::Body::from("Unauthorized"))
                                .unwrap();
                        }
                    }
                    next.run(request).await
                },
            );

            let app = Router::new()
                .nest_service("/mcp", mcp_service)
                .layer(auth_middleware)
                .layer(CorsLayer::permissive())
                .layer(TraceLayer::new_for_http());

            let listener = tokio::net::TcpListener::bind(&cli.http_bind).await?;
            info!("Listening on http://{}/mcp", cli.http_bind);

            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    tokio::signal::ctrl_c()
                        .await
                        .expect("failed to listen for ctrl-c");
                    info!("Shutting down HTTP server");
                })
                .await?;
        }
    }

    Ok(())
}
