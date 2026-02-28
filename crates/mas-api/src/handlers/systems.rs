//! System management handlers

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use mas_auth::AuthenticatedUser;
use mas_core::{
    agent_system::EchoHandler, load_system_from_json, validate_config, AgentBuilder, SendResult,
};
use serde::Deserialize;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::{
    AgentInfo, ConnectionInfo, DeleteSystemResponse, ListSystemsResponse, PromptResult,
    RegisterSystemRequest, RegisterSystemResponse, SendPromptRequest, SendPromptResponse,
    SystemConfigResponse, SystemDetailResponse, SystemSummary, UpdateSystemRequest,
    UpdateSystemResponse,
};
use crate::state::{extract_metadata, AppState, SystemEntry};

/// Query parameters for listing systems
#[derive(Debug, Deserialize)]
pub struct ListSystemsQuery {
    pub org_id: Option<String>,
}

/// Check if a user has access to a named system (skips check if auth is disabled)
async fn require_system_access(
    state: &AppState,
    user: &AuthenticatedUser,
    system_name: &str,
) -> ApiResult<()> {
    if state.is_auth_disabled() {
        return Ok(());
    }
    let has_access = mas_auth::repository::user_has_system_access(
        state.db(),
        &user.user_id,
        system_name,
    )
    .await
    .map_err(|e| ApiError::Internal(format!("Access check failed: {}", e)))?;

    if !has_access {
        return Err(ApiError::Forbidden(format!(
            "You do not have access to system '{}'",
            system_name
        )));
    }
    Ok(())
}

/// POST /api/v1/systems - Register a new multi-agent system
pub async fn create_system(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<RegisterSystemRequest>,
) -> ApiResult<(StatusCode, Json<RegisterSystemResponse>)> {
    info!("Registering system: {}", request.name);

    // Check if system already exists
    if state.system_exists(&request.name).await {
        return Err(ApiError::SystemAlreadyExists(request.name));
    }

    // Validate the configuration
    validate_config(&request.config).map_err(|e| ApiError::ConfigError(e.to_string()))?;

    // Extract metadata before we move the config
    let metadata = extract_metadata(&request.config);

    // Write config to a temporary file for load_system_from_json
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("mas-api-{}.json", Uuid::new_v4()));

    let config_json = serde_json::to_string_pretty(&request.config)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize config: {}", e)))?;

    std::fs::write(&temp_file, &config_json)
        .map_err(|e| ApiError::Internal(format!("Failed to write temp config: {}", e)))?;

    // Load the system
    let system = load_system_from_json(&temp_file)
        .await
        .map_err(|e| ApiError::ConfigError(e.to_string()))?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);

    // Create the entry and register
    let entry = SystemEntry::new(system, metadata.clone(), request.config.clone());
    let created_at = entry.created_at;

    state
        .register_system(request.name.clone(), entry)
        .await
        .map_err(|e| ApiError::SystemAlreadyExists(e))?;

    // Persist the configuration to disk
    state
        .system_store()
        .save(&request.name, &request.config)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to persist system config: {}", e)))?;

    // Record ownership and org association (skip in dev mode)
    if !state.is_auth_disabled() {
        // Always record the creator as owner (ensures personal-space access)
        if let Err(e) =
            mas_auth::repository::add_system_owner(state.db(), &request.name, &user.user_id).await
        {
            warn!(
                "Failed to record system ownership for '{}': {}",
                request.name, e
            );
        }

        // Also associate with an org if specified or available
        let org_id = if let Some(ref org_id) = request.org_id {
            Some(org_id.clone())
        } else {
            // Auto-associate with user's first org (if any)
            match mas_auth::repository::list_user_orgs(state.db(), &user.user_id).await {
                Ok(orgs) if !orgs.is_empty() => Some(orgs[0].org.id.clone()),
                _ => None,
            }
        };

        if let Some(org_id) = org_id {
            if let Err(e) =
                mas_auth::repository::add_system_org(state.db(), &request.name, &org_id).await
            {
                warn!(
                    "Failed to associate system '{}' with org '{}': {}",
                    request.name, org_id, e
                );
            }
        }
    }

    info!(
        "System '{}' registered with {} agents",
        request.name, metadata.agent_count
    );

    Ok((
        StatusCode::CREATED,
        Json(RegisterSystemResponse {
            name: request.name,
            message: "System registered successfully".to_string(),
            agent_count: metadata.agent_count,
            created_at,
        }),
    ))
}

