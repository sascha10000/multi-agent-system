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
    pub role: String,
    pub routing: bool,
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
