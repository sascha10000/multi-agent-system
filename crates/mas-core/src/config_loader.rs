//! JSON configuration loader for multi-agent systems
//!
//! This module provides functionality to load and instantiate a multi-agent system
//! from a JSON configuration file. It handles parsing, validation, and instantiation
//! of agents, LLM providers, and connections.
//!
//! # Example Configuration
//!
//! ```json
//! {
//!   "system": {
//!     "global_timeout_secs": 60
//!   },
//!   "llm_providers": {
//!     "default": {
//!       "type": "ollama",
//!       "base_url": "http://localhost:11434",
//!       "default_model": "llama3.2"
//!     }
//!   },
//!   "agents": [
//!     {
//!       "name": "Coordinator",
//!       "role": "Routes requests",
//!       "system_prompt": "You coordinate work.",
//!       "handler": {
//!         "provider": "default",
//!         "routing": true,
//!         "options": { "temperature": 0.3 }
//!       },
//!       "connections": {
//!         "Worker": { "type": "blocking", "timeout_secs": 60 }
//!       }
//!     }
//!   ]
//! }
//! ```

use crate::agent::AgentBuilder;
use crate::agent_system::AgentSystem;
use crate::config::SystemConfig;
use crate::connection::Connection;
use crate::errors::{AgentError, Result};
use crate::llm::{CompletionOptions, LlmHandler, LlmProvider, OllamaProvider, RoutingBehavior};

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// Top-level JSON configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfigJson {
    /// System-wide settings
    pub system: SystemSettings,
    /// Named LLM provider configurations
    pub llm_providers: HashMap<String, LlmProviderConfig>,
    /// Agent definitions
    pub agents: Vec<AgentConfig>,
}

/// System-wide settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    /// Global timeout for blocking connections (in seconds)
    #[serde(default = "default_timeout")]
    pub global_timeout_secs: u64,
}

fn default_timeout() -> u64 {
    30
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            global_timeout_secs: default_timeout(),
        }
    }
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    /// Provider type (currently only "ollama" is supported)
    #[serde(rename = "type")]
    pub provider_type: String,
    /// Base URL for the provider's API
    pub base_url: Option<String>,
    /// Default model to use
    pub default_model: Option<String>,
}

/// Agent definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Unique name for the agent
    pub name: String,
    /// Agent's role description
    #[serde(default)]
    pub role: String,
    /// System prompt for the LLM
    #[serde(default)]
    pub system_prompt: String,
    /// Handler configuration
    pub handler: HandlerConfig,
    /// Connections to other agents
    #[serde(default)]
    pub connections: HashMap<String, ConnectionConfig>,
}

/// Handler configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerConfig {
    /// References a key in llm_providers
    pub provider: String,
    /// Optional model override (uses provider's default if not specified)
    pub model: Option<String>,
    /// Enable routing mode (LLM can decide to forward to connected agents)
    #[serde(default)]
    pub routing: bool,
    /// How the agent should delegate to connected agents (only used when routing=true)
    /// - "best" (default): Forward to the single most appropriate agent
    /// - "all": MUST forward to ALL connected agents and synthesize responses
    /// - "direct_first": Try to answer directly, only forward if lacking expertise
    #[serde(default)]
    pub routing_behavior: RoutingBehavior,
    /// Completion options
    #[serde(default)]
    pub options: CompletionOptionsConfig,
}

/// Completion options for LLM calls
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompletionOptionsConfig {
    /// Temperature for sampling (0.0 = deterministic, higher = more random)
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// Stop sequences
    pub stop: Option<Vec<String>>,
}

impl From<CompletionOptionsConfig> for CompletionOptions {
    fn from(config: CompletionOptionsConfig) -> Self {
        let mut opts = CompletionOptions::new();
        if let Some(temp) = config.temperature {
            opts = opts.temperature(temp);
        }
        if let Some(max) = config.max_tokens {
            opts = opts.max_tokens(max);
        }
        if let Some(top_p) = config.top_p {
            opts = opts.top_p(top_p);
        }
        if let Some(stop) = config.stop {
            opts = opts.stop(stop);
        }
        opts
    }
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Connection type: "blocking" or "notify"
    #[serde(rename = "type")]
    pub connection_type: String,
    /// Optional timeout override for blocking connections (in seconds)
    pub timeout_secs: Option<u64>,
}