/// GET /api/v1/systems - List all registered systems
pub async fn list_systems(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<ListSystemsQuery>,
) -> ApiResult<Json<ListSystemsResponse>> {
    let systems = state.list_systems().await;

    // Filter systems by user access (skip in dev mode)
    let allowed_names: Option<HashSet<String>> = if state.is_auth_disabled() {
        None
    } else if let Some(ref org_id) = query.org_id {
        // Verify user is a member of the requested org
        let membership =
            mas_auth::repository::get_membership(state.db(), &user.user_id, org_id)
                .await
                .map_err(|e| ApiError::Internal(format!("Membership check failed: {}", e)))?;
        if membership.is_none() {
            return Err(ApiError::Forbidden(
                "You are not a member of this organization".to_string(),
            ));
        }
        let names = mas_auth::repository::list_org_systems(state.db(), org_id)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to list org systems: {}", e)))?;
        Some(names.into_iter().collect())
    } else {
        let names =
            mas_auth::repository::list_user_systems(state.db(), &user.user_id)
                .await
                .map_err(|e| {
                    ApiError::Internal(format!("Failed to list user systems: {}", e))
                })?;
        Some(names.into_iter().collect())
    };

    let summaries: Vec<SystemSummary> = systems
        .into_iter()
        .filter(|(name, _, _)| {
            allowed_names
                .as_ref()
                .map_or(true, |set| set.contains(name))
        })
        .map(|(name, metadata, created_at)| SystemSummary {
            name,
            agent_count: metadata.agent_count,
            agents: metadata.agent_names,
            created_at,
        })
        .collect();

    let total = summaries.len();

    Ok(Json(ListSystemsResponse {
        systems: summaries,
        total,
    }))
}

/// GET /api/v1/systems/{name} - Get details of a specific system
pub async fn get_system(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(name): Path<String>,
) -> ApiResult<Json<SystemDetailResponse>> {
    require_system_access(&state, &user, &name).await?;

    let (metadata, created_at) = state
        .get_system_metadata(&name)
        .await
        .ok_or_else(|| ApiError::SystemNotFound(name.clone()))?;

    let agents: Vec<AgentInfo> = metadata
        .agents
        .iter()
        .map(|agent| AgentInfo {
            name: agent.name.clone(),
            routing: agent.routing,
            routing_behavior: agent.routing_behavior.clone(),
            connections: agent
                .connections
                .iter()
                .map(|conn| ConnectionInfo {
                    target: conn.target.clone(),
                    connection_type: conn.connection_type.clone(),
                    timeout_secs: conn.timeout_secs,
                })
                .collect(),
        })
        .collect();

    Ok(Json(SystemDetailResponse {
        name,
        agent_count: metadata.agent_count,
        agents,
        global_timeout_secs: metadata.global_timeout_secs,
        created_at,
    }))
}

/// GET /api/v1/systems/{name}/config - Get the full system configuration
pub async fn get_system_config(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(name): Path<String>,
) -> ApiResult<Json<SystemConfigResponse>> {
    require_system_access(&state, &user, &name).await?;

    let (config, created_at) = state
        .get_system_config(&name)
        .await
        .ok_or_else(|| ApiError::SystemNotFound(name.clone()))?;

    Ok(Json(SystemConfigResponse {
        name,
        config,
        created_at,
    }))
}

/// DELETE /api/v1/systems/{name} - Remove a system
pub async fn delete_system(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(name): Path<String>,
) -> ApiResult<Json<DeleteSystemResponse>> {
    require_system_access(&state, &user, &name).await?;

    if !state.remove_system(&name).await {
        return Err(ApiError::SystemNotFound(name));
    }

    // Remove the persisted configuration (ignore errors if file doesn't exist)
    if let Err(e) = state.system_store().delete(&name).await {
        tracing::warn!("Failed to delete persisted config for '{}': {}", name, e);
    }

    // Clean up ownership records (skip in dev mode)
    if !state.is_auth_disabled() {
        if let Err(e) = mas_auth::repository::delete_system_owners(state.db(), &name).await {
            warn!("Failed to delete system ownership records for '{}': {}", name, e);
        }
    }

    info!("System '{}' removed", name);

    Ok(Json(DeleteSystemResponse {
        name: name.clone(),
        message: format!("System '{}' deleted successfully", name),
    }))
}

