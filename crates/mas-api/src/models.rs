//! Request and response models for the REST API

use chrono::{DateTime, Utc};
use mas_core::config_loader::SystemConfigJson;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request body for registering a new system
#[derive(Debug, Deserialize)]
pub struct RegisterSystemRequest {
    /// Unique name for this system instance
    pub name: String,
    /// The JSON configuration for the multi-agent system
    pub config: SystemConfigJson,
    /// Optional organization ID to associate this system with
    #[serde(default)]
    pub org_id: Option<String>,
}

/// Response after successfully registering a system
#[derive(Debug, Serialize)]
pub struct RegisterSystemResponse {
    pub name: String,
    pub message: String,
    pub agent_count: usize,
    pub created_at: DateTime<Utc>,
}

/// Summary information about a registered system
#[derive(Debug, Serialize)]
pub struct SystemSummary {
    pub name: String,
    pub agent_count: usize,
    pub agents: Vec<String>,
    pub created_at: DateTime<Utc>,
}

/// Response listing all registered systems
#[derive(Debug, Serialize)]
pub struct ListSystemsResponse {
    pub systems: Vec<SystemSummary>,
    pub total: usize,
}

/// Detailed information about a specific system
#[derive(Debug, Serialize)]
pub struct SystemDetailResponse {
    pub name: String,
    pub agent_count: usize,
    pub agents: Vec<AgentInfo>,
    pub global_timeout_secs: u64,
    pub created_at: DateTime<Utc>,
}

/// Information about an agent within a system
#[derive(Debug, Serialize)]
pub struct AgentInfo {
    pub name: String,
    pub routing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing_behavior: Option<String>,
    pub connections: Vec<ConnectionInfo>,
}

/// Information about a connection between agents
#[derive(Debug, Serialize)]
pub struct ConnectionInfo {
    pub target: String,
    pub connection_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

/// Full system config response (for editor reload)
#[derive(Debug, Serialize)]
pub struct SystemConfigResponse {
    pub name: String,
    pub config: SystemConfigJson,
    pub created_at: DateTime<Utc>,
}

/// Request body for sending a prompt to a system
#[derive(Debug, Deserialize)]
pub struct SendPromptRequest {
    /// The message content to send
    pub content: String,
    /// Optional target agent (auto-selects Coordinator or first routing agent if not specified)
    #[serde(default)]
    pub target_agent: Option<String>,
}

/// Response after sending a prompt
#[derive(Debug, Serialize)]
pub struct SendPromptResponse {
    /// Unique ID for this request
    pub message_id: Uuid,
    /// The agent that processed the request
    pub target_agent: String,
    /// The result of processing
    pub result: PromptResult,
    /// Time taken to process in milliseconds
    pub elapsed_ms: u64,
}

/// Result of processing a prompt
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptResult {
    /// Successfully received a response
    Response {
        content: String,
        from: String,
    },
    /// The request timed out
    Timeout {
        message: String,
    },
    /// Message was sent but no response expected (notify connection)
    Notified,
}

/// Response after deleting a system
#[derive(Debug, Serialize)]
pub struct DeleteSystemResponse {
    pub name: String,
    pub message: String,
}

/// Request body for updating an existing system
#[derive(Debug, Deserialize)]
pub struct UpdateSystemRequest {
    /// The updated JSON configuration for the multi-agent system
    pub config: SystemConfigJson,
}

/// Response after successfully updating a system
#[derive(Debug, Serialize)]
pub struct UpdateSystemResponse {
    pub name: String,
    pub message: String,
    pub agent_count: usize,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Session API Models
// ============================================================================

/// Request body for creating a new session
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    /// The name of the system this session is for
    pub system_name: String,
}

/// Response after creating a session
#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    /// The unique session ID
    pub id: String,
    /// The system this session is for
    pub system_name: String,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    pub message: String,
}

/// Summary info for a session
#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub system_name: String,
    pub created_at: DateTime<Utc>,
    pub message_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_activity: Option<DateTime<Utc>>,
}

/// Response listing all sessions
#[derive(Debug, Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionSummary>,
    pub total: usize,
}

/// Detailed information about a session
#[derive(Debug, Serialize)]
pub struct SessionDetailResponse {
    pub id: String,
    pub system_name: String,
    pub created_at: DateTime<Utc>,
    pub message_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_activity: Option<DateTime<Utc>>,
}

/// Response after deleting a session
#[derive(Debug, Serialize)]
pub struct DeleteSessionResponse {
    pub id: String,
    pub message: String,
}

/// A stored message in the session
#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Response for session history
#[derive(Debug, Serialize)]
pub struct SessionHistoryResponse {
    pub session_id: String,
    pub messages: Vec<MessageResponse>,
    pub total: usize,
}

/// Request body for sending a prompt to a session
#[derive(Debug, Deserialize)]
pub struct SessionPromptRequest {
    /// The message content to send
    pub content: String,
    /// Optional target agent (auto-selects if not specified)
    #[serde(default)]
    pub target_agent: Option<String>,
    /// Whether to include relevant context from past messages
    #[serde(default)]
    pub include_context: bool,
    /// How many past messages to include as context
    #[serde(default = "default_context_limit")]
    pub context_limit: usize,
}

fn default_context_limit() -> usize {
    5
}

/// A single step in the agent communication trace
#[derive(Debug, Clone, Serialize)]
pub struct AgentTraceStep {
    /// The agent sending the message
    pub from: String,
    /// The agent receiving the message
    pub to: String,
    /// The message content
    pub content: String,
    /// Type of message: "request", "response", "forward", "synthesis"
    pub step_type: String,
}

/// Response after sending a prompt to a session
#[derive(Debug, Serialize)]
pub struct SessionPromptResponse {
    /// Unique ID for this request
    pub message_id: Uuid,
    /// The session this was sent to
    pub session_id: String,
    /// The agent that processed the request
    pub target_agent: String,
    /// The result of processing
    pub result: PromptResult,
    /// Time taken to process in milliseconds
    pub elapsed_ms: u64,
    /// Context messages that were included (if any)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<MessageResponse>,
    /// Trace of agent communications (for verbose mode)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<AgentTraceStep>,
}

/// Request for searching session history
#[derive(Debug, Deserialize)]
pub struct SessionSearchRequest {
    /// The search query
    pub query: String,
    /// Maximum number of results to return
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

fn default_top_k() -> usize {
    5
}

/// A search result hit
#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub message: MessageResponse,
    pub score: f32,
}

/// Response for session search
#[derive(Debug, Serialize)]
pub struct SessionSearchResponse {
    pub session_id: String,
    pub query: String,
    pub hits: Vec<SearchHit>,
}
