//! REST API library for the Multi-Agent System
//!
//! This crate provides a REST API for managing and interacting with
//! multi-agent systems defined by JSON configurations.

pub mod app;
pub mod error;
pub mod handlers;
pub mod models;
pub mod session;
pub mod state;

pub use app::create_router;
pub use error::{ApiError, ApiResult};
pub use session::{create_session_manager, SessionError, SessionInfo, SessionManager, SharedSessionManager};
pub use state::AppState;
