//! Axum middleware for JWT authentication
//!
//! Provides the `AuthenticatedUser` extractor that can be added to handler
//! parameters to require authentication.

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::jwt::JwtConfig;

/// Authenticated user extracted from JWT token in the Authorization header.
///
/// Add this as a handler parameter to require authentication:
/// ```ignore
/// async fn my_handler(user: AuthenticatedUser, ...) -> ... { }
/// ```
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub email: String,
}

/// State needed for auth extraction
#[derive(Clone)]
pub struct AuthState {
    pub jwt_config: Arc<JwtConfig>,
    pub auth_disabled: bool,
}

/// Error response for auth failures
#[derive(Debug, Serialize)]
struct AuthErrorResponse {
    error: String,
    code: String,
}

enum AuthRejection {
    MissingToken,
    InvalidToken,
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthRejection::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authorization token"),
            AuthRejection::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired token"),
        };
        let body = AuthErrorResponse {
            error: message.to_string(),
            code: "UNAUTHORIZED".to_string(),
        };
        (status, Json(body)).into_response()
    }
}

/// Trait to extract AuthState from a larger state type.
/// Implement this for your AppState.
pub trait FromRef<T> {
    fn from_ref(input: &T) -> Self;
}

// Identity impl — if the state IS AuthState
impl FromRef<AuthState> for AuthState {
    fn from_ref(input: &AuthState) -> Self {
        input.clone()
    }
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync + 'static,
    AuthState: FromRef<S>,
{
    type Rejection = Response;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 S,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        let auth_state = AuthState::from_ref(state);
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Box::pin(async move {
            // If auth is disabled (dev mode), return a dummy user
            if auth_state.auth_disabled {
                return Ok(AuthenticatedUser {
                    user_id: "dev-user".to_string(),
                    email: "dev@localhost".to_string(),
                });
            }

            // Extract the Authorization header
            let auth_header = auth_header
                .ok_or_else(|| AuthRejection::MissingToken.into_response())?;

            // Parse "Bearer <token>"
            let token = auth_header
                .strip_prefix("Bearer ")
                .ok_or_else(|| AuthRejection::MissingToken.into_response())?;

            // Verify the JWT
            let claims = auth_state
                .jwt_config
                .verify_access_token(token)
                .map_err(|_| AuthRejection::InvalidToken.into_response())?;

            Ok(AuthenticatedUser {
                user_id: claims.sub,
                email: claims.email,
            })
        })
    }
}

/// Optional authenticated user — doesn't reject if no token is present.
/// Useful for endpoints that work differently for authed vs anon users.
#[derive(Debug, Clone)]
pub struct OptionalUser(pub Option<AuthenticatedUser>);

impl<S> FromRequestParts<S> for OptionalUser
where
    S: Send + Sync + 'static,
    AuthState: FromRef<S>,
{
    type Rejection = Response;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 S,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        let auth_state = AuthState::from_ref(state);
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Box::pin(async move {
            if auth_state.auth_disabled {
                return Ok(OptionalUser(Some(AuthenticatedUser {
                    user_id: "dev-user".to_string(),
                    email: "dev@localhost".to_string(),
                })));
            }

            // Try to extract, but don't fail if missing
            let user = auth_header
                .as_deref()
                .and_then(|h| h.strip_prefix("Bearer "))
                .and_then(|token| auth_state.jwt_config.verify_access_token(token).ok())
                .map(|claims| AuthenticatedUser {
                    user_id: claims.sub,
                    email: claims.email,
                });

            Ok(OptionalUser(user))
        })
    }
}
