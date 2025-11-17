use crate::chat::llm_trait::{LLMChat, UsageInfo};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Ollama API request structure
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

/// Ollama API response structure
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

/// Ollama chat implementation
pub struct OllamaChat {
    base_url: String,
    model: String,
    client: reqwest::Client,
    usage_info: Mutex<UsageInfo>,
}

impl OllamaChat {
    /// Creates a new OllamaChat instance
    pub fn new(base_url: String, model: String) -> Self {
        OllamaChat {
            base_url,
            model,
            client: reqwest::Client::new(),
            usage_info: Mutex::new(UsageInfo::default()),
        }
    }

    /// Creates a new OllamaChat instance with default settings
    pub fn default() -> Self {
        Self::new("http://localhost:11434".to_string(), "llama2".to_string())
    }

    /// Internal helper to send requests to Ollama
    async fn send_request(
        &self,
        message: &str,
        system_prompt: Option<&str>,
    ) -> Result<String, String> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: message.to_string(),
            stream: false,
            system: system_prompt.map(|s| s.to_string()),
        };

        let url = format!("{}/api/generate", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Ollama returned error status: {}",
                response.status()
            ));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Update usage information
        if let Ok(mut usage) = self.usage_info.lock() {
            usage.total_requests += 1;
        }

        Ok(ollama_response.response)
    }
}

#[async_trait]
impl LLMChat for OllamaChat {
    /// Sends a message to Ollama and returns the response
    async fn send_message(&self, message: &str) -> Result<String, String> {
        self.send_request(message, None).await
    }

    /// Sends a message with a system prompt to Ollama
    async fn send_message_with_system(
        &self,
        system_prompt: &str,
        message: &str,
    ) -> Result<String, String> {
        self.send_request(message, Some(system_prompt)).await
    }

    /// Sets the model to use
    fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    /// Gets the current model
    fn get_model(&self) -> &str {
        &self.model
    }

    /// Checks if Ollama is available
    async fn health_check(&self) -> Result<bool, String> {
        let url = format!("{}/api/tags", self.base_url);

        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => Err(format!("Health check failed: {}", e)),
        }
    }

    /// Gets usage information about LLM requests
    fn get_usage_info(&self) -> UsageInfo {
        self.usage_info.lock().unwrap().clone()
    }

    /// Resets the usage statistics
    fn reset_usage_info(&mut self) {
        if let Ok(mut usage) = self.usage_info.lock() {
            *usage = UsageInfo::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_chat_creation() {
        let chat = OllamaChat::new("http://localhost:11434".to_string(), "llama2".to_string());
        assert_eq!(chat.get_model(), "llama2");
        assert_eq!(chat.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_set_model() {
        let mut chat = OllamaChat::default();
        chat.set_model("mistral");
        assert_eq!(chat.get_model(), "mistral");
    }
}
