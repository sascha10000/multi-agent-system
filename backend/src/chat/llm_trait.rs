use async_trait::async_trait;

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
}
