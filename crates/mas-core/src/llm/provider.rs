use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Role of a message in the conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
        }
    }
}

/// A message in the LLM conversation format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: Role,
    pub content: String,
}

impl LlmMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// Configuration options for LLM completion requests
#[derive(Debug, Clone, Default)]
pub struct CompletionOptions {
    /// Temperature for sampling (0.0 = deterministic, higher = more random)
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    pub top_p: Option<f32>,
    /// Stop sequences
    pub stop: Option<Vec<String>>,
}

impl CompletionOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn stop(mut self, sequences: Vec<String>) -> Self {
        self.stop = Some(sequences);
        self
    }
}

/// Response from an LLM completion request
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    /// The generated text
    pub content: String,
    /// Model used for generation
    pub model: String,
    /// Token usage statistics (if available)
    pub usage: Option<TokenUsage>,
}

/// Token usage statistics
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Errors that can occur when interacting with an LLM provider
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Provider returned error: {0}")]
    ProviderError(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Rate limited: retry after {retry_after:?}")]
    RateLimited { retry_after: Option<std::time::Duration> },

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// The core trait that all LLM providers must implement
///
/// This trait abstracts over different LLM backends (Ollama, OpenAI, Gemini, etc.)
/// allowing the agent system to use any provider interchangeably.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get the name of this provider (e.g., "ollama", "openai", "gemini")
    fn name(&self) -> &str;

    /// Get the default model for this provider
    fn default_model(&self) -> &str;

    /// Complete a conversation with the given messages
    ///
    /// # Arguments
    /// * `messages` - The conversation history including system prompt
    /// * `model` - Optional model override (uses default if None)
    /// * `options` - Optional completion parameters
    async fn complete(
        &self,
        messages: &[LlmMessage],
        model: Option<&str>,
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, LlmError>;

    /// Check if the provider is available and properly configured
    async fn health_check(&self) -> Result<(), LlmError>;

    /// List available models (if supported by the provider)
    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        // Default implementation returns empty - providers can override
        Ok(vec![])
    }
}