/// PUT /api/v1/systems/{name} - Update an existing system
pub async fn update_system(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(name): Path<String>,
    Json(request): Json<UpdateSystemRequest>,
) -> ApiResult<Json<UpdateSystemResponse>> {
    info!("Updating system: {}", name);

    // Check existence first — a non-existent system should return 404, not 403
    if !state.system_exists(&name).await {
        return Err(ApiError::SystemNotFound(name));
    }

    require_system_access(&state, &user, &name).await?;

    validate_config(&request.config).map_err(|e| ApiError::ConfigError(e.to_string()))?;

    let metadata = extract_metadata(&request.config);

    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("mas-api-{}.json", Uuid::new_v4()));

    let config_json = serde_json::to_string_pretty(&request.config)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize config: {}", e)))?;

    std::fs::write(&temp_file, &config_json)
        .map_err(|e| ApiError::Internal(format!("Failed to write temp config: {}", e)))?;

    let system = load_system_from_json(&temp_file)
        .await
        .map_err(|e| ApiError::ConfigError(e.to_string()))?;

    let _ = std::fs::remove_file(&temp_file);

    state.remove_system(&name).await;

    let entry = SystemEntry::new(system, metadata.clone(), request.config.clone());
    let updated_at = entry.created_at;

    state
        .register_system(name.clone(), entry)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to re-register system: {}", e)))?;

    state
        .system_store()
        .save(&name, &request.config)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to persist system config: {}", e)))?;

    info!(
        "System '{}' updated with {} agents",
        name, metadata.agent_count
    );

    Ok(Json(UpdateSystemResponse {
        name,
        message: "System updated successfully".to_string(),
        agent_count: metadata.agent_count,
        updated_at,
    }))
}

/// POST /api/v1/systems/{name}/prompt - Send a prompt to a system
pub async fn send_prompt(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(name): Path<String>,
    Json(request): Json<SendPromptRequest>,
) -> ApiResult<Json<SendPromptResponse>> {
    require_system_access(&state, &user, &name).await?;

    let start = Instant::now();

    let system = state
        .get_system(&name)
        .await
        .ok_or_else(|| ApiError::SystemNotFound(name.clone()))?;

    let (metadata, _) = state
        .get_system_metadata(&name)
        .await
        .ok_or_else(|| ApiError::SystemNotFound(name.clone()))?;

    let target_agent = match request.target_agent {
        Some(ref agent_name) => {
            if !metadata.agent_names.contains(agent_name) {
                return Err(ApiError::AgentNotFound(format!(
                    "Agent '{}' not found in system '{}'. Available agents: {:?}",
                    agent_name, name, metadata.agent_names
                )));
            }
            agent_name.clone()
        }
        None => metadata
            .agents
            .iter()
            .find(|a| a.name == "Coordinator")
            .or_else(|| metadata.agents.iter().find(|a| a.routing))
            .or_else(|| metadata.agents.first())
            .map(|a| a.name.clone())
            .ok_or_else(|| ApiError::Internal("No agents available in system".to_string()))?,
    };

    info!(
        "Sending prompt to system '{}' agent '{}': {}",
        name,
        target_agent,
        &request.content[..request.content.len().min(50)]
    );

    let user_name = format!("_ApiUser_{}", Uuid::new_v4());
    let user = AgentBuilder::new(&user_name)
        .blocking_connection(&target_agent)
        .build();

    system
        .register_agent(user, Arc::new(EchoHandler))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create user agent: {}", e)))?;

    let message_id = Uuid::new_v4();
    let result = system
        .send_message(&user_name, &target_agent, &request.content)
        .await
        .map_err(|e| {
            error!("Error sending message: {}", e);
            ApiError::AgentSystemError(e)
        })?;

    let elapsed_ms = start.elapsed().as_millis() as u64;

    let prompt_result = match result {
        SendResult::Response(msg) => PromptResult::Response {
            content: msg.content,
            from: msg.from,
        },
        SendResult::Timeout(err) => PromptResult::Timeout {
            message: err.to_string(),
        },
        SendResult::Notified => PromptResult::Notified,
    };

    Ok(Json(SendPromptResponse {
        message_id,
        target_agent,
        result: prompt_result,
        elapsed_ms,
    }))
}
