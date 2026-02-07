//! Tool configuration types for HTTP-based tools
//!
//! Tools are external HTTP endpoints that agents can call to perform actions
//! like web searches, API calls, or data retrieval. Unlike agents, tools don't
//! have LLM-based decision making - they simply execute HTTP requests and
//! return results.
//!
//! # Example Configuration
//!
//! ```json
//! {
//!   "name": "WebSearch",
//!   "description": "Search the web for information",
//!   "parameters": {
//!     "type": "object",
//!     "properties": {
//!       "query": { "type": "string", "description": "Search query" }
//!     },
//!     "required": ["query"]
//!   },
//!   "endpoint": {
//!     "url": "https://api.example.com/search",
//!     "method": "POST",
//!     "headers": { "Authorization": "Bearer ${API_KEY}" },
//!     "body_template": { "q": "${query}" }
//!   },
//!   "response_mapping": {
//!     "extract_path": "$.results",
//!     "format": "json"
//!   },
//!   "timeout_secs": 30
//! }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// HTTP method for tool endpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

/// Transport type for tool endpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EndpointType {
    /// Standard HTTP request-response
    #[default]
    Http,
    /// MCP (Model Context Protocol) with JSON-RPC and SSE
    Mcp,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
            HttpMethod::POST => write!(f, "POST"),
            HttpMethod::PUT => write!(f, "PUT"),
            HttpMethod::DELETE => write!(f, "DELETE"),
            HttpMethod::PATCH => write!(f, "PATCH"),
        }
    }
}

/// Response format for tool output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseFormat {
    /// Return raw JSON
    #[default]
    Json,
    /// Return as plain text
    Text,
    /// Return as markdown-formatted text
    Markdown,
}

/// Configuration for the HTTP endpoint a tool calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEndpoint {
    /// URL to call. Can contain ${param} placeholders for parameter substitution
    /// and ${ENV_VAR} for environment variable substitution
    pub url: String,

    /// Transport type: "http" (default) or "mcp"
    #[serde(default, rename = "type")]
    pub endpoint_type: EndpointType,

    /// HTTP method to use (for http type)
    #[serde(default)]
    pub method: HttpMethod,

    /// HTTP headers. Values can contain ${ENV_VAR} for environment variable substitution
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request body template (for POST/PUT/PATCH with http type). Can contain ${param} placeholders
    #[serde(default)]
    pub body_template: Option<Value>,

    /// MCP tool name to call (required when endpoint_type is "mcp")
    /// This is the tool name registered on the MCP server
    #[serde(default)]
    pub mcp_tool_name: Option<String>,
}

/// Configuration for how to process the HTTP response
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponseMapping {
    /// JSONPath expression to extract data from the response (e.g., "$.data.results")
    #[serde(default)]
    pub extract_path: Option<String>,

    /// Format for the output
    #[serde(default)]
    pub format: ResponseFormat,
}

/// Complete configuration for a tool
///
/// A tool represents an HTTP endpoint that agents can call. Tools are defined
/// inline in the system configuration and appear as nodes in the visual editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Unique name for the tool (used for routing)
    pub name: String,

    /// Human-readable description of what the tool does
    pub description: String,

    /// JSON Schema describing the tool's parameters
    /// This is shown to the LLM so it knows how to call the tool
    #[serde(default = "default_parameters")]
    pub parameters: Value,

    /// HTTP endpoint configuration
    pub endpoint: ToolEndpoint,

    /// How to process the HTTP response
    #[serde(default)]
    pub response_mapping: ResponseMapping,

    /// Request timeout in seconds
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

fn default_parameters() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    })
}

impl ToolConfig {
    /// Create a new tool configuration
    pub fn new(name: impl Into<String>, description: impl Into<String>, endpoint: ToolEndpoint) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: default_parameters(),
            endpoint,
            response_mapping: ResponseMapping::default(),
            timeout_secs: None,
        }
    }

    /// Set the parameters schema
    pub fn with_parameters(mut self, parameters: Value) -> Self {
        self.parameters = parameters;
        self
    }

    /// Set response mapping
    pub fn with_response_mapping(mut self, mapping: ResponseMapping) -> Self {
        self.response_mapping = mapping;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = Some(timeout_secs);
        self
    }

    /// Get the effective timeout duration
    pub fn effective_timeout(&self, default: std::time::Duration) -> std::time::Duration {
        self.timeout_secs
            .map(std::time::Duration::from_secs)
            .unwrap_or(default)
    }
}

