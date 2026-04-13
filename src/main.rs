// Copyright (c) 2026 Sandy McArthur, Jr.
// SPDX-License-Identifier: MIT

use std::collections::HashSet;
use std::sync::Arc;

use clap::Parser;
use rmcp::ServiceExt;
use tracing::{info, warn};

mod mam;
mod oauth;
mod tools;

#[derive(Parser, Debug)]
#[command(name = "myanonamouse-mcp", about = "MCP server for MyAnonamouse", version)]
struct Cli {
    /// MyAnonamouse session cookie value (mam_id). Obtain from the Security tab of your Preferences on MyAnonamouse.
    #[arg(long, env = "MAM_SESSION", required_unless_present = "list_tools")]
    mam_session: Option<String>,

    /// MCP transport to use
    #[arg(long, default_value = "stdio")]
    transport: Transport,

    /// Bind address for HTTP transport (e.g. 0.0.0.0:8080 to expose on all interfaces)
    #[arg(long, default_value = "127.0.0.1:8080")]
    http_bind: String,

    /// Bearer token required for HTTP transport requests (recommended)
    #[arg(long, env = "MAM_API_TOKEN")]
    api_token: Option<String>,

    /// OAuth 2.1 issuer URL (e.g. https://mcp.example.com). Enables the embedded OAuth
    /// authorization server for HTTP transport. When set alongside --api-token, both OAuth
    /// and static Bearer token auth are accepted, and the api-token doubles as the consent
    /// page access code.
    #[arg(long, env = "MAM_OAUTH_ISSUER")]
    oauth_issuer: Option<String>,

    /// Enable the search_torrents (full cross-category power search) and list_categories tools.
    #[arg(long, default_value_t = false)]
    enable_power_tools: bool,

    /// Enable the get_user_data and get_user_bonus_history tools.
    #[arg(long, default_value_t = false)]
    enable_user_tools: bool,

    /// Enable the update_seedbox_ip tool.
    #[arg(long, default_value_t = false)]
    enable_seedbox: bool,

    /// Enable a specific tool by name. Can be repeated. Applied before --disable-tool.
    #[arg(long = "enable-tool", value_name = "TOOL")]
    enable_tool: Vec<String>,

    /// Disable a specific tool by name. Can be repeated. Applied last; wins over all enable flags.
    #[arg(long = "disable-tool", value_name = "TOOL")]
    disable_tool: Vec<String>,

    /// Print all available tool names with their default group membership and exit.
    #[arg(long, default_value_t = false)]
    list_tools: bool,

    /// Verify the session cookie is valid and exit.
    #[arg(long, default_value_t = false)]
    test_connection: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum Transport {
    Stdio,
    Http,
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

    let cli = Cli::parse();

    if cli.list_tools {
        eprintln!("Available tools (* = enabled by default):\n");
        for &(name, group, default) in tools::TOOL_REGISTRY {
            let marker = if default { "*" } else { " " };
            eprintln!("  {marker} {name:<30} ({})", group);
        }
        eprintln!("\nGroup flags: --enable-power-tools, --enable-user-tools, --enable-seedbox");
        eprintln!("Per-tool:    --enable-tool=<name>  --disable-tool=<name>");
        return Ok(());
    }

    info!("Starting myanonamouse-mcp");

    let mam_session = cli.mam_session.expect("--mam-session is required unless --list-tools is set");
    let client = Arc::new(mam::build_client(&mam_session)?);

    if cli.test_connection {
        let ip_info = mam::get_ip_info(&client).await?;
        eprintln!("Connection OK. IP: {}, ASN: {}", ip_info.ip, ip_info.asn_string());
        return Ok(());
    }

    // --- Compute the enabled tool set ---
    // Start from defaults defined in TOOL_REGISTRY; group flags and per-tool flags add/remove.
    // --disable-tool always wins (applied last).
    let mut enabled_tools: HashSet<String> = tools::TOOL_REGISTRY
        .iter()
        .filter(|(_, _, default)| *default)
        .map(|(name, _, _)| name.to_string())
        .collect();

    // Group flag: --enable-power-tools
    // (search_torrents and get_torrent_details are default today; this flag is a forward-compat
    //  hook so configs remain valid once per-category tools replace them as the default.)
    if cli.enable_power_tools {
        for (name, group, _) in tools::TOOL_REGISTRY {
            if *group == "power" {
                enabled_tools.insert(name.to_string());
            }
        }
    }
    // Group flag: --enable-user-tools
    if cli.enable_user_tools {
        for (name, group, _) in tools::TOOL_REGISTRY {
            if *group == "user" {
                enabled_tools.insert(name.to_string());
            }
        }
    }
    // Group flag: --enable-seedbox
    if cli.enable_seedbox {
        for (name, group, _) in tools::TOOL_REGISTRY {
            if *group == "seedbox" {
                enabled_tools.insert(name.to_string());
            }
        }
    }
    // Per-tool enables
    for tool in &cli.enable_tool {
        enabled_tools.insert(tool.clone());
    }
    // Per-tool disables (always wins)
    for tool in &cli.disable_tool {
        enabled_tools.remove(tool);
    }

    let mut sorted_tools: Vec<&str> = enabled_tools.iter().map(|s| s.as_str()).collect();
    sorted_tools.sort();
    info!(tools = ?sorted_tools, "Enabled tools");

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

            if cli.api_token.is_none() && cli.oauth_issuer.is_none() {
                warn!(
                    "HTTP transport started without --api-token or --oauth-issuer. \
                     Anyone who can reach this port can use this server."
                );
            }

            info!(bind = %cli.http_bind, "Starting MCP server on HTTP");

            let mcp_service = {
                let client = client.clone();
                StreamableHttpService::new(
                    move || Ok(tools::MamServer::new(client.clone(), enabled_tools.clone())),
                    Arc::new(LocalSessionManager::default()),
                    StreamableHttpServerConfig::default(),
                )
            };

            let app = if let Some(oauth_issuer) = cli.oauth_issuer {
                // --- OAuth 2.1 mode (optionally dual-mode with static api_token) ---
                info!(issuer = %oauth_issuer, "OAuth 2.1 authorization server enabled");

                let oauth_state = Arc::new(
                    oauth::state::OAuthState::new(oauth_issuer, cli.api_token.clone()),
                );

                // Spawn background cleanup task
                let _cleanup = oauth::cleanup::spawn_cleanup(oauth_state.clone());

                // MCP route with OAuth auth middleware and permissive CORS
                let mcp_router = Router::new()
                    .nest_service("/mcp", mcp_service)
                    .layer(middleware::from_fn_with_state(
                        oauth_state.clone(),
                        oauth::middleware::oauth_auth_middleware,
                    ))
                    .layer(CorsLayer::permissive());

                // OAuth routes (unauthenticated, no CORS)
                let oauth_router = oauth::oauth_routes(oauth_state);

                // Merge: OAuth routes first (more specific), then MCP
                oauth_router
                    .merge(mcp_router)
                    .layer(TraceLayer::new_for_http())
            } else if cli.api_token.is_some() {
                // --- Static Bearer token mode (existing behavior) ---
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
                                .map(|t| {
                                    use subtle::ConstantTimeEq;
                                    t.as_bytes().ct_eq(expected.as_bytes()).into()
                                })
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

                Router::new()
                    .nest_service("/mcp", mcp_service)
                    .layer(auth_middleware)
                    .layer(TraceLayer::new_for_http())
            } else {
                // --- No auth ---
                Router::new()
                    .nest_service("/mcp", mcp_service)
                    .layer(TraceLayer::new_for_http())
            };

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
