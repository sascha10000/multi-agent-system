use async_trait::async_trait;

/// Usage information for LLM requests
#[derive(Debug, Clone, Default)]
pub struct UsageInfo {
    /// Total number of requests made
    pub total_requests: u64,
    /// Total prompt tokens used (if available)
    pub prompt_tokens: Option<u64>,
    /// Total completion tokens used (if available)
    pub completion_tokens: Option<u64>,
    /// Total tokens used (if available)
    pub total_tokens: Option<u64>,
    /// Additional metadata
    pub metadata: Option<String>,
}

/// Trait for LLM communication
#[async_trait]
pub trait LLMChat {
    /// Sends a message to the LLM and returns the response
    async fn send_message(&self, message: &str) -> Result<String, String>;

    /// Sends a message with a system prompt to the LLM
    async fn send_message_with_system(&self, system_prompt: &str, message: &str) -> Result<String, String>;

    /// Sets the default model to use
    fn set_model(&mut self, model: &str);

    /// Gets the current model name
    fn get_model(&self) -> &str;

    /// Checks if the LLM service is available
    async fn health_check(&self) -> Result<bool, String>;

    /// Gets usage information about LLM requests
    fn get_usage_info(&self) -> UsageInfo;

    /// Resets the usage statistics
    fn reset_usage_info(&mut self);
}
