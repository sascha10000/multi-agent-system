//! Axum router configuration

use std::path::PathBuf;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::handlers;
use crate::state::AppState;

/// Find the static files directory
fn find_static_dir() -> PathBuf {
    // Check common locations for static files
    let candidates = [
        "static",                      // Running from crates/mas-api/
        "crates/mas-api/static",       // Running from workspace root
        "../mas-api/static",           // Running from another crate dir
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.join("index.html").exists() {
            return path;
        }
    }

    // Default fallback
    PathBuf::from("crates/mas-api/static")
}

/// Build the application router with all routes and middleware
pub fn create_router(state: AppState) -> Router {
    // Define API routes
    let api_routes = Router::new()
        // System management
        .route("/systems", post(handlers::create_system))
        .route("/systems", get(handlers::list_systems))
        .route("/systems/:name", get(handlers::get_system))
        .route("/systems/:name/config", get(handlers::get_system_config))
        .route("/systems/:name", put(handlers::update_system))
        .route("/systems/:name", delete(handlers::delete_system))
        // Prompt handling (direct system prompt, no session)
        .route("/systems/:name/prompt", post(handlers::send_prompt))
        // Session management
        .route("/sessions", post(handlers::create_session))
        .route("/sessions", get(handlers::list_sessions))
        .route("/sessions/:id", get(handlers::get_session_detail))
        .route("/sessions/:id", delete(handlers::delete_session))
        .route("/sessions/:id/history", get(handlers::get_session_history))
        .route("/sessions/:id/search", get(handlers::search_session))
        .route("/sessions/:id/prompt", post(handlers::send_session_prompt))
        .route("/sessions/:id/prompt/stream", post(handlers::send_session_prompt_stream))
        .route("/sessions/:id/build-index", post(handlers::build_session_index));

    // Serve static files, checking multiple possible locations
    let static_dir = find_static_dir();
    let index_path = static_dir.join("index.html");

    let static_service = ServeDir::new(&static_dir)
        .not_found_service(ServeFile::new(&index_path));

    // Build the main router with API version prefix
    Router::new()
        .nest("/api/v1", api_routes)
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        // Attach shared state
        .with_state(state)
        // Fallback to static file serving for the UI
        .fallback_service(static_service)
}
