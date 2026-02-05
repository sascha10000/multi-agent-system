//! Session memory persistence
//!
//! Each chat session gets its own storage for:
//! - Persistent conversation history (JSON-based, always available)
//! - Semantic search over past messages (requires `memvid` feature)
//! - Crash-safe, single-session storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

#[cfg(feature = "memvid")]
use memvid_rs::{Config, MemvidEncoder, MemvidRetriever, SearchResult};

/// Errors that can occur during session memory operations
#[derive(Debug, Error)]
pub enum SessionMemoryError {
    #[error("Failed to create session directory: {0}")]
    DirectoryCreation(std::io::Error),

    #[error("Failed to initialize encoder: {0}")]
    EncoderInit(String),

    #[error("Failed to initialize retriever: {0}")]
    RetrieverInit(String),

    #[error("Failed to store message: {0}")]
    StoreMessage(String),

    #[error("Failed to search messages: {0}")]
    SearchError(String),

    #[error("Failed to build index: {0}")]
    BuildIndex(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session already exists: {0}")]
    SessionAlreadyExists(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SessionMemoryError>;

/// A message stored in session memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    /// Unique message ID
    pub id: String,
    /// Sender identifier (e.g., "user", "Coordinator", "Researcher")
    pub from: String,
    /// Recipient identifier
    pub to: String,
    /// Message content
    pub content: String,
    /// When the message was sent
    pub timestamp: DateTime<Utc>,
    /// Optional metadata (e.g., elapsed_ms, routing info)
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

impl StoredMessage {
    /// Create a new stored message
    pub fn new(from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from: from.into(),
            to: to.into(),
            content: content.into(),
            timestamp: Utc::now(),
            metadata: None,
        }
    }

    /// Add metadata to the message
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Format message for storage/embedding
    #[allow(dead_code)]
    fn to_storage_text(&self) -> String {
        format!(
            "[{}] {} -> {}: {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.from,
            self.to,
            self.content
        )
    }
}

/// A hit from semantic search
#[derive(Debug, Clone, Serialize)]
pub struct ContextHit {
    /// The matching message
    pub message: StoredMessage,
    /// Relevance score (higher is more relevant)
    pub score: f32,
}

/// Configuration for session memory
#[derive(Debug, Clone)]
pub struct SessionMemoryConfig {
    /// Base directory for session storage
    pub base_path: PathBuf,
    /// Chunk size for text encoding (default: 512)
    pub chunk_size: usize,
    /// Chunk overlap for text encoding (default: 64)
    pub chunk_overlap: usize,
}

impl Default for SessionMemoryConfig {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from("data/sessions"),
            chunk_size: 512,
            chunk_overlap: 64,
        }
    }
}

impl SessionMemoryConfig {
    /// Create config with a custom base path
    pub fn with_base_path(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            ..Default::default()
        }
    }
}

/// Session memory for storing and retrieving conversation messages
///
/// Provides JSON-based storage (always available) and optional semantic search
/// via memvid (requires the `memvid` feature).
pub struct SessionMemory {
    session_id: String,
    #[allow(dead_code)]
    config: SessionMemoryConfig,
    /// Pending messages not yet committed to the video index
    #[allow(dead_code)]
    pending_messages: Vec<StoredMessage>,
    /// All messages (for quick access)
    messages: Vec<StoredMessage>,
    /// Path to the video file (for memvid feature)
    #[allow(dead_code)]
    video_path: PathBuf,
    /// Path to the index database (for memvid feature)
    #[allow(dead_code)]
    index_path: PathBuf,
    /// Path to the messages JSON file (primary storage)
    messages_path: PathBuf,
}

impl SessionMemory {
    /// Create a new session with fresh storage
    pub async fn create(session_id: &str, config: SessionMemoryConfig) -> Result<Self> {
        let session_dir = config.base_path.join(session_id);

        if session_dir.exists() {
            return Err(SessionMemoryError::SessionAlreadyExists(
                session_id.to_string(),
            ));
        }

        fs::create_dir_all(&session_dir)
            .await
            .map_err(SessionMemoryError::DirectoryCreation)?;

        let video_path = session_dir.join("memory.mp4");
        let index_path = session_dir.join("index.db");
        let messages_path = session_dir.join("messages.json");

        fs::write(&messages_path, "[]").await?;

        Ok(Self {
            session_id: session_id.to_string(),
            config,
            pending_messages: Vec::new(),
            messages: Vec::new(),
            video_path,
            index_path,
            messages_path,
        })
    }

