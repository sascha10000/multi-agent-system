use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug, Clone)]
pub enum AgentError {
    #[error("Timeout waiting for agent '{agent}' to respond to message {message_id}: waited {waited:?}")]
    Timeout {
        agent: String,
        message_id: Uuid,
        waited: Duration,
    },

    #[error("Agent '{0}' not found")]
    AgentNotFound(String),

    #[error("No connection from '{from}' to '{to}'")]
    NoConnection { from: String, to: String },

    #[error("Agent '{0}' has no registered handler")]
    NoHandler(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("System not running")]
    SystemNotRunning,

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, AgentError>;
