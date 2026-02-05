//! Route handlers for the REST API

use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use mas_core::{
    agent_system::EchoHandler, config_loader::SystemConfigJson, load_system_from_json,
    validate_config, AgentBuilder, SendResult,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::{
    AgentInfo, ConnectionInfo, DeleteSystemResponse, ListSystemsResponse, PromptResult,
    RegisterSystemRequest, RegisterSystemResponse, SendPromptRequest, SendPromptResponse,
    SystemDetailResponse, SystemSummary, UpdateSystemRequest, UpdateSystemResponse,
};
use crate::state::{AgentMetadata, AppState, ConfigMetadata, ConnectionMetadata, SystemEntry};

/// Extract metadata from a SystemConfigJson
fn extract_metadata(config: &SystemConfigJson) -> ConfigMetadata {
    let agents: Vec<AgentMetadata> = config
        .agents
        .iter()
        .map(|agent| {
            let connections: Vec<ConnectionMetadata> = agent
                .connections
                .iter()
                .map(|(target, conn)| ConnectionMetadata {
                    target: target.clone(),
                    connection_type: conn.connection_type.clone(),
                    timeout_secs: conn.timeout_secs,
                })
                .collect();

            AgentMetadata {
                name: agent.name.clone(),
                role: agent.role.clone(),
                routing: agent.handler.routing,
                connections,
            }
        })
        .collect();

    ConfigMetadata {
        agent_count: config.agents.len(),
        agent_names: config.agents.iter().map(|a| a.name.clone()).collect(),
        global_timeout_secs: config.system.global_timeout_secs,
        agents,
    }
}

/// POST /api/v1/systems - Register a new multi-agent system
pub async fn create_system(
    State(state): State<AppState>,
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
    // This is necessary because mas-core only exposes file-based loading
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
    let entry = SystemEntry::new(system, metadata.clone());
    let created_at = entry.created_at;

    state
        .register_system(request.name.clone(), entry)
        .await
        .map_err(|e| ApiError::SystemAlreadyExists(e))?;

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
pub async fn list_systems(State(state): State<AppState>) -> Json<ListSystemsResponse> {
    let systems = state.list_systems().await;

    let summaries: Vec<SystemSummary> = systems
        .into_iter()
        .map(|(name, metadata, created_at)| SystemSummary {
            name,
            agent_count: metadata.agent_count,
            agents: metadata.agent_names,
            created_at,
        })
        .collect();

    let total = summaries.len();

    Json(ListSystemsResponse {
        systems: summaries,
        total,
    })
}

/// GET /api/v1/systems/{name} - Get details of a specific system
pub async fn get_system(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<SystemDetailResponse>> {
    let (metadata, created_at) = state
        .get_system_metadata(&name)
        .await
        .ok_or_else(|| ApiError::SystemNotFound(name.clone()))?;

    let agents: Vec<AgentInfo> = metadata
        .agents
        .iter()
        .map(|agent| AgentInfo {
            name: agent.name.clone(),
            role: agent.role.clone(),
            routing: agent.routing,
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

/// DELETE /api/v1/systems/{name} - Remove a system
pub async fn delete_system(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<DeleteSystemResponse>> {
    if !state.remove_system(&name).await {
        return Err(ApiError::SystemNotFound(name));
    }

    info!("System '{}' removed", name);

    Ok(Json(DeleteSystemResponse {
        name: name.clone(),
        message: format!("System '{}' deleted successfully", name),
    }))
}

/// PUT /api/v1/systems/{name} - Update an existing system
///
/// Uses a replace strategy: removes the old system and creates a new one with the updated config.
pub async fn update_system(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(request): Json<UpdateSystemRequest>,
) -> ApiResult<Json<UpdateSystemResponse>> {
    info!("Updating system: {}", name);

    // Check if system exists
    if !state.system_exists(&name).await {
        return Err(ApiError::SystemNotFound(name));
    }

    // Validate the new configuration
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

    // Load the new system
    let system = load_system_from_json(&temp_file)
        .await
        .map_err(|e| ApiError::ConfigError(e.to_string()))?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);

    // Remove the old system
    state.remove_system(&name).await;

    // Create the new entry and register
    let entry = SystemEntry::new(system, metadata.clone());
    let updated_at = entry.created_at;

    state
        .register_system(name.clone(), entry)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to re-register system: {}", e)))?;

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
    Path(name): Path<String>,
    Json(request): Json<SendPromptRequest>,
) -> ApiResult<Json<SendPromptResponse>> {
    let start = Instant::now();

    // Get system and metadata
    let system = state
        .get_system(&name)
        .await
        .ok_or_else(|| ApiError::SystemNotFound(name.clone()))?;

    let (metadata, _) = state
        .get_system_metadata(&name)
        .await
        .ok_or_else(|| ApiError::SystemNotFound(name.clone()))?;

    // Determine target agent
    let target_agent = match request.target_agent {
        Some(ref agent_name) => {
            // Verify agent exists
            if !metadata.agent_names.contains(agent_name) {
                return Err(ApiError::AgentNotFound(format!(
                    "Agent '{}' not found in system '{}'. Available agents: {:?}",
                    agent_name, name, metadata.agent_names
                )));
            }
            agent_name.clone()
        }
        None => {
            // Auto-select: prefer "Coordinator", then first routing agent, then first agent
            metadata
                .agents
                .iter()
                .find(|a| a.name == "Coordinator")
                .or_else(|| metadata.agents.iter().find(|a| a.routing))
                .or_else(|| metadata.agents.first())
                .map(|a| a.name.clone())
                .ok_or_else(|| ApiError::Internal("No agents available in system".to_string()))?
        }
    };

    info!(
        "Sending prompt to system '{}' agent '{}': {}",
        name,
        target_agent,
        &request.content[..request.content.len().min(50)]
    );

    // Create a temporary "User" agent to send the message
    let user_name = format!("_ApiUser_{}", Uuid::new_v4());
    let user = AgentBuilder::new(&user_name)
        .role("API User")
        .blocking_connection(&target_agent)
        .build();

    system
        .register_agent(user, Arc::new(EchoHandler))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create user agent: {}", e)))?;

    // Send the message
    let message_id = Uuid::new_v4();
    let result = system
        .send_message(&user_name, &target_agent, &request.content)
        .await
        .map_err(|e| {
            error!("Error sending message: {}", e);
            ApiError::AgentSystemError(e)
        })?;

    let elapsed_ms = start.elapsed().as_millis() as u64;

    // Convert result to API response
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
