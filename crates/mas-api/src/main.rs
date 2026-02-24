//! Multi-Agent System REST API Server
//!
//! Provides HTTP endpoints for registering, managing, and interacting
//! with multi-agent systems.

use std::net::SocketAddr;

use clap::Parser;
use mas_api::{create_router, AppState};
use tracing::{info, Level};

/// Multi-Agent System REST API Server
///
/// Provides HTTP endpoints for registering, managing, and interacting
/// with multi-agent systems.
#[derive(Parser, Debug)]
#[command(name = "mas-api")]
#[command(author, version, about, long_about = None)]
#[command(after_help = r#"API ENDPOINTS:

  Systems:
    POST   /api/v1/systems                Register a new system
    GET    /api/v1/systems                List all systems
    GET    /api/v1/systems/{name}         Get system details
    PUT    /api/v1/systems/{name}         Update a system
    DELETE /api/v1/systems/{name}         Remove a system
    POST   /api/v1/systems/{name}/prompt  Send a prompt (no session)

  Sessions:
    POST   /api/v1/sessions               Create a new session
    GET    /api/v1/sessions               List all sessions
    GET    /api/v1/sessions/{id}          Get session details
    DELETE /api/v1/sessions/{id}          Delete a session
    GET    /api/v1/sessions/{id}/history  Get conversation history
    GET    /api/v1/sessions/{id}/search   Semantic search in session
    POST   /api/v1/sessions/{id}/prompt   Send a prompt (with memory)
    POST   /api/v1/sessions/{id}/build-index  Build search index

EXAMPLES:
    mas-api                     Start server on 0.0.0.0:8080
    mas-api --port 3000         Start server on port 3000
    mas-api --host 127.0.0.1    Bind to localhost only
"#)]
struct Args {
    /// Host address to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

fn main() -> anyhow::Result<()> {
    // Build a custom tokio runtime with larger worker thread stacks.
    // The default 2 MB stack overflows when deeply-nested async futures
    // are involved (agent routing → forwarding → MCP client calls).
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_stack_size(8 * 1024 * 1024) // 8 MB
        .enable_all()
        .build()?;

    runtime.block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize tracing
    let log_level = if args.debug { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;

    // Create application state
    let state = AppState::new();

    // Initialize application state (load existing sessions)
    if let Err(e) = state.init().await {
        tracing::warn!("Failed to initialize state: {}", e);
    }

    // Create router
    let app = create_router(state);

    info!("Starting Multi-Agent System API server on {}", addr);
    info!("API available at http://{}/api/v1/", addr);
    info!("Session data stored in data/sessions/");
    info!("System configs stored in data/systems/");

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
