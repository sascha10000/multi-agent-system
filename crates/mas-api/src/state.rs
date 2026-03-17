//! Application state management

use chrono::{DateTime, Utc};
use mas_auth::{AuthState, FromRef, JwtConfig};
use mas_core::config_loader::SystemConfigJson;
use mas_core::{load_system_from_json, AgentSystem};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

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
    pub routing: bool,
    pub routing_behavior: Option<String>,
    pub connections: Vec<ConnectionMetadata>,
    pub entry_point: bool,
}

/// Stored metadata for a connection
#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    pub target: String,
    pub connection_type: String,
    pub timeout_secs: Option<u64>,
}

/// Extract metadata from a SystemConfigJson
pub fn extract_metadata(config: &SystemConfigJson) -> ConfigMetadata {
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
                routing: agent.handler.routing,
                routing_behavior: if agent.handler.routing {
                    Some(format!("{:?}", agent.handler.routing_behavior).to_lowercase())
                } else {
                    None
                },
                connections,
                entry_point: agent.entry_point,
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

/// Errors that can occur during system storage operations
#[derive(Debug, Error)]
pub enum SystemStoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("System not found: {0}")]
    NotFound(String),
    #[error("System already exists: {0}")]
    AlreadyExists(String),
    #[error("Invalid system name: {0}")]
    InvalidName(String),
}

/// Store for persisting system configurations to disk
#[derive(Clone)]
pub struct SystemStore {
    base_path: PathBuf,
}

impl SystemStore {
    /// Create a new system store with the given base path
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// Initialize the store (create directory if needed)
    pub async fn init(&self) -> Result<(), SystemStoreError> {
        tokio::fs::create_dir_all(&self.base_path).await?;
        Ok(())
    }

    /// Get the file path for a system configuration
    fn config_path(&self, name: &str) -> PathBuf {
        self.base_path.join(format!("{}.json", name))
    }

    /// Validate a system name (no path separators, etc.)
    fn validate_name(name: &str) -> Result<(), SystemStoreError> {
        if name.is_empty() {
            return Err(SystemStoreError::InvalidName(
                "System name cannot be empty".to_string(),
            ));
        }
        if name.contains('/') || name.contains('\\') || name.contains('\0') {
            return Err(SystemStoreError::InvalidName(format!(
                "System name '{}' contains invalid characters",
                name
            )));
        }
        Ok(())
    }

    /// Save a system configuration to disk
    pub async fn save(&self, name: &str, config: &SystemConfigJson) -> Result<(), SystemStoreError> {
        Self::validate_name(name)?;
        let path = self.config_path(name);
        let json = serde_json::to_string_pretty(config)?;
        tokio::fs::write(&path, json).await?;
        info!("Saved system configuration '{}' to {:?}", name, path);
        Ok(())
    }

    /// Load a system configuration from disk
    pub async fn load(&self, name: &str) -> Result<SystemConfigJson, SystemStoreError> {
        Self::validate_name(name)?;
        let path = self.config_path(name);
        if !path.exists() {
            return Err(SystemStoreError::NotFound(name.to_string()));
        }
        let json = tokio::fs::read_to_string(&path).await?;
        let config: SystemConfigJson = serde_json::from_str(&json)?;
        Ok(config)
    }

    /// Delete a system configuration from disk
    pub async fn delete(&self, name: &str) -> Result<(), SystemStoreError> {
        Self::validate_name(name)?;
        let path = self.config_path(name);
        if !path.exists() {
            return Err(SystemStoreError::NotFound(name.to_string()));
        }
        tokio::fs::remove_file(&path).await?;
        info!("Deleted system configuration '{}' from {:?}", name, path);
        Ok(())
    }

    /// List all stored system names
    pub async fn list(&self) -> Result<Vec<String>, SystemStoreError> {
        let mut names = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.base_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    if let Some(name) = stem.to_str() {
                        names.push(name.to_string());
                    }
                }
            }
        }
        Ok(names)
    }

    /// Load all stored system configurations
    pub async fn load_all(&self) -> Result<Vec<(String, SystemConfigJson)>, SystemStoreError> {
        let names = self.list().await?;
        let mut configs = Vec::new();
        for name in names {
            match self.load(&name).await {
                Ok(config) => configs.push((name, config)),
                Err(e) => {
                    warn!("Failed to load system '{}': {}", name, e);
                }
            }
        }
        Ok(configs)
    }

    /// Check if a system configuration exists on disk
    pub async fn exists(&self, name: &str) -> bool {
        if Self::validate_name(name).is_err() {
            return false;
        }
        self.config_path(name).exists()
    }
}

/// Entry for a registered multi-agent system
pub struct SystemEntry {
    /// The running agent system
    pub system: Arc<AgentSystem>,
    /// Configuration metadata (extracted at registration time)
    pub metadata: ConfigMetadata,
    /// When this system was registered
    pub created_at: DateTime<Utc>,
    /// The original configuration (for persistence)
    pub config: SystemConfigJson,
}