impl ToolEndpoint {
    /// Create a new GET endpoint
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            endpoint_type: EndpointType::Http,
            method: HttpMethod::GET,
            headers: HashMap::new(),
            body_template: None,
            mcp_tool_name: None,
        }
    }

    /// Create a new POST endpoint
    pub fn post(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            endpoint_type: EndpointType::Http,
            method: HttpMethod::POST,
            headers: HashMap::new(),
            body_template: None,
            mcp_tool_name: None,
        }
    }

    /// Create a new MCP endpoint
    pub fn mcp(url: impl Into<String>, tool_name: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            endpoint_type: EndpointType::Mcp,
            method: HttpMethod::POST,
            headers: HashMap::new(),
            body_template: None,
            mcp_tool_name: Some(tool_name.into()),
        }
    }

    /// Add a header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the body template
    pub fn with_body(mut self, body: Value) -> Self {
        self.body_template = Some(body);
        self
    }
}

/// Runtime representation of a tool (config + any runtime state)
#[derive(Debug, Clone)]
pub struct Tool {
    pub config: ToolConfig,
}

impl Tool {
    pub fn new(config: ToolConfig) -> Self {
        Self { config }
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn description(&self) -> &str {
        &self.config.description
    }
}

impl From<ToolConfig> for Tool {
    fn from(config: ToolConfig) -> Self {
        Self::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_config() {
        let json = r#"{
            "name": "WebSearch",
            "description": "Search the web",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                },
                "required": ["query"]
            },
            "endpoint": {
                "url": "https://api.example.com/search",
                "method": "POST",
                "headers": {
                    "Authorization": "Bearer ${API_KEY}"
                },
                "body_template": {
                    "q": "${query}"
                }
            },
            "response_mapping": {
                "extract_path": "$.results",
                "format": "json"
            },
            "timeout_secs": 30
        }"#;

        let config: ToolConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "WebSearch");
        assert_eq!(config.description, "Search the web");
        assert_eq!(config.endpoint.method, HttpMethod::POST);
        assert_eq!(config.endpoint.url, "https://api.example.com/search");
        assert_eq!(
            config.endpoint.headers.get("Authorization"),
            Some(&"Bearer ${API_KEY}".to_string())
        );
        assert_eq!(
            config.response_mapping.extract_path,
            Some("$.results".to_string())
        );
        assert_eq!(config.response_mapping.format, ResponseFormat::Json);
        assert_eq!(config.timeout_secs, Some(30));
    }

    #[test]
    fn test_parse_minimal_tool_config() {
        let json = r#"{
            "name": "Echo",
            "description": "Echo endpoint",
            "endpoint": {
                "url": "https://httpbin.org/get"
            }
        }"#;

        let config: ToolConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "Echo");
        assert_eq!(config.endpoint.method, HttpMethod::GET);
        assert!(config.endpoint.headers.is_empty());
        assert!(config.endpoint.body_template.is_none());
        assert_eq!(config.response_mapping.format, ResponseFormat::Json);
        assert!(config.timeout_secs.is_none());
    }

    #[test]
    fn test_http_method_display() {
        assert_eq!(HttpMethod::GET.to_string(), "GET");
        assert_eq!(HttpMethod::POST.to_string(), "POST");
        assert_eq!(HttpMethod::PUT.to_string(), "PUT");
        assert_eq!(HttpMethod::DELETE.to_string(), "DELETE");
        assert_eq!(HttpMethod::PATCH.to_string(), "PATCH");
    }

    #[test]
    fn test_tool_builder() {
        let endpoint = ToolEndpoint::post("https://api.example.com/search")
            .with_header("Content-Type", "application/json")
            .with_body(serde_json::json!({ "q": "${query}" }));

        let tool = ToolConfig::new("Search", "Search API", endpoint)
            .with_parameters(serde_json::json!({
                "type": "object",
                "properties": { "query": { "type": "string" } }
            }))
            .with_timeout(30);

        assert_eq!(tool.name, "Search");
        assert_eq!(tool.timeout_secs, Some(30));
    }
}