    /// Open an existing session
    pub async fn open(session_id: &str, config: SessionMemoryConfig) -> Result<Self> {
        let session_dir = config.base_path.join(session_id);

        if !session_dir.exists() {
            return Err(SessionMemoryError::SessionNotFound(session_id.to_string()));
        }

        let video_path = session_dir.join("memory.mp4");
        let index_path = session_dir.join("index.db");
        let messages_path = session_dir.join("messages.json");

        let messages: Vec<StoredMessage> = if messages_path.exists() {
            let content = fs::read_to_string(&messages_path).await?;
            serde_json::from_str(&content)?
        } else {
            Vec::new()
        };

        Ok(Self {
            session_id: session_id.to_string(),
            config,
            pending_messages: Vec::new(),
            messages,
            video_path,
            index_path,
            messages_path,
        })
    }

    /// Open or create a session
    pub async fn open_or_create(session_id: &str, config: SessionMemoryConfig) -> Result<Self> {
        match Self::open(session_id, config.clone()).await {
            Ok(session) => Ok(session),
            Err(SessionMemoryError::SessionNotFound(_)) => Self::create(session_id, config).await,
            Err(e) => Err(e),
        }
    }

    /// Check if a session exists
    pub async fn exists(session_id: &str, base_path: &Path) -> bool {
        base_path.join(session_id).exists()
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Store a message in the session
    pub async fn store_message(&mut self, message: StoredMessage) -> Result<()> {
        self.messages.push(message.clone());
        self.pending_messages.push(message);

        let json = serde_json::to_string_pretty(&self.messages)?;
        fs::write(&self.messages_path, json).await?;

        Ok(())
    }

    /// Store a user message (convenience method)
    pub async fn store_user_message(&mut self, to: &str, content: &str) -> Result<StoredMessage> {
        let msg = StoredMessage::new("user", to, content);
        self.store_message(msg.clone()).await?;
        Ok(msg)
    }

    /// Store an agent response (convenience method)
    pub async fn store_agent_response(
        &mut self,
        from: &str,
        content: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<StoredMessage> {
        let mut msg = StoredMessage::new(from, "user", content);
        if let Some(meta) = metadata {
            msg = msg.with_metadata(meta);
        }
        self.store_message(msg.clone()).await?;
        Ok(msg)
    }

    /// Build/rebuild the video index from all messages (memvid feature)
    #[cfg(feature = "memvid")]
    pub async fn build_index(&mut self) -> Result<()> {
        if self.messages.is_empty() {
            return Ok(());
        }

        let combined_text: String = self
            .messages
            .iter()
            .map(|m| m.to_storage_text())
            .collect::<Vec<_>>()
            .join("\n\n");

        let memvid_config = Config::default();
        let mut encoder = MemvidEncoder::new(Some(memvid_config))
            .await
            .map_err(|e| SessionMemoryError::EncoderInit(e.to_string()))?;

        encoder
            .add_text(&combined_text, self.config.chunk_size, self.config.chunk_overlap)
            .await
            .map_err(|e| SessionMemoryError::StoreMessage(e.to_string()))?;

        encoder
            .build_video(
                self.video_path.to_string_lossy().as_ref(),
                self.index_path.to_string_lossy().as_ref(),
            )
            .await
            .map_err(|e| SessionMemoryError::BuildIndex(e.to_string()))?;

        self.pending_messages.clear();
        Ok(())
    }

    /// Build index (no-op without memvid feature)
    #[cfg(not(feature = "memvid"))]
    pub async fn build_index(&mut self) -> Result<()> {
        self.pending_messages.clear();
        Ok(())
    }

    /// Search for relevant context (memvid feature: semantic search)
    #[cfg(feature = "memvid")]
    pub async fn search_context(&self, query: &str, top_k: usize) -> Result<Vec<ContextHit>> {
        if !self.index_path.exists() || !self.video_path.exists() {
            return self.keyword_search(query, top_k);
        }

        let mut retriever = MemvidRetriever::new(
            self.video_path.to_string_lossy().as_ref(),
            self.index_path.to_string_lossy().as_ref(),
        )
        .await
        .map_err(|e| SessionMemoryError::RetrieverInit(e.to_string()))?;

        let results: Vec<SearchResult> = retriever
            .search(query, top_k)
            .await
            .map_err(|e| SessionMemoryError::SearchError(e.to_string()))?;

        let mut hits = Vec::new();
        for result in results {
            if let Some(msg) = self.find_message_by_content(&result.text) {
                hits.push(ContextHit {
                    message: msg.clone(),
                    score: result.score,
                });
            }
        }

        Ok(hits)
    }

    /// Search for relevant context (keyword search without memvid)
    #[cfg(not(feature = "memvid"))]
    pub async fn search_context(&self, query: &str, top_k: usize) -> Result<Vec<ContextHit>> {
        self.keyword_search(query, top_k)
    }

    /// Simple keyword-based search fallback
    fn keyword_search(&self, query: &str, top_k: usize) -> Result<Vec<ContextHit>> {
        let query_lower = query.to_lowercase();
        let mut hits: Vec<ContextHit> = self
            .messages
            .iter()
            .filter_map(|m| {
                let content_lower = m.content.to_lowercase();
                if content_lower.contains(&query_lower) {
                    let score = query_lower
                        .split_whitespace()
                        .filter(|word| content_lower.contains(word))
                        .count() as f32
                        / query_lower.split_whitespace().count().max(1) as f32;
                    Some(ContextHit {
                        message: m.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        hits.truncate(top_k);

        Ok(hits)
    }

    #[allow(dead_code)]
    fn find_message_by_content(&self, content: &str) -> Option<&StoredMessage> {
        self.messages.iter().find(|m| {
            let storage_text = m.to_storage_text();
            storage_text.contains(content) || content.contains(&m.content)
        })
    }

    /// Get recent messages (most recent first)
    pub fn get_recent_messages(&self, limit: usize) -> Vec<&StoredMessage> {
        self.messages.iter().rev().take(limit).collect()
    }

    /// Get all messages in chronological order
    pub fn get_all_messages(&self) -> &[StoredMessage] {
        &self.messages
    }

    /// Get the number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Check if there are pending messages not yet indexed
    pub fn has_pending_messages(&self) -> bool {
        !self.pending_messages.is_empty()
    }

    /// Delete the session and all its data
    pub async fn delete(self) -> Result<()> {
        let session_dir = self.config.base_path.join(&self.session_id);
        if session_dir.exists() {
            fs::remove_dir_all(session_dir).await?;
        }
        Ok(())
    }
}

/// List all session IDs in the base directory
pub async fn list_sessions(base_path: &Path) -> Result<Vec<String>> {
    if !base_path.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    let mut entries = fs::read_dir(base_path).await?;

    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                let messages_path = entry.path().join("messages.json");
                if messages_path.exists() {
                    sessions.push(name.to_string());
                }
            }
        }
    }

    Ok(sessions)
}

/// Delete a session by ID
pub async fn delete_session(session_id: &str, base_path: &Path) -> Result<()> {
    let session_dir = base_path.join(session_id);
    if !session_dir.exists() {
        return Err(SessionMemoryError::SessionNotFound(session_id.to_string()));
    }
    fs::remove_dir_all(session_dir).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_session() {
        let temp_dir = TempDir::new().unwrap();
        let config = SessionMemoryConfig::with_base_path(temp_dir.path());

        let session = SessionMemory::create("test-session", config).await.unwrap();
        assert_eq!(session.session_id(), "test-session");
        assert_eq!(session.message_count(), 0);
    }

    #[tokio::test]
    async fn test_store_and_retrieve_messages() {
        let temp_dir = TempDir::new().unwrap();
        let config = SessionMemoryConfig::with_base_path(temp_dir.path());

        let mut session = SessionMemory::create("test-session", config.clone())
            .await
            .unwrap();

        session
            .store_user_message("Coordinator", "Hello, how are you?")
            .await
            .unwrap();
        session
            .store_agent_response("Coordinator", "I'm doing well!", None)
            .await
            .unwrap();

        assert_eq!(session.message_count(), 2);

        let recent = session.get_recent_messages(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].content, "I'm doing well!");
        assert_eq!(recent[1].content, "Hello, how are you?");
    }

    #[tokio::test]
    async fn test_session_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config = SessionMemoryConfig::with_base_path(temp_dir.path());

        {
            let mut session = SessionMemory::create("persistent-session", config.clone())
                .await
                .unwrap();
            session
                .store_user_message("Agent", "Test message")
                .await
                .unwrap();
        }

        let session = SessionMemory::open("persistent-session", config)
            .await
            .unwrap();
        assert_eq!(session.message_count(), 1);
        assert_eq!(session.get_all_messages()[0].content, "Test message");
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let config = SessionMemoryConfig::with_base_path(temp_dir.path());

        SessionMemory::create("session-1", config.clone())
            .await
            .unwrap();
        SessionMemory::create("session-2", config.clone())
            .await
            .unwrap();

        let sessions = list_sessions(temp_dir.path()).await.unwrap();
        assert_eq!(sessions.len(), 2);
        assert!(sessions.contains(&"session-1".to_string()));
        assert!(sessions.contains(&"session-2".to_string()));
    }

    #[tokio::test]
    async fn test_keyword_search() {
        let temp_dir = TempDir::new().unwrap();
        let config = SessionMemoryConfig::with_base_path(temp_dir.path());

        let mut session = SessionMemory::create("search-session", config)
            .await
            .unwrap();

        session
            .store_user_message("Agent", "What is Rust programming?")
            .await
            .unwrap();
        session
            .store_agent_response("Agent", "Rust is a systems programming language.", None)
            .await
            .unwrap();
        session
            .store_user_message("Agent", "Tell me about Python")
            .await
            .unwrap();

        let hits = session.search_context("Rust", 5).await.unwrap();
        assert!(!hits.is_empty());
        assert!(hits[0].message.content.contains("Rust"));
    }
}
