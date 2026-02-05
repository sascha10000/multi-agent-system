//! Multi-Agent System REST API Server
//!
//! Provides HTTP endpoints for registering, managing, and interacting
//! with multi-agent systems.

use std::net::SocketAddr;

use mas_api::{create_router, AppState};
use tracing::{info, Level};

/// Parse command line arguments for server configuration
fn parse_args() -> (String, u16) {
    let args: Vec<String> = std::env::args().collect();

    let host = get_arg_value(&args, "--host").unwrap_or_else(|| "0.0.0.0".to_string());
    let port = get_arg_value(&args, "--port")
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    (host, port)
}

fn get_arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn print_usage() {
    println!("Multi-Agent System REST API Server");
    println!();
    println!("Usage: mas-api [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --host <HOST>  Host to bind to (default: 0.0.0.0)");
    println!("  --port <PORT>  Port to listen on (default: 3000)");
    println!("  --help         Show this help message");
    println!();
    println!("API Endpoints - Systems:");
    println!("  POST   /api/v1/systems                Register a new system");
    println!("  GET    /api/v1/systems                List all systems");
    println!("  GET    /api/v1/systems/{{name}}         Get system details");
    println!("  PUT    /api/v1/systems/{{name}}         Update a system");
    println!("  DELETE /api/v1/systems/{{name}}         Remove a system");
    println!("  POST   /api/v1/systems/{{name}}/prompt  Send a prompt (no session)");
    println!();
    println!("API Endpoints - Sessions:");
    println!("  POST   /api/v1/sessions                      Create a new session");
    println!("  GET    /api/v1/sessions                      List all sessions");
    println!("  GET    /api/v1/sessions/{{id}}                 Get session details");
    println!("  DELETE /api/v1/sessions/{{id}}                 Delete a session");
    println!("  GET    /api/v1/sessions/{{id}}/history         Get conversation history");
    println!("  GET    /api/v1/sessions/{{id}}/search?q=...    Semantic search in session");
    println!("  POST   /api/v1/sessions/{{id}}/prompt          Send a prompt (with memory)");
    println!("  POST   /api/v1/sessions/{{id}}/build-index     Build search index");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Check for --help flag
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return Ok(());
    }

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let (host, port) = parse_args();
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

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

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
