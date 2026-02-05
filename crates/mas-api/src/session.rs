//! Session management for the REST API
//!
//! Manages chat sessions with persistent memory using memvid.

use chrono::{DateTime, Utc};
use mas_core::{
    session_memory::{self, SessionMemoryError},
    ContextHit, SessionMemory, SessionMemoryConfig, StoredMessage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Errors that can occur during session operations
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Session already exists: {0}")]
    AlreadyExists(String),

    #[error("System not found: {0}")]
    SystemNotFound(String),

    #[error("Memory error: {0}")]
    Memory(#[from] SessionMemoryError),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, SessionError>;

/// Summary information about a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session ID
    pub id: String,
    /// Name of the associated multi-agent system
    pub system_name: String,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// Number of messages in the session
    pub message_count: usize,
    /// When the last message was sent (if any)
    pub last_activity: Option<DateTime<Utc>>,
}

/// A chat session with persistent memory
pub struct Session {
    /// Unique session ID
    pub id: String,
    /// Name of the associated multi-agent system
    pub system_name: String,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// Session memory for persistence and search
    pub memory: SessionMemory,
}

impl Session {
    /// Get summary info for this session
    pub fn info(&self) -> SessionInfo {
        let messages = self.memory.get_all_messages();
        let last_activity = messages.last().map(|m| m.timestamp);

        SessionInfo {
            id: self.id.clone(),
            system_name: self.system_name.clone(),
            created_at: self.created_at,
            message_count: messages.len(),
            last_activity,
        }
    }
}

/// Metadata stored alongside the session for quick access
#[derive(Debug, Serialize, Deserialize)]
struct SessionMetadata {
    system_name: String,
    created_at: DateTime<Utc>,
}

/// Manages all chat sessions
pub struct SessionManager {
    /// In-memory cache of active sessions
    sessions: HashMap<String, Session>,
    /// Base directory for session storage
    base_path: PathBuf,
    /// Memory configuration
    memory_config: SessionMemoryConfig,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(base_path: PathBuf) -> Self {
        let memory_config = SessionMemoryConfig::with_base_path(&base_path);
        Self {
            sessions: HashMap::new(),
            base_path,
            memory_config,
        }
    }

    /// Initialize the session manager and load existing sessions
    pub async fn init(&mut self) -> Result<()> {
        // Ensure base directory exists
        tokio::fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| SessionError::Internal(format!("Failed to create sessions directory: {}", e)))?;

        // Load existing sessions
        let session_ids = session_memory::list_sessions(&self.base_path)
            .await
            .map_err(SessionError::Memory)?;

        for session_id in session_ids {
            match self.load_session(&session_id).await {
                Ok(session) => {
                    info!("Loaded session: {}", session_id);
                    self.sessions.insert(session_id, session);
                }
                Err(e) => {
                    warn!("Failed to load session {}: {}", session_id, e);
                }
            }
        }

        info!("Session manager initialized with {} sessions", self.sessions.len());
        Ok(())
    }

    /// Load a session from disk
    async fn load_session(&self, session_id: &str) -> Result<Session> {
        let memory = SessionMemory::open(session_id, self.memory_config.clone())
            .await
            .map_err(SessionError::Memory)?;

        // Load metadata
        let metadata_path = self.base_path.join(session_id).join("metadata.json");
        let metadata: SessionMetadata = if metadata_path.exists() {
            let content = tokio::fs::read_to_string(&metadata_path)
                .await
                .map_err(|e| SessionError::Internal(format!("Failed to read metadata: {}", e)))?;
            serde_json::from_str(&content)
                .map_err(|e| SessionError::Internal(format!("Failed to parse metadata: {}", e)))?
        } else {
            // Fallback for sessions without metadata
            SessionMetadata {
                system_name: "unknown".to_string(),
                created_at: Utc::now(),
            }
        };

        Ok(Session {
            id: session_id.to_string(),
            system_name: metadata.system_name,
            created_at: metadata.created_at,
            memory,
        })
    }

    /// Create a new session for a system
    pub async fn create_session(&mut self, system_name: &str) -> Result<SessionInfo> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let created_at = Utc::now();

        // Create the session memory
        let memory = SessionMemory::create(&session_id, self.memory_config.clone())
            .await
            .map_err(|e| match e {
                SessionMemoryError::SessionAlreadyExists(id) => SessionError::AlreadyExists(id),
                other => SessionError::Memory(other),
            })?;

        // Save metadata
        let metadata = SessionMetadata {
            system_name: system_name.to_string(),
            created_at,
        };
        let metadata_path = self.base_path.join(&session_id).join("metadata.json");
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| SessionError::Internal(format!("Failed to serialize metadata: {}", e)))?;
        tokio::fs::write(&metadata_path, metadata_json)
            .await
            .map_err(|e| SessionError::Internal(format!("Failed to write metadata: {}", e)))?;

        let session = Session {
            id: session_id.clone(),
            system_name: system_name.to_string(),
            created_at,
            memory,
        };

        let info = session.info();
        self.sessions.insert(session_id, session);

        info!("Created session {} for system {}", info.id, system_name);
        Ok(info)
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    /// Get a mutable session by ID
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(session_id)
    }

    /// List all sessions, optionally filtered by system name
    pub fn list_sessions(&self, system_name: Option<&str>) -> Vec<SessionInfo> {
        self.sessions
            .values()
            .filter(|s| system_name.map_or(true, |name| s.system_name == name))
            .map(|s| s.info())
            .collect()
    }

    /// Delete a session
    pub async fn delete_session(&mut self, session_id: &str) -> Result<()> {
        let session = self.sessions.remove(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        // Delete the session files
        session.memory.delete().await.map_err(SessionError::Memory)?;

        // Also delete the metadata file
        let metadata_path = self.base_path.join(session_id).join("metadata.json");
        if metadata_path.exists() {
            let _ = tokio::fs::remove_file(&metadata_path).await;
        }

        info!("Deleted session: {}", session_id);
        Ok(())
    }

    /// Store a user message in a session
    pub async fn store_user_message(
        &mut self,
        session_id: &str,
        target_agent: &str,
        content: &str,
    ) -> Result<StoredMessage> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        session
            .memory
            .store_user_message(target_agent, content)
            .await
            .map_err(SessionError::Memory)
    }

    /// Store an agent response in a session
    pub async fn store_agent_response(
        &mut self,
        session_id: &str,
        from_agent: &str,
        content: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<StoredMessage> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        session
            .memory
            .store_agent_response(from_agent, content, metadata)
            .await
            .map_err(SessionError::Memory)
    }

    /// Get conversation history for a session
    pub fn get_history(&self, session_id: &str, limit: Option<usize>) -> Result<Vec<StoredMessage>> {
        let session = self.sessions.get(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        let messages = match limit {
            Some(n) => session.memory.get_recent_messages(n).into_iter().cloned().collect(),
            None => session.memory.get_all_messages().to_vec(),
        };

        Ok(messages)
    }

    /// Search for relevant context in a session
    pub async fn search_session(
        &self,
        session_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<ContextHit>> {
        let session = self.sessions.get(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        session
            .memory
            .search_context(query, top_k)
            .await
            .map_err(SessionError::Memory)
    }

    /// Build the search index for a session
    pub async fn build_index(&mut self, session_id: &str) -> Result<()> {
        let session = self.sessions.get_mut(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        session.memory.build_index().await.map_err(SessionError::Memory)
    }

    /// Get the system name for a session
    pub fn get_session_system(&self, session_id: &str) -> Option<&str> {
        self.sessions.get(session_id).map(|s| s.system_name.as_str())
    }
}

/// Thread-safe wrapper for SessionManager
pub type SharedSessionManager = Arc<RwLock<SessionManager>>;

/// Create a new shared session manager
pub fn create_session_manager(base_path: PathBuf) -> SharedSessionManager {
    Arc::new(RwLock::new(SessionManager::new(base_path)))
}
