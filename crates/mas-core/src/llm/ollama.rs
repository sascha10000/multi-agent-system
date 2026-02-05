use super::provider::{
    CompletionOptions, CompletionResponse, LlmError, LlmMessage, LlmProvider, TokenUsage,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Ollama LLM provider
///
/// Connects to a local or remote Ollama instance via its REST API.
/// Default endpoint is http://localhost:11434
pub struct OllamaProvider {
    client: Client,
    base_url: String,
    default_model: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider with default settings
    pub fn new() -> Self {
        Self::with_config("http://localhost:11434", "llama3.2")
    }

    /// Create a new Ollama provider with custom endpoint and model
    pub fn with_config(base_url: impl Into<String>, default_model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            default_model: default_model.into(),
        }
    }

    /// Create a new Ollama provider with a custom reqwest client
    pub fn with_client(client: Client, base_url: impl Into<String>, default_model: impl Into<String>) -> Self {
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            default_model: default_model.into(),
        }
    }

    /// Auto-detect available models and create provider with the first one
    ///
    /// This is useful when you don't know which models are installed.
    /// Returns error if Ollama is not running or no models are available.
    pub async fn detect() -> Result<Self, LlmError> {
        Self::detect_at("http://localhost:11434").await
    }

    /// Auto-detect available models at a custom endpoint
    pub async fn detect_at(base_url: impl Into<String>) -> Result<Self, LlmError> {
        let base_url = base_url.into();
        let client = Client::new();
        let url = format!("{}/api/tags", base_url.trim_end_matches('/'));

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Cannot connect to Ollama: {}", e)))?;

        let body = response
            .text()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        let tags: OllamaTagsResponse = serde_json::from_str(&body)
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        let first_model = tags
            .models
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::ConfigurationError("No models available in Ollama".into()))?;

        Ok(Self::with_client(client, base_url, first_model.name))
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Ollama chat API request format
#[derive(Debug, Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: &'a [LlmMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

/// Ollama-specific options
#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

impl From<CompletionOptions> for OllamaOptions {
    fn from(opts: CompletionOptions) -> Self {
        Self {
            temperature: opts.temperature,
            num_predict: opts.max_tokens,
            top_p: opts.top_p,
            stop: opts.stop,
        }
    }
}

/// Ollama chat API response format
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    model: String,
    message: OllamaMessage,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

/// Ollama tags (models) API response
#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
}

/// Ollama error response
#[derive(Debug, Deserialize)]
struct OllamaErrorResponse {
    error: String,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    async fn complete(
        &self,
        messages: &[LlmMessage],
        model: Option<&str>,
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, LlmError> {
        let model = model.unwrap_or(&self.default_model);
        let url = format!("{}/api/chat", self.base_url);

        let request = OllamaChatRequest {
            model,
            messages,
            stream: false,
            options: options.map(|o| o.into()),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if !status.is_success() {
            // Try to parse error response
            if let Ok(err_response) = serde_json::from_str::<OllamaErrorResponse>(&body) {
                if err_response.error.contains("model") && err_response.error.contains("not found") {
                    return Err(LlmError::ModelNotFound(model.to_string()));
                }
                return Err(LlmError::ProviderError(err_response.error));
            }
            return Err(LlmError::ProviderError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let chat_response: OllamaChatResponse = serde_json::from_str(&body)
            .map_err(|e| LlmError::ParseError(format!("{}: {}", e, body)))?;

        let usage = match (chat_response.prompt_eval_count, chat_response.eval_count) {
            (Some(prompt), Some(completion)) => Some(TokenUsage {
                prompt_tokens: prompt,
                completion_tokens: completion,
                total_tokens: prompt + completion,
            }),
            _ => None,
        };

        Ok(CompletionResponse {
            content: chat_response.message.content,
            model: chat_response.model,
            usage,
        })
    }

    async fn health_check(&self) -> Result<(), LlmError> {
        let url = format!("{}/api/tags", self.base_url);

        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Cannot connect to Ollama: {}", e)))?
            .error_for_status()
            .map_err(|e| LlmError::ProviderError(e.to_string()))?;

        Ok(())
    }

    async fn list_models(&self) -> Result<Vec<String>, LlmError> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        let body = response
            .text()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        let tags: OllamaTagsResponse = serde_json::from_str(&body)
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = OllamaProvider::new();
        assert_eq!(provider.name(), "ollama");
        assert_eq!(provider.default_model(), "llama3.2");
    }

    #[test]
    fn test_custom_config() {
        let provider = OllamaProvider::with_config("http://custom:8080", "mistral");
        assert_eq!(provider.base_url, "http://custom:8080");
        assert_eq!(provider.default_model(), "mistral");
    }

    #[test]
    fn test_url_trailing_slash_removed() {
        let provider = OllamaProvider::with_config("http://localhost:11434/", "llama3.2");
        assert_eq!(provider.base_url, "http://localhost:11434");
    }
}
