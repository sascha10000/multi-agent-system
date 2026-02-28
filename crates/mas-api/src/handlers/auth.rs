//! Authentication handlers (register, login, refresh, profile)

use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use mas_auth::{
    jwt::hash_refresh_token,
    password, repository, AuthenticatedUser, UserInfo,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// ─── Request/Response Types ─────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user: UserInfo,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserInfo,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub password: Option<String>,
}

// ─── Handlers ───────────────────────────────────────────

/// POST /api/v1/auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<(StatusCode, Json<RegisterResponse>)> {
    let pool = state.db();
    let jwt = state.jwt_config();

    // Validate input
    if request.email.is_empty() || !request.email.contains('@') {
        return Err(ApiError::BadRequest("Invalid email address".to_string()));
    }
    if request.password.len() < 8 {
        return Err(ApiError::BadRequest(
            "Password must be at least 8 characters".to_string(),
        ));
    }
    if request.display_name.is_empty() {
        return Err(ApiError::BadRequest(
            "Display name is required".to_string(),
        ));
    }

    // Hash password
    let password_hash = password::hash_password(&request.password)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Create user
    let user_id = uuid::Uuid::new_v4().to_string();
    let user = repository::create_user(pool, &user_id, &request.email, &request.display_name, &password_hash)
        .await
        .map_err(|e| match e {
            mas_auth::AuthError::EmailTaken(email) => {
                ApiError::BadRequest(format!("Email already registered: {}", email))
            }
            _ => ApiError::Internal(e.to_string()),
        })?;

    info!("New user registered: {} ({})", user.email, user.id);

    // Auto-create a "Personal" organization for the new user
    let org_id = uuid::Uuid::new_v4().to_string();
    let org_slug = format!("personal-{}", &user_id[..8]);
    let _ = repository::create_org(pool, &org_id, "Personal", &org_slug, None).await;
    let _ = repository::add_membership(pool, &user_id, &org_id, mas_auth::OrgRole::Owner).await;

    // Create tokens
    let access_token = jwt
        .create_access_token(&user.id, &user.email)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let refresh_token = jwt.create_refresh_token();
    let refresh_hash = hash_refresh_token(&refresh_token);
    let refresh_expires = Utc::now() + chrono::Duration::seconds(jwt.refresh_token_ttl_secs as i64);

    repository::store_refresh_token(
        pool,
        &uuid::Uuid::new_v4().to_string(),
        &user.id,
        &refresh_hash,
        &refresh_expires.format("%Y-%m-%d %H:%M:%S").to_string(),
    )
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user: user.into(),
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: jwt.access_token_ttl_secs,
        }),
    ))
}

/// POST /api/v1/auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    let pool = state.db();
    let jwt = state.jwt_config();

    // Find user by email
    let user = repository::find_user_by_email(pool, &request.email)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Unauthorized("Invalid email or password".to_string()))?;

    // Verify password
    let valid = password::verify_password(&request.password, &user.password_hash)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    if !valid {
        return Err(ApiError::Unauthorized(
            "Invalid email or password".to_string(),
        ));
    }

    info!("User logged in: {} ({})", user.email, user.id);

    // Create tokens
    let access_token = jwt
        .create_access_token(&user.id, &user.email)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let refresh_token = jwt.create_refresh_token();
    let refresh_hash = hash_refresh_token(&refresh_token);
    let refresh_expires = Utc::now() + chrono::Duration::seconds(jwt.refresh_token_ttl_secs as i64);

    repository::store_refresh_token(
        pool,
        &uuid::Uuid::new_v4().to_string(),
        &user.id,
        &refresh_hash,
        &refresh_expires.format("%Y-%m-%d %H:%M:%S").to_string(),
    )
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(LoginResponse {
        user: user.into(),
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: jwt.access_token_ttl_secs,
    }))
}

/// POST /api/v1/auth/refresh
pub async fn refresh(
    State(state): State<AppState>,
    Json(request): Json<RefreshRequest>,
) -> ApiResult<Json<RefreshResponse>> {
    let pool = state.db();
    let jwt = state.jwt_config();

    // Hash the provided refresh token and look it up
    let token_hash = hash_refresh_token(&request.refresh_token);
    let (old_token_id, user_id) = repository::find_valid_refresh_token(pool, &token_hash)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Unauthorized("Invalid or expired refresh token".to_string()))?;

    // Delete the old refresh token (rotation)
    repository::delete_refresh_token(pool, &old_token_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Find the user
    let user = repository::find_user_by_id(pool, &user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    // Create new token pair
    let access_token = jwt
        .create_access_token(&user.id, &user.email)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let refresh_token = jwt.create_refresh_token();
    let refresh_hash = hash_refresh_token(&refresh_token);
    let refresh_expires = Utc::now() + chrono::Duration::seconds(jwt.refresh_token_ttl_secs as i64);

    repository::store_refresh_token(
        pool,
        &uuid::Uuid::new_v4().to_string(),
        &user.id,
        &refresh_hash,
        &refresh_expires.format("%Y-%m-%d %H:%M:%S").to_string(),
    )
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(RefreshResponse {
        access_token,
        refresh_token,
        token_type: "Bearer".to_string(),
        expires_in: jwt.access_token_ttl_secs,
    }))
}

/// GET /api/v1/auth/me
pub async fn get_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<UserInfo>> {
    let pool = state.db();

    let db_user = repository::find_user_by_id(pool, &user.user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    Ok(Json(db_user.into()))
}

/// PUT /api/v1/auth/me
pub async fn update_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<UpdateProfileRequest>,
) -> ApiResult<Json<UserInfo>> {
    let pool = state.db();

    let new_hash = match &request.password {
        Some(pw) => {
            if pw.len() < 8 {
                return Err(ApiError::BadRequest(
                    "Password must be at least 8 characters".to_string(),
                ));
            }
            Some(
                password::hash_password(pw)
                    .map_err(|e| ApiError::Internal(e.to_string()))?,
            )
        }
        None => None,
    };

    repository::update_user(
        pool,
        &user.user_id,
        request.display_name.as_deref(),
        new_hash.as_deref(),
    )
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let updated_user = repository::find_user_by_id(pool, &user.user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Internal("User disappeared".to_string()))?;

    Ok(Json(updated_user.into()))
}
