//! API error types and HTTP response handling

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// API errors that can occur during request handling
#[derive(Debug, Error)]
pub enum ApiError {
    /// The requested system was not found
    #[error("System not found: {0}")]
    SystemNotFound(String),

    /// A system with this name already exists
    #[error("System already exists: {0}")]
    SystemAlreadyExists(String),

    /// The specified agent was not found in the system
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Configuration validation failed
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Error from the underlying agent system
    #[error("Agent system error: {0}")]
    AgentSystemError(#[from] mas_core::AgentError),

    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Request parsing error
    #[error("Invalid request: {0}")]
    BadRequest(String),

    /// Authentication required
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Insufficient permissions
    #[error("Forbidden: {0}")]
    Forbidden(String),
}

/// JSON error response body
#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    code: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ApiError::SystemNotFound(_) => (StatusCode::NOT_FOUND, "SYSTEM_NOT_FOUND"),
            ApiError::SystemAlreadyExists(_) => (StatusCode::CONFLICT, "SYSTEM_ALREADY_EXISTS"),
            ApiError::AgentNotFound(_) => (StatusCode::NOT_FOUND, "AGENT_NOT_FOUND"),
            ApiError::ConfigError(_) => (StatusCode::BAD_REQUEST, "CONFIG_ERROR"),
            ApiError::AgentSystemError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "AGENT_SYSTEM_ERROR"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            ApiError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            ApiError::Forbidden(_) => (StatusCode::FORBIDDEN, "FORBIDDEN"),
        };

        let body = ErrorResponse {
            error: self.to_string(),
            code: code.to_string(),
        };

        (status, Json(body)).into_response()
    }
}

/// Result type alias for API handlers
pub type ApiResult<T> = Result<T, ApiError>;
