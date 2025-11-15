use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::chat::llm_trait::LLMChat;

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
    done: bool,
}

/// Ollama chat implementation
pub struct OllamaChat {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaChat {
    /// Creates a new OllamaChat instance
    pub fn new(base_url: String, model: String) -> Self {
        OllamaChat {
            base_url,
            model,
            client: reqwest::Client::new(),
        }
    }

    /// Creates a new OllamaChat instance with default settings
    pub fn default() -> Self {
        Self::new("http://localhost:11434".to_string(), "llama2".to_string())
    }
}

#[async_trait]
impl LLMChat for OllamaChat {
    /// Sends a message to Ollama and returns the response
    async fn send_message(&self, message: &str) -> Result<String, String> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: message.to_string(),
            stream: false,
            system: None,
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
            return Err(format!("Ollama returned error status: {}", response.status()));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(ollama_response.response)
    }

    /// Sends a message with a system prompt to Ollama
    async fn send_message_with_system(&self, system_prompt: &str, message: &str) -> Result<String, String> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: message.to_string(),
            stream: false,
            system: Some(system_prompt.to_string()),
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
            return Err(format!("Ollama returned error status: {}", response.status()));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(ollama_response.response)
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