impl SystemEntry {
    /// Create a new system entry
    pub fn new(system: Arc<AgentSystem>, metadata: ConfigMetadata, config: SystemConfigJson) -> Self {
        Self {
            system,
            metadata,
            created_at: Utc::now(),
            config,
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
    /// System store for persisting configurations
    system_store: SystemStore,
    /// Database connection pool
    db: Option<SqlitePool>,
    /// JWT configuration
    jwt_config: Arc<JwtConfig>,
    /// Whether auth is disabled (dev mode)
    auth_disabled: bool,
}

/// Implement FromRef so the AuthenticatedUser extractor can pull AuthState from AppState
impl FromRef<AppState> for AuthState {
    fn from_ref(state: &AppState) -> AuthState {
        AuthState {
            jwt_config: state.jwt_config.clone(),
            auth_disabled: state.auth_disabled,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    /// Create a new empty application state
    pub fn new() -> Self {
        Self::with_paths(
            PathBuf::from("data/sessions"),
            PathBuf::from("data/systems"),
        )
    }

    /// Create a new application state with custom paths
    pub fn with_paths(sessions_path: PathBuf, systems_path: PathBuf) -> Self {
        let auth_disabled = std::env::var("MAS_DISABLE_AUTH")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        Self {
            systems: Arc::new(RwLock::new(HashMap::new())),
            session_manager: create_session_manager(sessions_path),
            system_store: SystemStore::new(systems_path),
            db: None,
            jwt_config: Arc::new(JwtConfig::for_testing()),
            auth_disabled,
        }
    }

    /// Create a new application state with a custom sessions path (legacy compatibility)
    pub fn with_sessions_path(sessions_path: PathBuf) -> Self {
        Self::with_paths(sessions_path, PathBuf::from("data/systems"))
    }

    /// Set the database pool
    pub fn with_db(mut self, pool: SqlitePool) -> Self {
        self.db = Some(pool);
        self
    }

    /// Set the JWT configuration
    pub fn with_jwt_config(mut self, config: JwtConfig) -> Self {
        self.jwt_config = Arc::new(config);
        self
    }

    /// Set whether auth is disabled
    pub fn with_auth_disabled(mut self, disabled: bool) -> Self {
        self.auth_disabled = disabled;
        self
    }

    /// Get the database pool
    pub fn db(&self) -> &SqlitePool {
        self.db
            .as_ref()
            .expect("Database pool not initialized. Call with_db() first.")
    }

    /// Get the JWT config
    pub fn jwt_config(&self) -> &JwtConfig {
        &self.jwt_config
    }

    /// Whether auth is disabled (dev mode)
    pub fn is_auth_disabled(&self) -> bool {
        self.auth_disabled
    }

    /// Get the session manager
    pub fn session_manager(&self) -> &SharedSessionManager {
        &self.session_manager
    }

    /// Get the system store
    pub fn system_store(&self) -> &SystemStore {
        &self.system_store
    }

    /// Initialize the application state (load existing sessions and systems)
    pub async fn init(&self) -> Result<(), String> {
        // Initialize session manager
        {
            let mut manager = self.session_manager.write().await;
            manager.init().await.map_err(|e| e.to_string())?;
        }

        // Initialize system store and load persisted systems
        self.system_store
            .init()
            .await
            .map_err(|e| e.to_string())?;

        self.load_persisted_systems().await?;

        Ok(())
    }

    /// Load all persisted system configurations and register them
    async fn load_persisted_systems(&self) -> Result<(), String> {
        let configs = self
            .system_store
            .load_all()
            .await
            .map_err(|e| e.to_string())?;

        if configs.is_empty() {
            info!("No persisted systems found");
            return Ok(());
        }

        info!("Loading {} persisted system(s)...", configs.len());

        for (name, config) in configs {
            match self.register_system_from_config(name.clone(), config).await {
                Ok(_) => info!("Loaded system: {}", name),
                Err(e) => error!("Failed to load system '{}': {}", name, e),
            }
        }

        Ok(())
    }

    /// Register a system from a configuration (used during loading and creation)
    async fn register_system_from_config(
        &self,
        name: String,
        config: SystemConfigJson,
    ) -> Result<ConfigMetadata, String> {
        // Extract metadata before we use the config
        let metadata = extract_metadata(&config);

        // Write config to a temporary file for load_system_from_json
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("mas-api-{}.json", uuid::Uuid::new_v4()));

        let config_json = serde_json::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        std::fs::write(&temp_file, &config_json)
            .map_err(|e| format!("Failed to write temp config: {}", e))?;

        // Load the system
        let system = load_system_from_json(&temp_file)
            .await
            .map_err(|e| e.to_string())?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        // Create the entry and register
        let entry = SystemEntry::new(system, metadata.clone(), config);

        let mut systems = self.systems.write().await;
        if systems.contains_key(&name) {
            return Err(format!("System '{}' already exists", name));
        }
        systems.insert(name, entry);

        Ok(metadata)
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

    /// Get the full system config by name
    pub async fn get_system_config(&self, name: &str) -> Option<(SystemConfigJson, DateTime<Utc>)> {
        let systems = self.systems.read().await;
        systems
            .get(name)
            .map(|e| (e.config.clone(), e.created_at))
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