/// Validation errors that can occur when loading a configuration
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse JSON: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Duplicate agent name: {0}")]
    DuplicateAgentName(String),

    #[error("Agent '{from}' connects to unknown agent '{to}'")]
    UnknownConnectionTarget { from: String, to: String },

    #[error("Agent '{0}' has a self-connection")]
    SelfConnection(String),

    #[error("Agent '{0}' references unknown provider '{1}'")]
    UnknownProvider(String, String),

    #[error("Unsupported provider type: {0}")]
    UnsupportedProvider(String),

    #[error("Invalid connection type '{1}' for agent '{0}'")]
    InvalidConnectionType(String, String),

    #[error("LLM provider error: {0}")]
    LlmError(String),
}

impl From<ConfigError> for AgentError {
    fn from(err: ConfigError) -> Self {
        AgentError::ConfigError(err.to_string())
    }
}

/// Validate a configuration without instantiating anything
pub fn validate_config(config: &SystemConfigJson) -> std::result::Result<(), ConfigError> {
    // Collect all agent names for validation
    let mut agent_names: HashSet<String> = HashSet::new();

    for agent in &config.agents {
        // Check for duplicate names
        if !agent_names.insert(agent.name.clone()) {
            return Err(ConfigError::DuplicateAgentName(agent.name.clone()));
        }

        // Check provider reference
        if !config.llm_providers.contains_key(&agent.handler.provider) {
            return Err(ConfigError::UnknownProvider(
                agent.name.clone(),
                agent.handler.provider.clone(),
            ));
        }
    }

    // Validate connections (second pass - need all agent names first)
    for agent in &config.agents {
        for (target, conn_config) in &agent.connections {
            // Check for self-connection
            if target == &agent.name {
                return Err(ConfigError::SelfConnection(agent.name.clone()));
            }

            // Check target exists
            if !agent_names.contains(target) {
                return Err(ConfigError::UnknownConnectionTarget {
                    from: agent.name.clone(),
                    to: target.clone(),
                });
            }

            // Validate connection type
            match conn_config.connection_type.to_lowercase().as_str() {
                "blocking" | "notify" => {}
                other => {
                    return Err(ConfigError::InvalidConnectionType(
                        agent.name.clone(),
                        other.to_string(),
                    ));
                }
            }
        }
    }

    // Validate provider configurations
    for (name, provider_config) in &config.llm_providers {
        match provider_config.provider_type.to_lowercase().as_str() {
            "ollama" => {}
            other => {
                return Err(ConfigError::UnsupportedProvider(format!(
                    "{} (provider '{}')",
                    other, name
                )));
            }
        }
    }

    Ok(())
}

/// Create LLM providers from configuration
async fn create_providers(
    configs: &HashMap<String, LlmProviderConfig>,
) -> std::result::Result<HashMap<String, Arc<dyn LlmProvider>>, ConfigError> {
    let mut providers: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();

    for (name, config) in configs {
        let provider: Arc<dyn LlmProvider> = match config.provider_type.to_lowercase().as_str() {
            "ollama" => {
                let base_url = config
                    .base_url
                    .as_deref()
                    .unwrap_or("http://localhost:11434");
                let default_model = config.default_model.as_deref().unwrap_or("llama3.2");

                Arc::new(OllamaProvider::with_config(base_url, default_model))
            }
            other => {
                return Err(ConfigError::UnsupportedProvider(format!(
                    "{} (provider '{}')",
                    other, name
                )));
            }
        };

        debug!("Created provider '{}' ({:?})", name, config.provider_type);
        providers.insert(name.clone(), provider);
    }

    Ok(providers)
}

/// Load and instantiate an AgentSystem from a JSON configuration file
///
/// # Arguments
/// * `json_path` - Path to the JSON configuration file
///
/// # Returns
/// An Arc-wrapped AgentSystem ready for use
///
/// # Errors
/// Returns an error if:
/// - The file cannot be read
/// - The JSON is invalid
/// - Validation fails (duplicate names, invalid references, etc.)
/// - LLM provider creation fails
pub async fn load_system_from_json(json_path: &Path) -> Result<Arc<AgentSystem>> {
    info!("Loading agent system from: {}", json_path.display());

    // Read and parse JSON
    let content = std::fs::read_to_string(json_path).map_err(ConfigError::from)?;
    let config: SystemConfigJson = serde_json::from_str(&content).map_err(ConfigError::from)?;

    // Validate configuration
    validate_config(&config)?;
    info!(
        "Configuration validated: {} agents, {} providers",
        config.agents.len(),
        config.llm_providers.len()
    );

    // Create system config
    let system_config = SystemConfig::with_timeout_secs(config.system.global_timeout_secs);

    // Create LLM providers
    let providers = create_providers(&config.llm_providers).await?;

    // Create the agent system (wrapped in Arc for routing agents)
    let system = Arc::new(AgentSystem::new(system_config));

    // Register all agents
    for agent_config in &config.agents {
        register_agent_from_config(system.clone(), agent_config, &providers).await?;
    }

    info!("Agent system loaded successfully");
    Ok(system)
}

