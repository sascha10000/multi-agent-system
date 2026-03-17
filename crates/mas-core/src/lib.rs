pub mod agent;
pub mod agent_system;
pub mod config;
pub mod config_loader;
pub mod connection;
pub mod conversation;
pub mod database;
pub mod database_handler;
pub mod decision;
pub mod errors;
pub mod llm;
pub mod message;
pub mod session_memory;
pub mod tool;
pub mod tool_handler;
pub mod tracer;

// Re-export commonly used types
pub use agent::{Agent, AgentBuilder};
pub use agent_system::{AgentSystem, MessageHandler, RoutingHandler, SendResult, ToolInfo};
pub use config::SystemConfig;
pub use config_loader::{load_system_from_json, parse_config_file, validate_config, SystemConfigJson};
pub use connection::{Connection, ConnectionType};
pub use decision::{ConversationTurn, EvaluationDecision, ForwardTarget, HandlerDecision};
pub use errors::{AgentError, Result};
pub use llm::{LlmHandler, LlmProvider, OllamaProvider, RoutingBehavior};
pub use message::Message;
pub use session_memory::{
    delete_session, list_sessions, ContextHit, SessionMemory, SessionMemoryConfig,
    SessionMemoryError, StoredMessage,
};
pub use database::{Database, DatabaseConfig, DatabaseType};
pub use database_handler::DatabaseHandler;
pub use tool::{EndpointType, HttpMethod, ResponseFormat, ResponseMapping, Tool, ToolConfig, ToolEndpoint};
pub use tool_handler::ToolHandler;
pub use tracer::{TraceCollector, TraceEvent, TraceEventType};
