//! Organization management handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use mas_auth::{repository, AuthenticatedUser, MemberInfo, OrgRole, OrgWithRole};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// ─── Request/Response Types ─────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateOrgRequest {
    pub name: String,
    pub slug: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrgResponse {
    #[serde(flatten)]
    pub org: mas_auth::Organization,
    pub role: Option<OrgRole>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrgRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: String,
}

// ─── Organization CRUD ──────────────────────────────────

/// POST /api/v1/orgs
pub async fn create_org(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreateOrgRequest>,
) -> ApiResult<(StatusCode, Json<OrgResponse>)> {
    let pool = state.db();

    if request.name.is_empty() {
        return Err(ApiError::BadRequest("Organization name is required".to_string()));
    }
    if request.slug.is_empty() {
        return Err(ApiError::BadRequest("Organization slug is required".to_string()));
    }

    // If parent_id is specified, verify the user has access to the parent
    if let Some(ref parent_id) = request.parent_id {
        let role = repository::get_membership(pool, &user.user_id, parent_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        match role {
            Some(OrgRole::Owner | OrgRole::Admin) => {}
            _ => {
                return Err(ApiError::Forbidden(
                    "You must be an owner or admin of the parent organization".to_string(),
                ))
            }
        }
    }

    let org_id = uuid::Uuid::new_v4().to_string();
    let org = repository::create_org(
        pool,
        &org_id,
        &request.name,
        &request.slug,
        request.parent_id.as_deref(),
    )
    .await
    .map_err(|e| match e {
        mas_auth::AuthError::OrgSlugTaken(slug) => {
            ApiError::BadRequest(format!("Organization slug already taken: {}", slug))
        }
        _ => ApiError::Internal(e.to_string()),
    })?;

    // Auto-assign creator as owner
    repository::add_membership(pool, &user.user_id, &org_id, OrgRole::Owner)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    info!("Organization '{}' created by {}", org.name, user.user_id);

    Ok((
        StatusCode::CREATED,
        Json(OrgResponse {
            org,
            role: Some(OrgRole::Owner),
        }),
    ))
}

/// GET /api/v1/orgs
pub async fn list_orgs(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<Vec<OrgWithRole>>> {
    let pool = state.db();

    let orgs = repository::list_user_orgs(pool, &user.user_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(orgs))
}

/// GET /api/v1/orgs/{id}
pub async fn get_org(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(org_id): Path<String>,
) -> ApiResult<Json<OrgResponse>> {
    let pool = state.db();

    // Check membership
    let role = repository::get_membership(pool, &user.user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("You are not a member of this organization".to_string()))?;

    let org = repository::find_org_by_id(pool, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::BadRequest("Organization not found".to_string()))?;

    Ok(Json(OrgResponse {
        org,
        role: Some(role),
    }))
}

/// PUT /api/v1/orgs/{id}
pub async fn update_org(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(org_id): Path<String>,
    Json(request): Json<UpdateOrgRequest>,
) -> ApiResult<Json<OrgResponse>> {
    let pool = state.db();

    let role = repository::get_membership(pool, &user.user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("You are not a member of this organization".to_string()))?;

    if !role.can_modify_org() {
        return Err(ApiError::Forbidden(
            "Only owners and admins can modify the organization".to_string(),
        ));
    }

    repository::update_org(pool, &org_id, request.name.as_deref(), request.slug.as_deref())
        .await
        .map_err(|e| match e {
            mas_auth::AuthError::OrgSlugTaken(slug) => {
                ApiError::BadRequest(format!("Organization slug already taken: {}", slug))
            }
            _ => ApiError::Internal(e.to_string()),
        })?;

    let org = repository::find_org_by_id(pool, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Internal("Organization disappeared".to_string()))?;

    Ok(Json(OrgResponse {
        org,
        role: Some(role),
    }))
}

/// DELETE /api/v1/orgs/{id}
pub async fn delete_org(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(org_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db();

    let role = repository::get_membership(pool, &user.user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("You are not a member of this organization".to_string()))?;

    if !role.can_delete_org() {
        return Err(ApiError::Forbidden(
            "Only owners can delete the organization".to_string(),
        ));
    }

    repository::delete_org(pool, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    info!("Organization {} deleted by {}", org_id, user.user_id);

    Ok(Json(serde_json::json!({
        "message": "Organization deleted successfully"
    })))
}

/// GET /api/v1/orgs/{id}/children
pub async fn list_child_orgs(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(org_id): Path<String>,
) -> ApiResult<Json<Vec<mas_auth::Organization>>> {
    let pool = state.db();

    // Verify membership
    repository::get_membership(pool, &user.user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("You are not a member of this organization".to_string()))?;

    let children = repository::list_child_orgs(pool, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(children))
}

// ─── Membership Management ──────────────────────────────

/// GET /api/v1/orgs/{id}/members
pub async fn list_members(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(org_id): Path<String>,
) -> ApiResult<Json<Vec<MemberInfo>>> {
    let pool = state.db();

    // Verify membership
    repository::get_membership(pool, &user.user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("You are not a member of this organization".to_string()))?;

    let members = repository::list_org_members(pool, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(members))
}

/// POST /api/v1/orgs/{id}/members
pub async fn add_member(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(org_id): Path<String>,
    Json(request): Json<AddMemberRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let pool = state.db();

    // Check requester's role
    let role = repository::get_membership(pool, &user.user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("You are not a member of this organization".to_string()))?;

    if !role.can_manage_members() {
        return Err(ApiError::Forbidden(
            "Only owners and admins can add members".to_string(),
        ));
    }

    // Parse the requested role
    let new_role = OrgRole::from_str(&request.role)
        .ok_or_else(|| ApiError::BadRequest(format!("Invalid role: {}", request.role)))?;

    // Find the user to add by email
    let target_user = repository::find_user_by_email(pool, &request.email)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::BadRequest(format!("User not found: {}", request.email)))?;

    // Add membership
    repository::add_membership(pool, &target_user.id, &org_id, new_role)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    info!(
        "User {} added to org {} as {:?} by {}",
        target_user.email, org_id, new_role, user.user_id
    );

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Member added successfully",
            "user_id": target_user.id,
            "role": request.role
        })),
    ))
}

