//! Application state management

use chrono::{DateTime, Utc};
use mas_core::AgentSystem;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::session::{create_session_manager, SharedSessionManager};

/// Stored configuration metadata for a system
#[derive(Debug, Clone)]
pub struct ConfigMetadata {
    /// Number of agents
    pub agent_count: usize,
    /// Agent names
    pub agent_names: Vec<String>,
    /// Global timeout setting
    pub global_timeout_secs: u64,
    /// Agent details for introspection
    pub agents: Vec<AgentMetadata>,
}

/// Stored metadata for an agent
#[derive(Debug, Clone)]
pub struct AgentMetadata {
    pub name: String,
    pub role: String,
    pub routing: bool,
    pub routing_behavior: Option<String>,
    pub connections: Vec<ConnectionMetadata>,
}

/// Stored metadata for a connection
#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    pub target: String,
    pub connection_type: String,
    pub timeout_secs: Option<u64>,
}

/// Entry for a registered multi-agent system
pub struct SystemEntry {
    /// The running agent system
    pub system: Arc<AgentSystem>,
    /// Configuration metadata (extracted at registration time)
    pub metadata: ConfigMetadata,
    /// When this system was registered
    pub created_at: DateTime<Utc>,
}

impl SystemEntry {
    /// Create a new system entry
    pub fn new(system: Arc<AgentSystem>, metadata: ConfigMetadata) -> Self {
        Self {
            system,
            metadata,
            created_at: Utc::now(),
        }
    }
}

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    /// Registry of named multi-agent systems
    systems: Arc<RwLock<HashMap<String, SystemEntry>>>,
    /// Session manager for persistent chat sessions
    session_manager: SharedSessionManager,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create a new empty application state
    pub fn new() -> Self {
        Self::with_sessions_path(PathBuf::from("data/sessions"))
    }

    /// Create a new application state with a custom sessions path
    pub fn with_sessions_path(sessions_path: PathBuf) -> Self {
        Self {
            systems: Arc::new(RwLock::new(HashMap::new())),
            session_manager: create_session_manager(sessions_path),
        }
    }

    /// Get the session manager
    pub fn session_manager(&self) -> &SharedSessionManager {
        &self.session_manager
    }

    /// Initialize the application state (load existing sessions, etc.)
    pub async fn init(&self) -> Result<(), String> {
        let mut manager = self.session_manager.write().await;
        manager.init().await.map_err(|e| e.to_string())
    }

    /// Register a new system
    ///
    /// Returns an error if a system with this name already exists
    pub async fn register_system(&self, name: String, entry: SystemEntry) -> Result<(), String> {
        let mut systems = self.systems.write().await;
        if systems.contains_key(&name) {
            return Err(format!("System '{}' already exists", name));
        }
        systems.insert(name, entry);
        Ok(())
    }

    /// Get a system by name
    pub async fn get_system(&self, name: &str) -> Option<Arc<AgentSystem>> {
        let systems = self.systems.read().await;
        systems.get(name).map(|e| e.system.clone())
    }

    /// Get system metadata by name
    pub async fn get_system_metadata(&self, name: &str) -> Option<(ConfigMetadata, DateTime<Utc>)> {
        let systems = self.systems.read().await;
        systems
            .get(name)
            .map(|e| (e.metadata.clone(), e.created_at))
    }

    /// Remove a system by name
    ///
    /// Returns true if the system was removed, false if it didn't exist
    pub async fn remove_system(&self, name: &str) -> bool {
        let mut systems = self.systems.write().await;
        systems.remove(name).is_some()
    }

    /// List all registered systems with summary info
    pub async fn list_systems(&self) -> Vec<(String, ConfigMetadata, DateTime<Utc>)> {
        let systems = self.systems.read().await;
        systems
            .iter()
            .map(|(name, entry)| (name.clone(), entry.metadata.clone(), entry.created_at))
            .collect()
    }

    /// Check if a system exists
    pub async fn system_exists(&self, name: &str) -> bool {
        let systems = self.systems.read().await;
        systems.contains_key(name)
    }

    /// Check if an agent exists in a system
    pub async fn agent_exists(&self, system_name: &str, agent_name: &str) -> bool {
        let systems = self.systems.read().await;
        systems
            .get(system_name)
            .map(|e| e.metadata.agent_names.contains(&agent_name.to_string()))
            .unwrap_or(false)
    }
}
