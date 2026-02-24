//! Route handlers for the REST API

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures::stream::Stream;
use mas_core::{
    agent_system::EchoHandler, load_system_from_json, validate_config, AgentBuilder, SendResult,
    StoredMessage, TraceCollector, TraceEventType,
};
use serde::Deserialize;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::{
    AgentInfo, AgentTraceStep, ConnectionInfo, CreateSessionRequest, CreateSessionResponse,
    DeleteSessionResponse, DeleteSystemResponse, ListSessionsResponse, ListSystemsResponse,
    MessageResponse, PromptResult, RegisterSystemRequest, RegisterSystemResponse, SearchHit,
    SendPromptRequest, SendPromptResponse, SessionDetailResponse, SessionHistoryResponse,
    SessionPromptRequest, SessionPromptResponse, SessionSearchRequest, SessionSearchResponse,
    SessionSummary, SystemConfigResponse, SystemDetailResponse, SystemSummary,
    UpdateSystemRequest, UpdateSystemResponse,
};
use crate::session::SessionError;
use crate::state::{extract_metadata, AppState, SystemEntry};

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
    Path(name): Path<String>,
) -> ApiResult<Json<SystemConfigResponse>> {
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
    Path(name): Path<String>,
) -> ApiResult<Json<DeleteSystemResponse>> {
    if !state.remove_system(&name).await {
        return Err(ApiError::SystemNotFound(name));
    }

    // Remove the persisted configuration (ignore errors if file doesn't exist)
    if let Err(e) = state.system_store().delete(&name).await {
        // Only log, don't fail the request - the in-memory system was removed
        tracing::warn!("Failed to delete persisted config for '{}': {}", name, e);
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
    let entry = SystemEntry::new(system, metadata.clone(), request.config.clone());
    let updated_at = entry.created_at;

    state
        .register_system(name.clone(), entry)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to re-register system: {}", e)))?;

    // Persist the updated configuration (overwrites existing)
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

// ============================================================================
// Session Handlers
// ============================================================================

/// Convert a StoredMessage to a MessageResponse
fn stored_to_response(msg: &StoredMessage) -> MessageResponse {
    MessageResponse {
        id: msg.id.clone(),
        from: msg.from.clone(),
        to: msg.to.clone(),
        content: msg.content.clone(),
        timestamp: msg.timestamp,
        metadata: msg.metadata.clone(),
    }
}

/// Convert a SessionError to an ApiError
fn session_to_api_error(e: SessionError) -> ApiError {
    match e {
        SessionError::NotFound(id) => ApiError::BadRequest(format!("Session not found: {}", id)),
        SessionError::AlreadyExists(id) => ApiError::BadRequest(format!("Session already exists: {}", id)),
        SessionError::SystemNotFound(name) => ApiError::SystemNotFound(name),
        SessionError::Memory(e) => ApiError::Internal(format!("Memory error: {}", e)),
        SessionError::Internal(msg) => ApiError::Internal(msg),
    }
}

/// Query parameters for listing sessions
#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    /// Filter by system name
    pub system_name: Option<String>,
}

/// POST /api/v1/sessions - Create a new session
pub async fn create_session(
    State(state): State<AppState>,
    Json(request): Json<CreateSessionRequest>,
) -> ApiResult<(StatusCode, Json<CreateSessionResponse>)> {
    info!("Creating session for system: {}", request.system_name);

    // Verify the system exists
    if !state.system_exists(&request.system_name).await {
        return Err(ApiError::SystemNotFound(request.system_name));
    }

    let mut manager = state.session_manager().write().await;
    let session_info = manager
        .create_session(&request.system_name)
        .await
        .map_err(session_to_api_error)?;

    info!("Created session {} for system {}", session_info.id, request.system_name);

    Ok((
        StatusCode::CREATED,
        Json(CreateSessionResponse {
            id: session_info.id,
            system_name: session_info.system_name,
            created_at: session_info.created_at,
            message: "Session created successfully".to_string(),
        }),
    ))
}

/// GET /api/v1/sessions - List all sessions
pub async fn list_sessions(
    State(state): State<AppState>,
    Query(query): Query<ListSessionsQuery>,
) -> Json<ListSessionsResponse> {
    let manager = state.session_manager().read().await;
    let sessions = manager.list_sessions(query.system_name.as_deref());

    let summaries: Vec<SessionSummary> = sessions
        .into_iter()
        .map(|s| SessionSummary {
            id: s.id,
            system_name: s.system_name,
            created_at: s.created_at,
            message_count: s.message_count,
            last_activity: s.last_activity,
        })
        .collect();

    let total = summaries.len();

    Json(ListSessionsResponse {
        sessions: summaries,
        total,
    })
}

/// GET /api/v1/sessions/{id} - Get session details
pub async fn get_session_detail(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> ApiResult<Json<SessionDetailResponse>> {
    let manager = state.session_manager().read().await;
    let session = manager
        .get_session(&session_id)
        .ok_or_else(|| ApiError::BadRequest(format!("Session not found: {}", session_id)))?;

    let info = session.info();

    Ok(Json(SessionDetailResponse {
        id: info.id,
        system_name: info.system_name,
        created_at: info.created_at,
        message_count: info.message_count,
        last_activity: info.last_activity,
    }))
}

/// DELETE /api/v1/sessions/{id} - Delete a session
pub async fn delete_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> ApiResult<Json<DeleteSessionResponse>> {
    let mut manager = state.session_manager().write().await;
    manager
        .delete_session(&session_id)
        .await
        .map_err(session_to_api_error)?;

    info!("Deleted session: {}", session_id);

    Ok(Json(DeleteSessionResponse {
        id: session_id.clone(),
        message: format!("Session '{}' deleted successfully", session_id),
    }))
}

/// GET /api/v1/sessions/{id}/history - Get conversation history
pub async fn get_session_history(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(params): Query<HistoryParams>,
) -> ApiResult<Json<SessionHistoryResponse>> {
    let manager = state.session_manager().read().await;
    let messages = manager
        .get_history(&session_id, params.limit)
        .map_err(session_to_api_error)?;

    let total = messages.len();
    let message_responses: Vec<MessageResponse> = messages.iter().map(stored_to_response).collect();

    Ok(Json(SessionHistoryResponse {
        session_id,
        messages: message_responses,
        total,
    }))
}

/// Query parameters for history endpoint
#[derive(Debug, Deserialize)]
pub struct HistoryParams {
    /// Maximum number of messages to return (most recent first)
    pub limit: Option<usize>,
}

/// GET /api/v1/sessions/{id}/search - Search session history
pub async fn search_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(params): Query<SessionSearchRequest>,
) -> ApiResult<Json<SessionSearchResponse>> {
    let manager = state.session_manager().read().await;
    let hits = manager
        .search_session(&session_id, &params.query, params.top_k)
        .await
        .map_err(session_to_api_error)?;

    let search_hits: Vec<SearchHit> = hits
        .into_iter()
        .map(|h| SearchHit {
            message: stored_to_response(&h.message),
            score: h.score,
        })
        .collect();

    Ok(Json(SessionSearchResponse {
        session_id,
        query: params.query,
        hits: search_hits,
    }))
}

/// POST /api/v1/sessions/{id}/prompt - Send a prompt to a session
pub async fn send_session_prompt(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<SessionPromptRequest>,
) -> ApiResult<Json<SessionPromptResponse>> {
    let start = Instant::now();

    // Get the system name for this session
    let system_name = {
        let manager = state.session_manager().read().await;
        manager
            .get_session_system(&session_id)
            .ok_or_else(|| ApiError::BadRequest(format!("Session not found: {}", session_id)))?
            .to_string()
    };

    // Get system and metadata
    let system = state
        .get_system(&system_name)
        .await
        .ok_or_else(|| ApiError::SystemNotFound(system_name.clone()))?;

    let (metadata, _) = state
        .get_system_metadata(&system_name)
        .await
        .ok_or_else(|| ApiError::SystemNotFound(system_name.clone()))?;

    // Determine target agent
    let target_agent = match request.target_agent {
        Some(ref agent_name) => {
            if !metadata.agent_names.contains(agent_name) {
                return Err(ApiError::AgentNotFound(format!(
                    "Agent '{}' not found in system '{}'. Available agents: {:?}",
                    agent_name, system_name, metadata.agent_names
                )));
            }
            agent_name.clone()
        }
        None => {
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

    // Get recent conversation history for context
    let mut context_messages = Vec::new();
    let mut history_context = String::new();

    if request.include_context && request.context_limit > 0 {
        let manager = state.session_manager().read().await;
        // Get recent messages (chronological order) instead of semantic search
        if let Ok(history) = manager.get_history(&session_id, Some(request.context_limit)) {
            // Build context string from conversation history
            if !history.is_empty() {
                history_context.push_str("<conversation_history>\n");
                for msg in &history {
                    let role = if msg.from == "user" { "User" } else { &msg.from };
                    history_context.push_str(&format!("{}: {}\n", role, msg.content));
                }
                history_context.push_str("</conversation_history>\n\n");
            }
            context_messages = history.into_iter().map(|m| stored_to_response(&m)).collect();
        }
    }

    // Store the user message
    {
        let mut manager = state.session_manager().write().await;
        manager
            .store_user_message(&session_id, &target_agent, &request.content)
            .await
            .map_err(session_to_api_error)?;
    }

    // Build the full message with context
    let full_message = if history_context.is_empty() {
        request.content.clone()
    } else {
        format!("{}Current message: {}", history_context, request.content)
    };

    info!(
        "Sending prompt to session '{}' system '{}' agent '{}': {}",
        session_id,
        system_name,
        target_agent,
        &request.content[..request.content.len().min(50)]
    );

    // Create a trace collector to capture agent communications
    let trace_collector = TraceCollector::new();

    // Create a temporary "User" agent to send the message
    let user_name = format!("_ApiUser_{}", Uuid::new_v4());
    let user = AgentBuilder::new(&user_name)
        .blocking_connection(&target_agent)
        .build();

    system
        .register_agent(user, Arc::new(EchoHandler))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create user agent: {}", e)))?;

    // Send the message with conversation history context and tracing enabled
    let message_id = Uuid::new_v4();
    let result = system
        .send_message_with_trace(&user_name, &target_agent, &full_message, trace_collector.clone())
        .await
        .map_err(|e| {
            error!("Error sending message: {}", e);
            ApiError::AgentSystemError(e)
        })?;

    let elapsed_ms = start.elapsed().as_millis() as u64;

    // Build trace of agent communications from the collector
    let mut trace = Vec::new();

    // Record the initial user request
    trace.push(AgentTraceStep {
        from: "User".to_string(),
        to: target_agent.clone(),
        content: request.content.clone(),
        step_type: "request".to_string(),
    });

    // Add the recorded trace events (forwards, responses, synthesis)
    let trace_events = trace_collector.events().await;
    for event in trace_events {
        trace.push(AgentTraceStep {
            from: event.from,
            to: event.to,
            content: event.content,
            step_type: match event.event_type {
                TraceEventType::Request => "request".to_string(),
                TraceEventType::Response => "response".to_string(),
                TraceEventType::Forward => "forward".to_string(),
                TraceEventType::Synthesis => "synthesis".to_string(),
            },
        });
    }

    // Convert result to API response and store the response
    let prompt_result = match &result {
        SendResult::Response(msg) => {
            // Store the agent response
            let mut manager = state.session_manager().write().await;
            let meta = serde_json::json!({ "elapsed_ms": elapsed_ms });
            manager
                .store_agent_response(&session_id, &msg.from, &msg.content, Some(meta))
                .await
                .map_err(session_to_api_error)?;

            // Record the final response to user in trace
            trace.push(AgentTraceStep {
                from: msg.from.clone(),
                to: "User".to_string(),
                content: msg.content.clone(),
                step_type: "response".to_string(),
            });

            PromptResult::Response {
                content: msg.content.clone(),
                from: msg.from.clone(),
            }
        }
        SendResult::Timeout(err) => {
            trace.push(AgentTraceStep {
                from: target_agent.clone(),
                to: "User".to_string(),
                content: format!("Timeout: {}", err),
                step_type: "response".to_string(),
            });
            PromptResult::Timeout {
                message: err.to_string(),
            }
        }
        SendResult::Notified => PromptResult::Notified,
    };

    Ok(Json(SessionPromptResponse {
        message_id,
        session_id,
        target_agent,
        result: prompt_result,
        elapsed_ms,
        context: context_messages,
        trace,
    }))
}

/// POST /api/v1/sessions/{id}/prompt/stream - Send a prompt via SSE streaming
///
/// Streams trace events in real-time as agents process, then sends the final response.
/// This avoids proxy timeouts for long-running LLM calls.
pub async fn send_session_prompt_stream(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(request): Json<SessionPromptRequest>,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let start = Instant::now();

    // --- Validation (same as send_session_prompt) ---

    let system_name = {
        let manager = state.session_manager().read().await;
        manager
            .get_session_system(&session_id)
            .ok_or_else(|| ApiError::BadRequest(format!("Session not found: {}", session_id)))?
            .to_string()
    };

    let system = state
        .get_system(&system_name)
        .await
        .ok_or_else(|| ApiError::SystemNotFound(system_name.clone()))?;

    let (metadata, _) = state
        .get_system_metadata(&system_name)
        .await
        .ok_or_else(|| ApiError::SystemNotFound(system_name.clone()))?;

    let target_agent = match request.target_agent {
        Some(ref agent_name) => {
            if !metadata.agent_names.contains(agent_name) {
                return Err(ApiError::AgentNotFound(format!(
                    "Agent '{}' not found in system '{}'. Available agents: {:?}",
                    agent_name, system_name, metadata.agent_names
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

    // Get conversation history context
    let mut context_messages = Vec::new();
    let mut history_context = String::new();

    if request.include_context && request.context_limit > 0 {
        let manager = state.session_manager().read().await;
        if let Ok(history) = manager.get_history(&session_id, Some(request.context_limit)) {
            if !history.is_empty() {
                history_context.push_str("<conversation_history>\n");
                for msg in &history {
                    let role = if msg.from == "user" { "User" } else { &msg.from };
                    history_context.push_str(&format!("{}: {}\n", role, msg.content));
                }
                history_context.push_str("</conversation_history>\n\n");
            }
            context_messages = history.into_iter().map(|m| stored_to_response(&m)).collect();
        }
    }

    // Store the user message
    {
        let mut manager = state.session_manager().write().await;
        manager
            .store_user_message(&session_id, &target_agent, &request.content)
            .await
            .map_err(session_to_api_error)?;
    }

    let full_message = if history_context.is_empty() {
        request.content.clone()
    } else {
        format!("{}Current message: {}", history_context, request.content)
    };

    info!(
        "SSE streaming prompt to session '{}' system '{}' agent '{}': {}",
        session_id,
        system_name,
        target_agent,
        &request.content[..request.content.len().min(50)]
    );

    // Create trace collector and subscribe BEFORE spawning work
    let trace_collector = TraceCollector::new();
    let mut trace_rx = trace_collector.subscribe();

    // Register temporary user agent
    let user_name = format!("_ApiUser_{}", Uuid::new_v4());
    let user = AgentBuilder::new(&user_name)
        .blocking_connection(&target_agent)
        .build();

    system
        .register_agent(user, Arc::new(EchoHandler))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create user agent: {}", e)))?;

    let message_id = Uuid::new_v4();

    // Channel for SSE payloads
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);

    // Capture values for the background task
    let task_content = request.content.clone();
    let task_target = target_agent.clone();
    let task_session_id = session_id.clone();

    // Spawn the work + trace forwarding in one task
    tokio::spawn(async move {
        // Spawn the actual agent work
        let work_trace = trace_collector.clone();
        let work_system = system.clone();
        let work_user = user_name.clone();
        let work_target = task_target.clone();
        let work_msg = full_message;

        let work = tokio::spawn(async move {
            work_system
                .send_message_with_trace(&work_user, &work_target, &work_msg, work_trace)
                .await
        });

        tokio::pin!(work);

        // Forward trace events until work completes
        loop {
            tokio::select! {
                biased;
                event = trace_rx.recv() => {
                    match event {
                        Ok(trace_event) => {
                            let step = AgentTraceStep {
                                from: trace_event.from.clone(),
                                to: trace_event.to.clone(),
                                content: trace_event.content.clone(),
                                step_type: match trace_event.event_type {
                                    TraceEventType::Request => "request".to_string(),
                                    TraceEventType::Response => "response".to_string(),
                                    TraceEventType::Forward => "forward".to_string(),
                                    TraceEventType::Synthesis => "synthesis".to_string(),
                                },
                            };
                            if let Ok(json) = serde_json::to_string(&step) {
                                let event = Event::default().event("trace").data(json);
                                if tx.send(Ok(event)).await.is_err() {
                                    return; // client disconnected
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("SSE trace receiver lagged by {} events", n);
                            continue;
                        }
                        Err(_) => break, // channel closed
                    }
                }
                result = &mut work => {
                    // Drain any remaining trace events
                    while let Ok(trace_event) = trace_rx.try_recv() {
                        let step = AgentTraceStep {
                            from: trace_event.from.clone(),
                            to: trace_event.to.clone(),
                            content: trace_event.content.clone(),
                            step_type: match trace_event.event_type {
                                TraceEventType::Request => "request".to_string(),
                                TraceEventType::Response => "response".to_string(),
                                TraceEventType::Forward => "forward".to_string(),
                                TraceEventType::Synthesis => "synthesis".to_string(),
                            },
                        };
                        if let Ok(json) = serde_json::to_string(&step) {
                            let event = Event::default().event("trace").data(json);
                            let _ = tx.send(Ok(event)).await;
                        }
                    }

                    let elapsed_ms = start.elapsed().as_millis() as u64;

                    match result {
                        Ok(Ok(send_result)) => {
                            // Build trace from collector
                            let mut trace = vec![AgentTraceStep {
                                from: "User".to_string(),
                                to: task_target.clone(),
                                content: task_content.clone(),
                                step_type: "request".to_string(),
                            }];

                            let trace_events = trace_collector.events().await;
                            for te in &trace_events {
                                trace.push(AgentTraceStep {
                                    from: te.from.clone(),
                                    to: te.to.clone(),
                                    content: te.content.clone(),
                                    step_type: match te.event_type {
                                        TraceEventType::Request => "request".to_string(),
                                        TraceEventType::Response => "response".to_string(),
                                        TraceEventType::Forward => "forward".to_string(),
                                        TraceEventType::Synthesis => "synthesis".to_string(),
                                    },
                                });
                            }

                            let prompt_result = match &send_result {
                                SendResult::Response(msg) => {
                                    // Store the agent response
                                    let mut manager = state.session_manager().write().await;
                                    let meta = serde_json::json!({ "elapsed_ms": elapsed_ms });
                                    let _ = manager
                                        .store_agent_response(
                                            &task_session_id,
                                            &msg.from,
                                            &msg.content,
                                            Some(meta),
                                        )
                                        .await;

                                    trace.push(AgentTraceStep {
                                        from: msg.from.clone(),
                                        to: "User".to_string(),
                                        content: msg.content.clone(),
                                        step_type: "response".to_string(),
                                    });

                                    PromptResult::Response {
                                        content: msg.content.clone(),
                                        from: msg.from.clone(),
                                    }
                                }
                                SendResult::Timeout(err) => {
                                    trace.push(AgentTraceStep {
                                        from: task_target.clone(),
                                        to: "User".to_string(),
                                        content: format!("Timeout: {}", err),
                                        step_type: "response".to_string(),
                                    });
                                    PromptResult::Timeout {
                                        message: err.to_string(),
                                    }
                                }
                                SendResult::Notified => PromptResult::Notified,
                            };

                            let response = SessionPromptResponse {
                                message_id,
                                session_id: task_session_id,
                                target_agent: task_target,
                                result: prompt_result,
                                elapsed_ms,
                                context: context_messages,
                                trace,
                            };

                            if let Ok(json) = serde_json::to_string(&response) {
                                let event = Event::default().event("complete").data(json);
                                let _ = tx.send(Ok(event)).await;
                            }
                        }
                        Ok(Err(agent_err)) => {
                            let err_json = serde_json::json!({ "error": agent_err.to_string() });
                            let event = Event::default()
                                .event("error")
                                .data(err_json.to_string());
                            let _ = tx.send(Ok(event)).await;
                        }
                        Err(join_err) => {
                            let err_json = serde_json::json!({ "error": format!("Task failed: {}", join_err) });
                            let event = Event::default()
                                .event("error")
                                .data(err_json.to_string());
                            let _ = tx.send(Ok(event)).await;
                        }
                    }
                    break;
                }
            }
        }
    });

    let stream = ReceiverStream::new(rx);

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new().interval(std::time::Duration::from_secs(15)),
    ))
}

/// POST /api/v1/sessions/{id}/build-index - Build the search index for a session
pub async fn build_session_index(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut manager = state.session_manager().write().await;
    manager
        .build_index(&session_id)
        .await
        .map_err(session_to_api_error)?;

    info!("Built index for session: {}", session_id);

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "message": "Index built successfully"
    })))
}
