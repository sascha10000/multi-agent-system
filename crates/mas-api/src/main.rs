//! Multi-Agent System REST API Server
//!
//! Provides HTTP endpoints for registering, managing, and interacting
//! with multi-agent systems.

use std::net::SocketAddr;

use clap::Parser;
use mas_api::{create_router, AppState};
use mas_auth::{create_pool, run_migrations, JwtConfig};
use tracing::{info, Level};

/// Multi-Agent System REST API Server
///
/// Provides HTTP endpoints for registering, managing, and interacting
/// with multi-agent systems.
#[derive(Parser, Debug)]
#[command(name = "mas-api")]
#[command(author, version, about, long_about = None)]
#[command(after_help = r#"API ENDPOINTS:

  Auth (public):
    POST   /api/v1/auth/register       Register a new account
    POST   /api/v1/auth/login          Log in
    POST   /api/v1/auth/refresh        Refresh access token

  Auth (authenticated):
    GET    /api/v1/auth/me             Get current user profile
    PUT    /api/v1/auth/me             Update profile

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

  Organizations:
    POST   /api/v1/orgs                   Create organization
    GET    /api/v1/orgs                   List user's organizations
    GET    /api/v1/orgs/{id}              Get organization details
    PUT    /api/v1/orgs/{id}              Update organization
    DELETE /api/v1/orgs/{id}              Delete organization
    GET    /api/v1/orgs/{id}/children     List child organizations
    GET    /api/v1/orgs/{id}/members      List members
    POST   /api/v1/orgs/{id}/members      Add member
    PUT    /api/v1/orgs/{id}/members/{uid} Change member role
    DELETE /api/v1/orgs/{id}/members/{uid} Remove member
    GET    /api/v1/orgs/{id}/systems      List org's systems
    PUT    /api/v1/orgs/{id}/systems/{n}  Add system to org
    DELETE /api/v1/orgs/{id}/systems/{n}  Remove system from org

EXAMPLES:
    mas-api                     Start server on 0.0.0.0:8080
    mas-api --port 3000         Start server on port 3000
    mas-api --host 127.0.0.1    Bind to localhost only

ENVIRONMENT VARIABLES:
    MAS_JWT_SECRET              JWT signing secret (required in production)
    MAS_DISABLE_AUTH=true       Disable authentication (dev mode)
    MAS_DB_PATH                 SQLite database path (default: data/mas.db)
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
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_stack_size(8 * 1024 * 1024)
        .enable_all()
        .build()?;

    runtime.block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    let args = Args::parse();

    let log_level = if args.debug { Level::DEBUG } else { Level::INFO };
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;

    // Determine auth configuration
    let auth_disabled = std::env::var("MAS_DISABLE_AUTH")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let jwt_secret = std::env::var("MAS_JWT_SECRET").unwrap_or_else(|_| {
        if !auth_disabled {
            info!("MAS_JWT_SECRET not set, using a random secret (tokens won't survive restarts)");
        }
        uuid::Uuid::new_v4().to_string()
    });

    let db_path = std::env::var("MAS_DB_PATH").unwrap_or_else(|_| "data/mas.db".to_string());

    // Initialize database
    let pool = create_pool(&db_path).await?;
    run_migrations(&pool).await?;

    // Create JWT config
    let jwt_config = JwtConfig::new(jwt_secret);

    // Create application state
    let state = AppState::new()
        .with_db(pool)
        .with_jwt_config(jwt_config)
        .with_auth_disabled(auth_disabled);

    // Initialize application state (load existing sessions and systems)
    if let Err(e) = state.init().await {
        tracing::warn!("Failed to initialize state: {}", e);
    }

    let app = create_router(state);

    info!("Starting Multi-Agent System API server on {}", addr);
    info!("API available at http://{}/api/v1/", addr);
    if auth_disabled {
        info!("Authentication is DISABLED (MAS_DISABLE_AUTH=true)");
    }
    info!("Database: {}", db_path);
    info!("Session data stored in data/sessions/");
    info!("System configs stored in data/systems/");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
