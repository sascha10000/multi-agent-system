pub mod agent;
pub mod agent_system;
pub mod config;
pub mod config_loader;
pub mod connection;
pub mod conversation;
pub mod decision;
pub mod errors;
pub mod llm;
pub mod message;
pub mod session_memory;

// Re-export commonly used types
pub use agent::{Agent, AgentBuilder};
pub use agent_system::{AgentSystem, MessageHandler, RoutingHandler, SendResult};
pub use config::SystemConfig;
pub use config_loader::{load_system_from_json, parse_config_file, validate_config, SystemConfigJson};
pub use connection::{Connection, ConnectionType};
pub use decision::{ForwardTarget, HandlerDecision};
pub use errors::{AgentError, Result};
pub use llm::{LlmHandler, LlmProvider, OllamaProvider, RoutingBehavior};
pub use message::Message;
pub use session_memory::{
    delete_session, list_sessions, ContextHit, SessionMemory, SessionMemoryConfig,
    SessionMemoryError, StoredMessage,
};