/// PUT /api/v1/orgs/{id}/members/{user_id}
pub async fn update_member_role(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((org_id, target_user_id)): Path<(String, String)>,
    Json(request): Json<UpdateMemberRoleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db();

    let role = repository::get_membership(pool, &user.user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("You are not a member of this organization".to_string()))?;

    if !role.can_manage_members() {
        return Err(ApiError::Forbidden(
            "Only owners and admins can change member roles".to_string(),
        ));
    }

    let new_role = OrgRole::from_str(&request.role)
        .ok_or_else(|| ApiError::BadRequest(format!("Invalid role: {}", request.role)))?;

    repository::update_membership_role(pool, &target_user_id, &org_id, new_role)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "Member role updated",
        "user_id": target_user_id,
        "role": request.role
    })))
}

/// DELETE /api/v1/orgs/{id}/members/{user_id}
pub async fn remove_member(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((org_id, target_user_id)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db();

    let role = repository::get_membership(pool, &user.user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("You are not a member of this organization".to_string()))?;

    if !role.can_manage_members() {
        return Err(ApiError::Forbidden(
            "Only owners and admins can remove members".to_string(),
        ));
    }

    repository::remove_membership(pool, &target_user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    info!(
        "User {} removed from org {} by {}",
        target_user_id, org_id, user.user_id
    );

    Ok(Json(serde_json::json!({
        "message": "Member removed successfully"
    })))
}

// ─── System-Org Association ─────────────────────────────

/// GET /api/v1/orgs/{id}/systems
pub async fn list_org_systems(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(org_id): Path<String>,
) -> ApiResult<Json<Vec<String>>> {
    let pool = state.db();

    repository::get_membership(pool, &user.user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("You are not a member of this organization".to_string()))?;

    let systems = repository::list_org_systems(pool, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(systems))
}

/// PUT /api/v1/orgs/{id}/systems/{name}
pub async fn add_system_to_org(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((org_id, system_name)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db();

    let role = repository::get_membership(pool, &user.user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("You are not a member of this organization".to_string()))?;

    if !role.can_modify_org() {
        return Err(ApiError::Forbidden(
            "Only owners and admins can add systems to the organization".to_string(),
        ));
    }

    // Verify the system exists
    if !state.system_exists(&system_name).await {
        return Err(ApiError::SystemNotFound(system_name));
    }

    repository::add_system_org(pool, &system_name, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "System added to organization"
    })))
}

/// DELETE /api/v1/orgs/{id}/systems/{name}
pub async fn remove_system_from_org(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((org_id, system_name)): Path<(String, String)>,
) -> ApiResult<Json<serde_json::Value>> {
    let pool = state.db();

    let role = repository::get_membership(pool, &user.user_id, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("You are not a member of this organization".to_string()))?;

    if !role.can_modify_org() {
        return Err(ApiError::Forbidden(
            "Only owners and admins can remove systems from the organization".to_string(),
        ));
    }

    repository::remove_system_org(pool, &system_name, &org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({
        "message": "System removed from organization"
    })))
}
