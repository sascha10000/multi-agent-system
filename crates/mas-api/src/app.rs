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
    let candidates = [
        "static",
        "crates/mas-api/static",
        "../mas-api/static",
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.join("index.html").exists() {
            return path;
        }
    }

    PathBuf::from("crates/mas-api/static")
}

/// Build the application router with all routes and middleware
pub fn create_router(state: AppState) -> Router {
    // Public auth routes (no token required)
    let auth_routes = Router::new()
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/refresh", post(handlers::auth::refresh));

    // Authenticated auth routes
    let auth_protected = Router::new()
        .route("/auth/me", get(handlers::auth::get_me))
        .route("/auth/me", put(handlers::auth::update_me));

    // System management routes
    let system_routes = Router::new()
        .route("/systems", post(handlers::systems::create_system))
        .route("/systems", get(handlers::systems::list_systems))
        .route("/systems/:name", get(handlers::systems::get_system))
        .route("/systems/:name/config", get(handlers::systems::get_system_config))
        .route("/systems/:name", put(handlers::systems::update_system))
        .route("/systems/:name", delete(handlers::systems::delete_system))
        .route("/systems/:name/prompt", post(handlers::systems::send_prompt));

    // Session management routes
    let session_routes = Router::new()
        .route("/sessions", post(handlers::sessions::create_session))
        .route("/sessions", get(handlers::sessions::list_sessions))
        .route("/sessions/:id", get(handlers::sessions::get_session_detail))
        .route("/sessions/:id", delete(handlers::sessions::delete_session))
        .route("/sessions/:id/history", get(handlers::sessions::get_session_history))
        .route("/sessions/:id/search", get(handlers::sessions::search_session))
        .route("/sessions/:id/prompt", post(handlers::sessions::send_session_prompt))
        .route("/sessions/:id/prompt/stream", post(handlers::sessions::send_session_prompt_stream))
        .route("/sessions/:id/build-index", post(handlers::sessions::build_session_index));

    // Organization routes (all authenticated)
    let org_routes = Router::new()
        .route("/orgs", post(handlers::orgs::create_org))
        .route("/orgs", get(handlers::orgs::list_orgs))
        .route("/orgs/:id", get(handlers::orgs::get_org))
        .route("/orgs/:id", put(handlers::orgs::update_org))
        .route("/orgs/:id", delete(handlers::orgs::delete_org))
        .route("/orgs/:id/children", get(handlers::orgs::list_child_orgs))
        .route("/orgs/:id/members", get(handlers::orgs::list_members))
        .route("/orgs/:id/members", post(handlers::orgs::add_member))
        .route("/orgs/:id/members/:user_id", put(handlers::orgs::update_member_role))
        .route("/orgs/:id/members/:user_id", delete(handlers::orgs::remove_member))
        .route("/orgs/:id/systems", get(handlers::orgs::list_org_systems))
        .route("/orgs/:id/systems/:name", put(handlers::orgs::add_system_to_org))
        .route("/orgs/:id/systems/:name", delete(handlers::orgs::remove_system_from_org));

    // Combine all API routes under /api/v1
    let api_routes = Router::new()
        .merge(auth_routes)
        .merge(auth_protected)
        .merge(system_routes)
        .merge(session_routes)
        .merge(org_routes);

    // Serve static files
    let static_dir = find_static_dir();
    let index_path = static_dir.join("index.html");

    let static_service = ServeDir::new(&static_dir)
        .not_found_service(ServeFile::new(&index_path));

    Router::new()
        .nest("/api/v1", api_routes)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state)
        .fallback_service(static_service)
}
