use std::sync::Arc;

use clap::Parser;
use rmcp::ServiceExt;
use tracing::{info, warn};

mod mam;
mod tools;

#[derive(Parser, Debug)]
#[command(name = "myanonamouse-mcp", about = "MCP server for MyAnonamouse", version)]
struct Cli {
    /// MyAnonamouse session cookie value (mam_id). Obtain from the Security tab of your Preferences on MyAnonamouse.
    #[arg(long, env = "MAM_SESSION")]
    mam_session: String,

    /// MCP transport to use
    #[arg(long, default_value = "stdio")]
    transport: Transport,

    /// Bind address for HTTP transport (e.g. 0.0.0.0:8080)
    #[arg(long, default_value = "0.0.0.0:8080")]
    http_bind: String,

    /// Bearer token required for HTTP transport requests (recommended)
    #[arg(long, env = "MAM_API_TOKEN")]
    api_token: Option<String>,

    /// Verify the session cookie is valid and exit
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
            let server = tools::MamServer::new(client);
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
                StreamableHttpService::new(
                    move || Ok(tools::MamServer::new(client.clone())),
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
