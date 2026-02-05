//! LLM Provider abstraction layer
//!
//! This module provides a trait-based abstraction for different LLM backends,
//! allowing the agent system to use any provider interchangeably.
//!
//! # Example
//!
//! ```ignore
//! use multi_agent_system::llm::{OllamaProvider, LlmHandler, LlmProvider};
//! use std::sync::Arc;
//!
//! // Create a provider
//! let provider = Arc::new(OllamaProvider::new());
//!
//! // Create a handler for an agent
//! let handler = LlmHandler::new(provider)
//!     .with_model("llama3.2")
//!     .with_options(CompletionOptions::new().temperature(0.7));
//!
//! // Register with agent system
//! system.register_agent(agent, Arc::new(handler)).await?;
//! ```

mod handler;
mod ollama;
mod provider;

pub use handler::{LlmHandler, LlmHandlerBuilder};
pub use ollama::OllamaProvider;
pub use provider::{
    CompletionOptions, CompletionResponse, LlmError, LlmMessage, LlmProvider, Role, TokenUsage,
};
