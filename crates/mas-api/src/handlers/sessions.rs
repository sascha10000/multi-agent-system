//! Session management handlers

use std::collections::HashSet;
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
use mas_auth::AuthenticatedUser;
use mas_core::{
    agent_system::EchoHandler, AgentBuilder, SendResult, StoredMessage, TraceCollector,
    TraceEventType,
};
use serde::Deserialize;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::models::{
    AgentTraceStep, CreateSessionRequest, CreateSessionResponse, DeleteSessionResponse,
    ListSessionsResponse, MessageResponse, PromptResult, SearchHit, SessionDetailResponse,
    SessionHistoryResponse, SessionPromptRequest, SessionPromptResponse, SessionSearchRequest,
    SessionSearchResponse, SessionSummary,
};
use crate::session::SessionError;
use crate::state::AppState;

/// Check if a user owns a session (skips check if auth is disabled)
async fn require_session_ownership(
    state: &AppState,
    user: &AuthenticatedUser,
    session_id: &str,
) -> ApiResult<()> {
    if state.is_auth_disabled() {
        return Ok(());
    }
    let owns = mas_auth::repository::user_owns_session(state.db(), &user.user_id, session_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Session ownership check failed: {}", e)))?;

    if !owns {
        return Err(ApiError::Forbidden(format!(
            "You do not have access to session '{}'",
            session_id
        )));
    }
    Ok(())
}

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
        SessionError::AlreadyExists(id) => {
            ApiError::BadRequest(format!("Session already exists: {}", id))
        }
        SessionError::SystemNotFound(name) => ApiError::SystemNotFound(name),
        SessionError::Memory(e) => ApiError::Internal(format!("Memory error: {}", e)),
        SessionError::Internal(msg) => ApiError::Internal(msg),
    }
}

/// Query parameters for listing sessions
#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    pub system_name: Option<String>,
}

