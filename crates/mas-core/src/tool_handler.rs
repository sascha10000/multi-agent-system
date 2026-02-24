//! HTTP and MCP tool handler
//!
//! This module implements the `MessageHandler` trait for tools, allowing them
//! to be used as message recipients in the agent system. When a message is sent
//! to a tool, the handler:
//!
//! 1. Parses parameters from the message content (JSON or plain text)
//! 2. Substitutes ${param} placeholders in URL, headers, and body
//! 3. Substitutes ${ENV_VAR} placeholders with environment variables
//! 4. Executes the HTTP request (or MCP protocol for MCP endpoints)
//! 5. Extracts and formats the response according to response_mapping
//!
//! ## MCP Support
//!
//! For MCP (Model Context Protocol) endpoints, the handler uses the `rmcp` SDK
//! to manage the protocol lifecycle (initialize, call tool, etc.)
//! The MCP client is created lazily on the first call and reused for subsequent
//! calls, avoiding the overhead of reconnecting for every request.

use crate::agent::Agent;
use crate::agent_system::MessageHandler;
use crate::message::Message;
use crate::tool::{EndpointType, HttpMethod, ResponseFormat, Tool};

use async_trait::async_trait;
use reqwest::Client;
use rmcp::model::{CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation, ProtocolVersion};
use rmcp::service::RunningService;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransport;
use rmcp::{RoleClient, ServiceExt};
use std::borrow::Cow;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Default timeout for tool HTTP requests (30 seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Cached MCP client connection
type McpClient = RunningService<RoleClient, ClientInfo>;

/// Handler that executes HTTP requests for a tool
pub struct ToolHandler {
    tool: Arc<Tool>,
    client: Client,
    /// Cached MCP client — lazily initialized on first MCP call, reused across calls.
    mcp_client: Mutex<Option<McpClient>>,
}

impl ToolHandler {
    /// Create a new tool handler
    pub fn new(tool: Arc<Tool>) -> Self {
        let timeout = tool
            .config
            .timeout_secs
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            tool,
            client,
            mcp_client: Mutex::new(None),
        }
    }

    /// Parse parameters from message content, preserving JSON types
    ///
    /// Tries to parse as JSON first. If that fails, treats the entire
    /// content as a single "input" parameter. Returns typed Values to
    /// preserve booleans, numbers, and nested objects for MCP tools.
    fn parse_parameters(&self, content: &str) -> HashMap<String, Value> {
        let mut params = HashMap::new();

        // Try to parse as JSON
        if let Ok(json) = serde_json::from_str::<Value>(content) {
            if let Some(obj) = json.as_object() {
                for (key, value) in obj {
                    params.insert(key.clone(), value.clone());
                }
            } else {
                // JSON but not an object - use as "input"
                params.insert("input".to_string(), json);
            }
        } else {
            // Not JSON - use entire content as "input" and "query" for convenience
            params.insert("input".to_string(), Value::String(content.to_string()));
            params.insert("query".to_string(), Value::String(content.to_string()));
        }

        params
    }

    /// Convert typed parameters to strings for HTTP placeholder substitution
    fn params_to_strings(&self, params: &HashMap<String, Value>) -> HashMap<String, String> {
        params
            .iter()
            .map(|(k, v)| {
                let s = match v {
                    Value::String(s) => s.clone(),
                    Value::Null => String::new(),
                    // For non-string types, use their JSON representation
                    other => other.to_string(),
                };
                (k.clone(), s)
            })
            .collect()
    }

    /// Substitute ${param} placeholders in a string with values from params
    /// Also substitutes ${ENV_VAR} with environment variables
    fn substitute_placeholders(&self, template: &str, params: &HashMap<String, String>) -> String {
        let mut result = template.to_string();

        // First, substitute parameters
        for (key, value) in params {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }

        // Then, substitute environment variables for remaining ${...} patterns
        let mut final_result = String::new();
        let mut chars = result.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' && chars.peek() == Some(&'{') {
                chars.next(); // consume '{'
                let mut var_name = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '}' {
                        chars.next(); // consume '}'
                        break;
                    }
                    var_name.push(chars.next().unwrap());
                }
                // Try to get environment variable
                match env::var(&var_name) {
                    Ok(value) => final_result.push_str(&value),
                    Err(_) => {
                        // Keep the placeholder if env var not found
                        warn!("Environment variable {} not found", var_name);
                        final_result.push_str(&format!("${{{}}}", var_name));
                    }
                }
            } else {
                final_result.push(c);
            }
        }

        final_result
    }

    /// Substitute placeholders in a JSON value
    fn substitute_json(&self, value: &Value, params: &HashMap<String, String>) -> Value {
        match value {
            Value::String(s) => Value::String(self.substitute_placeholders(s, params)),
            Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    new_obj.insert(k.clone(), self.substitute_json(v, params));
                }
                Value::Object(new_obj)
            }
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.substitute_json(v, params)).collect())
            }
            other => other.clone(),
        }
    }

    /// Extract data from response using a JSONPath-like expression
    ///
    /// Supports simple paths like "$.data.results" or "$.items[0].name"
    fn extract_path(&self, response: &Value, path: &str) -> Option<Value> {
        if path.is_empty() || path == "$" {
            return Some(response.clone());
        }

        // Remove leading "$." if present
        let path = path.strip_prefix("$.").unwrap_or(path);
        let path = path.strip_prefix('$').unwrap_or(path);

        let mut current = response;

        for segment in path.split('.') {
            if segment.is_empty() {
                continue;
            }

            // Check for array index like "items[0]"
            if let Some(bracket_pos) = segment.find('[') {
                let key = &segment[..bracket_pos];
                let index_str = &segment[bracket_pos + 1..segment.len() - 1];

                // Navigate to the key first
                if !key.is_empty() {
                    current = current.get(key)?;
                }

                // Then get the array index
                let index: usize = index_str.parse().ok()?;
                current = current.get(index)?;
            } else {
                current = current.get(segment)?;
            }
        }

        Some(current.clone())
    }

    /// Format the response according to the configured format
    fn format_response(&self, value: &Value) -> String {
        match self.tool.config.response_mapping.format {
            ResponseFormat::Json => {
                serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
            }
            ResponseFormat::Text => match value {
                Value::String(s) => s.clone(),
                Value::Array(arr) => arr
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        _ => v.to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => value.to_string(),
            },
            ResponseFormat::Markdown => {
                // Simple markdown formatting
                match value {
                    Value::String(s) => s.clone(),
                    Value::Array(arr) => arr
                        .iter()
                        .map(|v| format!("- {}", v))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    Value::Object(obj) => obj
                        .iter()
                        .map(|(k, v)| format!("**{}**: {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    _ => value.to_string(),
                }
            }
        }
    }

    /// Execute the HTTP request
    async fn execute_request(&self, params: &HashMap<String, Value>) -> Result<String, String> {
        // Convert to strings for placeholder substitution in URLs/headers
        let string_params = self.params_to_strings(params);
        let endpoint = &self.tool.config.endpoint;

        // Build URL with substitutions
        let url = self.substitute_placeholders(&endpoint.url, &string_params);
        debug!("[{}] Making {} request to: {}", self.tool.name(), endpoint.method, url);

        // Build request
        let mut request = match endpoint.method {
            HttpMethod::GET => self.client.get(&url),
            HttpMethod::POST => self.client.post(&url),
            HttpMethod::PUT => self.client.put(&url),
            HttpMethod::DELETE => self.client.delete(&url),
            HttpMethod::PATCH => self.client.patch(&url),
        };

        // Add headers with substitutions
        for (key, value) in &endpoint.headers {
            let substituted_value = self.substitute_placeholders(value, &string_params);
            request = request.header(key, substituted_value);
        }

        // Add body if present and method supports it
        if let Some(body_template) = &endpoint.body_template {
            let body = self.substitute_json(body_template, &string_params);
            debug!("[{}] Request body: {}", self.tool.name(), body);
            request = request.json(&body);
        }

        // Execute request
        let response = request.send().await.map_err(|e| {
            error!("[{}] HTTP request failed: {}", self.tool.name(), e);
            format!("HTTP request failed: {}", e)
        })?;

        let status = response.status();
        info!("[{}] Response status: {}", self.tool.name(), status);

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            error!("[{}] HTTP error {}: {}", self.tool.name(), status, error_body);
            return Err(format!("HTTP error {}: {}", status, error_body));
        }

        // Parse response as JSON
        let response_text = response.text().await.map_err(|e| {
            error!("[{}] Failed to read response body: {}", self.tool.name(), e);
            format!("Failed to read response: {}", e)
        })?;

        debug!("[{}] Raw response: {}", self.tool.name(), &response_text[..response_text.len().min(500)]);

        // Try to parse as JSON for extraction
        let response_value: Value = serde_json::from_str(&response_text).unwrap_or_else(|_| {
            // If not JSON, wrap as string
            Value::String(response_text)
        });

        // Extract data using path if configured
        let extracted = if let Some(ref path) = self.tool.config.response_mapping.extract_path {
            self.extract_path(&response_value, path)
                .unwrap_or(response_value)
        } else {
            response_value
        };

        // Format the response
        Ok(self.format_response(&extracted))
    }

    /// Get or create a cached MCP client connection.
    /// Returns the lock guard so the caller can use the client.
    async fn get_or_create_mcp_client(&self, url: &str) -> Result<(), String> {
        let mut guard = self.mcp_client.lock().await;

        // Check if existing client is still alive
        if let Some(ref client) = *guard {
            if !client.is_transport_closed() {
                debug!("[{}] Reusing cached MCP client", self.tool.name());
                return Ok(());
            }
            info!("[{}] MCP client disconnected, reconnecting...", self.tool.name());
            // Drop the stale client
            *guard = None;
        }

        info!("[{}] Connecting to MCP server: {}", self.tool.name(), url);

        let client_info = ClientInfo {
            meta: None,
            protocol_version: ProtocolVersion::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "mas-tool-handler".to_string(),
                title: None,
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
        };

        let transport = StreamableHttpClientTransport::from_uri(url);
        let client = client_info.serve(transport).await.map_err(|e| {
            error!("[{}] Failed to initialize MCP client: {}", self.tool.name(), e);
            format!("Failed to initialize MCP client: {}", e)
        })?;

        info!("[{}] MCP client connected", self.tool.name());
        *guard = Some(client);
        Ok(())
    }

    /// Execute an MCP (Model Context Protocol) request using a cached client
    ///
    /// The MCP client is created once and reused across calls. If the connection
    /// is lost, it reconnects automatically.
    async fn execute_mcp_request(&self, params: &HashMap<String, Value>) -> Result<String, String> {
        let endpoint = &self.tool.config.endpoint;
        let string_params = self.params_to_strings(params);
        let url = self.substitute_placeholders(&endpoint.url, &string_params);

        let mcp_tool_name = endpoint.mcp_tool_name.as_ref().ok_or_else(|| {
            "MCP endpoint requires mcp_tool_name to be set".to_string()
        })?;

        info!("[{}] MCP call: {} (tool: {})", self.tool.name(), url, mcp_tool_name);

        // Ensure we have a connected MCP client
        self.get_or_create_mcp_client(&url).await?;

        // Build the tool arguments from typed params (preserves JSON types)
        let arguments: Option<serde_json::Map<String, Value>> = Some(
            params.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        );

        debug!(
            "[{}] Calling MCP tool '{}' with args: {}",
            self.tool.name(),
            mcp_tool_name,
            serde_json::to_string(&arguments).unwrap_or_default()
        );

        // Call the tool using the cached client
        let tool_result = {
            let guard = self.mcp_client.lock().await;
            let client = guard.as_ref().ok_or_else(|| {
                "MCP client not available".to_string()
            })?;

            client
                .call_tool(CallToolRequestParams {
                    meta: None,
                    name: Cow::Owned(mcp_tool_name.clone()),
                    arguments,
                    task: None,
                })
                .await
                .map_err(|e| {
                    error!("[{}] MCP tool call failed: {}", self.tool.name(), e);
                    format!("MCP tool call failed: {}", e)
                })?
        };

        info!("[{}] MCP tool call succeeded", self.tool.name());

        // Check for errors in the result
        if tool_result.is_error.unwrap_or(false) {
            let error_text = tool_result
                .content
                .iter()
                .filter_map(|c| match &**c {
                    rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Err(format!("MCP tool returned error: {}", error_text));
        }

        // Extract the text content from the result
        let result_text = tool_result
            .content
            .iter()
            .filter_map(|c| match &**c {
                rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Try to parse as JSON for potential path extraction
        if let Ok(result_value) = serde_json::from_str::<Value>(&result_text) {
            let extracted = if let Some(ref path) = self.tool.config.response_mapping.extract_path {
                self.extract_path(&result_value, path).unwrap_or(result_value)
            } else {
                result_value
            };
            Ok(self.format_response(&extracted))
        } else {
            // If not JSON, return the text directly
            Ok(result_text)
        }
    }
}

#[async_trait]
impl MessageHandler for ToolHandler {
    async fn handle(&self, message: &Message, _agent: &Agent) -> Option<String> {
        info!(
            "[Tool:{}] Received message from {}: {}",
            self.tool.name(),
            message.from,
            &message.content[..message.content.len().min(100)]
        );

        // Parse parameters from message
        let params = self.parse_parameters(&message.content);
        debug!("[Tool:{}] Parsed parameters: {:?}", self.tool.name(), params);

        // Execute based on endpoint type
        let result = match self.tool.config.endpoint.endpoint_type {
            EndpointType::Http => self.execute_request(&params).await,
            EndpointType::Mcp => self.execute_mcp_request(&params).await,
        };

        match result {
            Ok(result) => {
                info!("[Tool:{}] Request succeeded", self.tool.name());
                Some(result)
            }
            Err(e) => {
                error!("[Tool:{}] Request failed: {}", self.tool.name(), e);
                Some(format!("Tool error: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{ToolConfig, ToolEndpoint, ResponseMapping};

    fn create_test_tool() -> Arc<Tool> {
        let endpoint = ToolEndpoint::post("https://api.example.com/search")
            .with_header("Authorization", "Bearer ${API_KEY}")
            .with_body(serde_json::json!({
                "query": "${query}",
                "limit": 10
            }));

        let config = ToolConfig::new("TestTool", "Test tool", endpoint)
            .with_parameters(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                }
            }));

        Arc::new(Tool::new(config))
    }

    #[test]
    fn test_parse_json_parameters() {
        let tool = create_test_tool();
        let handler = ToolHandler::new(tool);

        let params = handler.parse_parameters(r#"{"query": "test", "count": 5}"#);
        // Values are preserved with their original JSON types
        assert_eq!(params.get("query"), Some(&Value::String("test".to_string())));
        assert_eq!(params.get("count"), Some(&Value::Number(5.into())));
    }

    #[test]
    fn test_parse_json_preserves_types() {
        let tool = create_test_tool();
        let handler = ToolHandler::new(tool);

        let params = handler.parse_parameters(r#"{"name": "job", "active": true, "limit": 10}"#);
        assert_eq!(params.get("name"), Some(&Value::String("job".to_string())));
        assert_eq!(params.get("active"), Some(&Value::Bool(true)));
        assert_eq!(params.get("limit"), Some(&Value::Number(10.into())));
    }

    #[test]
    fn test_parse_plain_text_parameters() {
        let tool = create_test_tool();
        let handler = ToolHandler::new(tool);

        let params = handler.parse_parameters("search for rust");
        assert_eq!(params.get("input"), Some(&Value::String("search for rust".to_string())));
        assert_eq!(params.get("query"), Some(&Value::String("search for rust".to_string())));
    }

    #[test]
    fn test_params_to_strings() {
        let tool = create_test_tool();
        let handler = ToolHandler::new(tool);

        let mut params = HashMap::new();
        params.insert("query".to_string(), Value::String("test".to_string()));
        params.insert("count".to_string(), Value::Number(5.into()));
        params.insert("active".to_string(), Value::Bool(true));

        let strings = handler.params_to_strings(&params);
        assert_eq!(strings.get("query"), Some(&"test".to_string()));
        assert_eq!(strings.get("count"), Some(&"5".to_string()));
        assert_eq!(strings.get("active"), Some(&"true".to_string()));
    }

    #[test]
    fn test_substitute_placeholders() {
        let tool = create_test_tool();
        let handler = ToolHandler::new(tool);

        let mut params = HashMap::new();
        params.insert("query".to_string(), "rust programming".to_string());

        let result = handler.substitute_placeholders(
            "https://api.example.com/search?q=${query}&lang=en",
            &params,
        );
        assert_eq!(
            result,
            "https://api.example.com/search?q=rust programming&lang=en"
        );
    }

    #[test]
    fn test_substitute_json() {
        let tool = create_test_tool();
        let handler = ToolHandler::new(tool);

        let mut params = HashMap::new();
        params.insert("query".to_string(), "test".to_string());

        let template = serde_json::json!({
            "search": "${query}",
            "nested": {
                "value": "${query}"
            }
        });

        let result = handler.substitute_json(&template, &params);
        assert_eq!(result["search"], "test");
        assert_eq!(result["nested"]["value"], "test");
    }

    #[test]
    fn test_extract_simple_path() {
        let tool = create_test_tool();
        let handler = ToolHandler::new(tool);

        let response = serde_json::json!({
            "data": {
                "results": ["a", "b", "c"]
            }
        });

        let extracted = handler.extract_path(&response, "$.data.results").unwrap();
        assert_eq!(extracted, serde_json::json!(["a", "b", "c"]));
    }

    #[test]
    fn test_extract_array_index() {
        let tool = create_test_tool();
        let handler = ToolHandler::new(tool);

        let response = serde_json::json!({
            "items": [
                { "name": "first" },
                { "name": "second" }
            ]
        });

        let extracted = handler.extract_path(&response, "$.items[0].name").unwrap();
        assert_eq!(extracted, serde_json::json!("first"));
    }

    #[test]
    fn test_format_json() {
        let endpoint = ToolEndpoint::get("https://example.com");
        let config = ToolConfig::new("Test", "Test", endpoint)
            .with_response_mapping(ResponseMapping {
                extract_path: None,
                format: ResponseFormat::Json,
            });
        let handler = ToolHandler::new(Arc::new(Tool::new(config)));

        let value = serde_json::json!({"key": "value"});
        let formatted = handler.format_response(&value);
        assert!(formatted.contains("key"));
        assert!(formatted.contains("value"));
    }

    #[test]
    fn test_format_text() {
        let endpoint = ToolEndpoint::get("https://example.com");
        let config = ToolConfig::new("Test", "Test", endpoint)
            .with_response_mapping(ResponseMapping {
                extract_path: None,
                format: ResponseFormat::Text,
            });
        let handler = ToolHandler::new(Arc::new(Tool::new(config)));

        let value = serde_json::json!(["item1", "item2"]);
        let formatted = handler.format_response(&value);
        assert_eq!(formatted, "item1\nitem2");
    }

    #[test]
    fn test_format_markdown() {
        let endpoint = ToolEndpoint::get("https://example.com");
        let config = ToolConfig::new("Test", "Test", endpoint)
            .with_response_mapping(ResponseMapping {
                extract_path: None,
                format: ResponseFormat::Markdown,
            });
        let handler = ToolHandler::new(Arc::new(Tool::new(config)));

        let value = serde_json::json!({"title": "Hello", "count": 5});
        let formatted = handler.format_response(&value);
        assert!(formatted.contains("**title**"));
        assert!(formatted.contains("**count**"));
    }
}