/// Load and parse a JSON configuration without instantiating
///
/// Useful for validation and inspection
pub fn parse_config_file(json_path: &Path) -> std::result::Result<SystemConfigJson, ConfigError> {
    let content = std::fs::read_to_string(json_path)?;
    let config: SystemConfigJson = serde_json::from_str(&content)?;
    validate_config(&config)?;
    Ok(config)
}

/// Register a single agent from its configuration
async fn register_agent_from_config(
    system: Arc<AgentSystem>,
    config: &AgentConfig,
    providers: &HashMap<String, Arc<dyn LlmProvider>>,
) -> Result<()> {
    // Build the agent
    let mut builder = AgentBuilder::new(&config.name)
        .role(&config.role)
        .system_prompt(&config.system_prompt);

    // Add connections
    for (target, conn_config) in &config.connections {
        let connection = match conn_config.connection_type.to_lowercase().as_str() {
            "blocking" => {
                let timeout = conn_config.timeout_secs.map(Duration::from_secs);
                Connection::blocking(timeout)
            }
            "notify" => Connection::notify(),
            _ => unreachable!(), // Already validated
        };
        builder = builder.connection(target, connection);
    }

    let agent = builder.build();
    debug!(
        "Built agent '{}' with {} connections",
        config.name,
        config.connections.len()
    );

    // Get the provider
    let provider = providers
        .get(&config.handler.provider)
        .ok_or_else(|| {
            AgentError::ConfigError(format!(
                "Provider '{}' not found for agent '{}'",
                config.handler.provider, config.name
            ))
        })?
        .clone();

    // Build the handler
    let mut handler = LlmHandler::new(provider);

    // Apply model override
    if let Some(model) = &config.handler.model {
        handler = handler.with_model(model);
    }

    // Apply completion options
    let options: CompletionOptions = config.handler.options.clone().into();
    // Only set options if at least one field is configured
    if config.handler.options.temperature.is_some()
        || config.handler.options.max_tokens.is_some()
        || config.handler.options.top_p.is_some()
        || config.handler.options.stop.is_some()
    {
        handler = handler.with_options(options);
    }

    // Register based on routing mode
    if config.handler.routing {
        handler = handler
            .with_routing()
            .with_routing_behavior(config.handler.routing_behavior);
        debug!(
            "Registering '{}' as routing agent with behavior {:?}",
            config.name, config.handler.routing_behavior
        );
        AgentSystem::register_routing_agent(system, agent, Arc::new(handler)).await?;
    } else {
        debug!("Registering '{}' as simple agent", config.name);
        system.register_agent(agent, Arc::new(handler)).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let json = r#"{
            "system": {},
            "llm_providers": {
                "default": {
                    "type": "ollama"
                }
            },
            "agents": [
                {
                    "name": "Agent1",
                    "handler": { "provider": "default" }
                }
            ]
        }"#;

        let config: SystemConfigJson = serde_json::from_str(json).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.system.global_timeout_secs, 30); // default
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_parse_full_config() {
        let json = r#"{
            "system": {
                "global_timeout_secs": 60
            },
            "llm_providers": {
                "default": {
                    "type": "ollama",
                    "base_url": "http://localhost:11434",
                    "default_model": "llama3.2"
                },
                "fast": {
                    "type": "ollama",
                    "default_model": "llama3.2:1b"
                }
            },
            "agents": [
                {
                    "name": "Coordinator",
                    "role": "Routes requests",
                    "system_prompt": "You coordinate work.",
                    "handler": {
                        "provider": "default",
                        "routing": true,
                        "options": {
                            "temperature": 0.3,
                            "max_tokens": 500
                        }
                    },
                    "connections": {
                        "Worker": { "type": "blocking", "timeout_secs": 60 }
                    }
                },
                {
                    "name": "Worker",
                    "role": "Does work",
                    "handler": { "provider": "fast" },
                    "connections": {}
                }
            ]
        }"#;

        let config: SystemConfigJson = serde_json::from_str(json).unwrap();
        assert_eq!(config.system.global_timeout_secs, 60);
        assert_eq!(config.llm_providers.len(), 2);
        assert_eq!(config.agents.len(), 2);

        let coordinator = &config.agents[0];
        assert_eq!(coordinator.name, "Coordinator");
        assert!(coordinator.handler.routing);
        assert_eq!(coordinator.handler.options.temperature, Some(0.3));
        assert_eq!(coordinator.connections.len(), 1);

        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_duplicate_agent_name() {
        let json = r#"{
            "system": {},
            "llm_providers": { "default": { "type": "ollama" } },
            "agents": [
                { "name": "Agent1", "handler": { "provider": "default" } },
                { "name": "Agent1", "handler": { "provider": "default" } }
            ]
        }"#;

        let config: SystemConfigJson = serde_json::from_str(json).unwrap();
        let result = validate_config(&config);
        assert!(matches!(result, Err(ConfigError::DuplicateAgentName(_))));
    }

    #[test]
    fn test_validate_unknown_connection_target() {
        let json = r#"{
            "system": {},
            "llm_providers": { "default": { "type": "ollama" } },
            "agents": [
                {
                    "name": "Agent1",
                    "handler": { "provider": "default" },
                    "connections": {
                        "NonExistent": { "type": "blocking" }
                    }
                }
            ]
        }"#;

        let config: SystemConfigJson = serde_json::from_str(json).unwrap();
        let result = validate_config(&config);
        assert!(matches!(
            result,
            Err(ConfigError::UnknownConnectionTarget { .. })
        ));
    }

    #[test]
    fn test_validate_self_connection() {
        let json = r#"{
            "system": {},
            "llm_providers": { "default": { "type": "ollama" } },
            "agents": [
                {
                    "name": "Agent1",
                    "handler": { "provider": "default" },
                    "connections": {
                        "Agent1": { "type": "blocking" }
                    }
                }
            ]
        }"#;

        let config: SystemConfigJson = serde_json::from_str(json).unwrap();
        let result = validate_config(&config);
        assert!(matches!(result, Err(ConfigError::SelfConnection(_))));
    }

    #[test]
    fn test_validate_unknown_provider() {
        let json = r#"{
            "system": {},
            "llm_providers": { "default": { "type": "ollama" } },
            "agents": [
                {
                    "name": "Agent1",
                    "handler": { "provider": "nonexistent" }
                }
            ]
        }"#;

        let config: SystemConfigJson = serde_json::from_str(json).unwrap();
        let result = validate_config(&config);
        assert!(matches!(result, Err(ConfigError::UnknownProvider(_, _))));
    }

    #[test]
    fn test_validate_unsupported_provider_type() {
        let json = r#"{
            "system": {},
            "llm_providers": { "default": { "type": "openai" } },
            "agents": [
                {
                    "name": "Agent1",
                    "handler": { "provider": "default" }
                }
            ]
        }"#;

        let config: SystemConfigJson = serde_json::from_str(json).unwrap();
        let result = validate_config(&config);
        assert!(matches!(result, Err(ConfigError::UnsupportedProvider(_))));
    }

    #[test]
    fn test_validate_invalid_connection_type() {
        let json = r#"{
            "system": {},
            "llm_providers": { "default": { "type": "ollama" } },
            "agents": [
                {
                    "name": "Agent1",
                    "handler": { "provider": "default" },
                    "connections": {
                        "Agent2": { "type": "invalid" }
                    }
                },
                { "name": "Agent2", "handler": { "provider": "default" } }
            ]
        }"#;

        let config: SystemConfigJson = serde_json::from_str(json).unwrap();
        let result = validate_config(&config);
        assert!(matches!(
            result,
            Err(ConfigError::InvalidConnectionType(_, _))
        ));
    }

    #[test]
    fn test_completion_options_conversion() {
        let config = CompletionOptionsConfig {
            temperature: Some(0.7),
            max_tokens: Some(100),
            top_p: Some(0.9),
            stop: Some(vec!["END".to_string()]),
        };

        let options: CompletionOptions = config.into();
        assert_eq!(options.temperature, Some(0.7));
        assert_eq!(options.max_tokens, Some(100));
        assert_eq!(options.top_p, Some(0.9));
        assert_eq!(options.stop, Some(vec!["END".to_string()]));
    }
}