/// POST /api/v1/sessions - Create a new session
pub async fn create_session(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<CreateSessionRequest>,
) -> ApiResult<(StatusCode, Json<CreateSessionResponse>)> {
    info!("Creating session for system: {}", request.system_name);

    if !state.system_exists(&request.system_name).await {
        return Err(ApiError::SystemNotFound(request.system_name));
    }

    // Verify user has access to the system (skip in dev mode)
    if !state.is_auth_disabled() {
        let has_access = mas_auth::repository::user_has_system_access(
            state.db(),
            &user.user_id,
            &request.system_name,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Access check failed: {}", e)))?;

        if !has_access {
            return Err(ApiError::Forbidden(format!(
                "You do not have access to system '{}'",
                request.system_name
            )));
        }
    }

    let mut manager = state.session_manager().write().await;
    let session_info = manager
        .create_session(&request.system_name)
        .await
        .map_err(session_to_api_error)?;

    // Record session ownership (skip in dev mode)
    if !state.is_auth_disabled() {
        if let Err(e) = mas_auth::repository::create_session_record(
            state.db(),
            &session_info.id,
            &user.user_id,
            &request.system_name,
        )
        .await
        {
            warn!(
                "Failed to record session ownership for '{}': {}",
                session_info.id, e
            );
        }
    }

    info!(
        "Created session {} for system {}",
        session_info.id, request.system_name
    );

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
    user: AuthenticatedUser,
    Query(query): Query<ListSessionsQuery>,
) -> ApiResult<Json<ListSessionsResponse>> {
    let manager = state.session_manager().read().await;
    let sessions = manager.list_sessions(query.system_name.as_deref());

    // Filter sessions by ownership (skip in dev mode)
    let owned_ids: Option<HashSet<String>> = if state.is_auth_disabled() {
        None
    } else {
        let ids = mas_auth::repository::list_user_sessions(state.db(), &user.user_id)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to list user sessions: {}", e)))?;
        Some(ids.into_iter().collect())
    };

    let summaries: Vec<SessionSummary> = sessions
        .into_iter()
        .filter(|s| owned_ids.as_ref().map_or(true, |set| set.contains(&s.id)))
        .map(|s| SessionSummary {
            id: s.id,
            system_name: s.system_name,
            created_at: s.created_at,
            message_count: s.message_count,
            last_activity: s.last_activity,
        })
        .collect();

    let total = summaries.len();

    Ok(Json(ListSessionsResponse {
        sessions: summaries,
        total,
    }))
}

/// GET /api/v1/sessions/{id} - Get session details
pub async fn get_session_detail(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(session_id): Path<String>,
) -> ApiResult<Json<SessionDetailResponse>> {
    require_session_ownership(&state, &user, &session_id).await?;

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
    user: AuthenticatedUser,
    Path(session_id): Path<String>,
) -> ApiResult<Json<DeleteSessionResponse>> {
    require_session_ownership(&state, &user, &session_id).await?;

    let mut manager = state.session_manager().write().await;
    manager
        .delete_session(&session_id)
        .await
        .map_err(session_to_api_error)?;

    // Clean up ownership record (skip in dev mode)
    if !state.is_auth_disabled() {
        if let Err(e) = mas_auth::repository::delete_session_record(state.db(), &session_id).await
        {
            warn!(
                "Failed to delete session ownership record for '{}': {}",
                session_id, e
            );
        }
    }

    info!("Deleted session: {}", session_id);

    Ok(Json(DeleteSessionResponse {
        id: session_id.clone(),
        message: format!("Session '{}' deleted successfully", session_id),
    }))
}

/// Query parameters for history endpoint
#[derive(Debug, Deserialize)]
pub struct HistoryParams {
    pub limit: Option<usize>,
}

/// GET /api/v1/sessions/{id}/history - Get conversation history
pub async fn get_session_history(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(session_id): Path<String>,
    Query(params): Query<HistoryParams>,
) -> ApiResult<Json<SessionHistoryResponse>> {
    require_session_ownership(&state, &user, &session_id).await?;
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

/// GET /api/v1/sessions/{id}/search - Search session history
pub async fn search_session(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(session_id): Path<String>,
    Query(params): Query<SessionSearchRequest>,
) -> ApiResult<Json<SessionSearchResponse>> {
    require_session_ownership(&state, &user, &session_id).await?;
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
    user: AuthenticatedUser,
    Path(session_id): Path<String>,
    Json(request): Json<SessionPromptRequest>,
) -> ApiResult<Json<SessionPromptResponse>> {
    require_session_ownership(&state, &user, &session_id).await?;

    let start = Instant::now();

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
            .find(|a| a.entry_point)
            .or_else(|| metadata.agents.iter().find(|a| a.name == "Coordinator"))
            .or_else(|| metadata.agents.iter().find(|a| a.routing))
            .or_else(|| metadata.agents.first())
            .map(|a| a.name.clone())
            .ok_or_else(|| ApiError::Internal("No agents available in system".to_string()))?,
    };

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
        "Sending prompt to session '{}' system '{}' agent '{}': {}",
        session_id,
        system_name,
        target_agent,
        &request.content[..request.content.len().min(50)]
    );

    let trace_collector = TraceCollector::new();

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
        .send_message_with_trace(
            &user_name,
            &target_agent,
            &full_message,
            trace_collector.clone(),
        )
        .await
        .map_err(|e| {
            error!("Error sending message: {}", e);
            ApiError::AgentSystemError(e)
        })?;

    let elapsed_ms = start.elapsed().as_millis() as u64;

    let mut trace = Vec::new();
    trace.push(AgentTraceStep {
        from: "User".to_string(),
        to: target_agent.clone(),
        content: request.content.clone(),
        step_type: "request".to_string(),
    });

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

    let prompt_result = match &result {
        SendResult::Response(msg) => {
            let mut manager = state.session_manager().write().await;
            let meta = serde_json::json!({ "elapsed_ms": elapsed_ms });
            manager
                .store_agent_response(&session_id, &msg.from, &msg.content, Some(meta))
                .await
                .map_err(session_to_api_error)?;

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
pub async fn send_session_prompt_stream(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(session_id): Path<String>,
    Json(request): Json<SessionPromptRequest>,
) -> ApiResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    require_session_ownership(&state, &user, &session_id).await?;

    let start = Instant::now();

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
            .find(|a| a.entry_point)
            .or_else(|| metadata.agents.iter().find(|a| a.name == "Coordinator"))
            .or_else(|| metadata.agents.iter().find(|a| a.routing))
            .or_else(|| metadata.agents.first())
            .map(|a| a.name.clone())
            .ok_or_else(|| ApiError::Internal("No agents available in system".to_string()))?,
    };

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

    let trace_collector = TraceCollector::new();
    let mut trace_rx = trace_collector.subscribe();

    let user_name = format!("_ApiUser_{}", Uuid::new_v4());
    let user = AgentBuilder::new(&user_name)
        .blocking_connection(&target_agent)
        .build();

    system
        .register_agent(user, Arc::new(EchoHandler))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create user agent: {}", e)))?;

    let message_id = Uuid::new_v4();
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);

    let task_content = request.content.clone();
    let task_target = target_agent.clone();
    let task_session_id = session_id.clone();

    tokio::spawn(async move {
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
                                    return;
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("SSE trace receiver lagged by {} events", n);
                            continue;
                        }
                        Err(_) => break,
                    }
                }
                result = &mut work => {
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
    user: AuthenticatedUser,
    Path(session_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    require_session_ownership(&state, &user, &session_id).await?;
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
