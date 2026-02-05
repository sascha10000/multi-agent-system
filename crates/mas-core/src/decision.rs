//! Decision types for LLM-based routing
//!
//! This module defines the structured response format that LLM handlers use
//! to indicate routing decisions - whether to respond directly, forward to
//! other agents, or both.

use serde::{Deserialize, Serialize};

/// A target agent for message forwarding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForwardTarget {
    /// Name of the agent to forward to
    pub agent: String,
    /// Message content to send
    pub message: String,
}

impl ForwardTarget {
    pub fn new(agent: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            agent: agent.into(),
            message: message.into(),
        }
    }
}

/// The decision made by a handler about how to process a message
///
/// This is the structured return type from routing-aware handlers,
/// allowing the LLM to decide whether to respond directly or
/// delegate work to other agents.
#[derive(Debug, Clone, PartialEq)]
pub enum HandlerDecision {
    /// Respond directly to the sender
    Response { content: String },

    /// Forward to other agents (can be multiple, processed in parallel)
    Forward { targets: Vec<ForwardTarget> },

    /// Both respond AND forward (e.g., acknowledge + delegate)
    ResponseAndForward {
        content: String,
        targets: Vec<ForwardTarget>,
    },

    /// No action - handler chose not to respond or forward
    None,
}

impl HandlerDecision {
    /// Create a direct response decision
    pub fn response(content: impl Into<String>) -> Self {
        Self::Response {
            content: content.into(),
        }
    }

    /// Create a forward decision to a single agent
    pub fn forward_to(agent: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Forward {
            targets: vec![ForwardTarget::new(agent, message)],
        }
    }

    /// Create a forward decision to multiple agents
    pub fn forward_to_many(targets: Vec<ForwardTarget>) -> Self {
        Self::Forward { targets }
    }

    /// Create a response-and-forward decision
    pub fn respond_and_forward(
        content: impl Into<String>,
        targets: Vec<ForwardTarget>,
    ) -> Self {
        Self::ResponseAndForward {
            content: content.into(),
            targets,
        }
    }

    /// Check if this decision includes a response
    pub fn has_response(&self) -> bool {
        matches!(
            self,
            HandlerDecision::Response { .. } | HandlerDecision::ResponseAndForward { .. }
        )
    }

    /// Check if this decision includes forwarding
    pub fn has_forward(&self) -> bool {
        matches!(
            self,
            HandlerDecision::Forward { .. } | HandlerDecision::ResponseAndForward { .. }
        )
    }

    /// Get the response content if present
    pub fn response_content(&self) -> Option<&str> {
        match self {
            HandlerDecision::Response { content } => Some(content),
            HandlerDecision::ResponseAndForward { content, .. } => Some(content),
            _ => None,
        }
    }

    /// Get the forward targets if present
    pub fn forward_targets(&self) -> Option<&[ForwardTarget]> {
        match self {
            HandlerDecision::Forward { targets } => Some(targets),
            HandlerDecision::ResponseAndForward { targets, .. } => Some(targets),
            _ => None,
        }
    }
}

/// JSON structure for parsing LLM responses
///
/// The LLM returns JSON in one of these formats:
/// - `{ "response": "..." }` - direct response
/// - `{ "forward_to": [{ "agent": "...", "message": "..." }] }` - forward
/// - Both fields present - respond and forward
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmDecisionJson {
    /// Direct response content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,

    /// Agents to forward to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward_to: Option<Vec<ForwardTarget>>,
}

impl LlmDecisionJson {
    /// Parse JSON string into LlmDecisionJson
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Convert to HandlerDecision
    pub fn into_decision(self) -> HandlerDecision {
        match (self.response, self.forward_to) {
            (Some(content), Some(targets)) if !targets.is_empty() => {
                HandlerDecision::ResponseAndForward { content, targets }
            }
            (Some(content), _) => HandlerDecision::Response { content },
            (None, Some(targets)) if !targets.is_empty() => {
                HandlerDecision::Forward { targets }
            }
            _ => HandlerDecision::None,
        }
    }
}

impl From<LlmDecisionJson> for HandlerDecision {
    fn from(json: LlmDecisionJson) -> Self {
        json.into_decision()
    }
}

/// Try to extract JSON from LLM response that may contain extra text
///
/// LLMs sometimes wrap JSON in markdown code blocks or add explanatory text.
/// This function attempts to extract the JSON object.
pub fn extract_json_from_response(response: &str) -> Option<&str> {
    let response = response.trim();

    // Try to find JSON object boundaries
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            if end > start {
                return Some(&response[start..=end]);
            }
        }
    }

    None
}

/// Parse an LLM response into a HandlerDecision
///
/// This handles various edge cases:
/// - Pure JSON response
/// - JSON wrapped in markdown code blocks
/// - JSON with surrounding text
/// - Fallback to treating entire response as direct response
pub fn parse_llm_response(response: &str) -> HandlerDecision {
    // First, try to extract and parse JSON
    if let Some(json_str) = extract_json_from_response(response) {
        if let Ok(decision_json) = LlmDecisionJson::parse(json_str) {
            let decision = decision_json.into_decision();
            // Only return if we got a meaningful decision
            if !matches!(decision, HandlerDecision::None) {
                return decision;
            }
        }
    }

    // Fallback: treat entire response as direct response
    // (for backwards compatibility or when LLM doesn't follow JSON format)
    let content = response.trim();
    if content.is_empty() {
        HandlerDecision::None
    } else {
        HandlerDecision::Response {
            content: content.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_response_only() {
        let json = r#"{ "response": "Here is my answer" }"#;
        let decision = parse_llm_response(json);
        assert_eq!(
            decision,
            HandlerDecision::Response {
                content: "Here is my answer".to_string()
            }
        );
    }

    #[test]
    fn test_parse_forward_only() {
        let json = r#"{ "forward_to": [{ "agent": "Researcher", "message": "Look into this" }] }"#;
        let decision = parse_llm_response(json);
        assert_eq!(
            decision,
            HandlerDecision::Forward {
                targets: vec![ForwardTarget::new("Researcher", "Look into this")]
            }
        );
    }

    #[test]
    fn test_parse_response_and_forward() {
        let json = r#"{
            "response": "I'll get help on this.",
            "forward_to": [{ "agent": "Analyst", "message": "Analyze this data" }]
        }"#;
        let decision = parse_llm_response(json);
        assert_eq!(
            decision,
            HandlerDecision::ResponseAndForward {
                content: "I'll get help on this.".to_string(),
                targets: vec![ForwardTarget::new("Analyst", "Analyze this data")]
            }
        );
    }

    #[test]
    fn test_parse_multiple_forwards() {
        let json = r#"{
            "forward_to": [
                { "agent": "Researcher", "message": "Research this" },
                { "agent": "Analyst", "message": "Analyze that" }
            ]
        }"#;
        let decision = parse_llm_response(json);
        assert_eq!(
            decision,
            HandlerDecision::Forward {
                targets: vec![
                    ForwardTarget::new("Researcher", "Research this"),
                    ForwardTarget::new("Analyst", "Analyze that")
                ]
            }
        );
    }

    #[test]
    fn test_parse_json_with_markdown() {
        let response = r#"Here's my decision:
```json
{ "response": "The answer is 42" }
```"#;
        let decision = parse_llm_response(response);
        assert_eq!(
            decision,
            HandlerDecision::Response {
                content: "The answer is 42".to_string()
            }
        );
    }

    #[test]
    fn test_parse_plain_text_fallback() {
        let response = "This is just plain text without JSON";
        let decision = parse_llm_response(response);
        assert_eq!(
            decision,
            HandlerDecision::Response {
                content: "This is just plain text without JSON".to_string()
            }
        );
    }

    #[test]
    fn test_parse_empty_response() {
        let decision = parse_llm_response("");
        assert_eq!(decision, HandlerDecision::None);
    }

    #[test]
    fn test_decision_helpers() {
        let response = HandlerDecision::response("Hello");
        assert!(response.has_response());
        assert!(!response.has_forward());
        assert_eq!(response.response_content(), Some("Hello"));

        let forward = HandlerDecision::forward_to("Agent", "Message");
        assert!(!forward.has_response());
        assert!(forward.has_forward());
        assert_eq!(forward.forward_targets().unwrap().len(), 1);

        let both = HandlerDecision::respond_and_forward(
            "Ack",
            vec![ForwardTarget::new("X", "Do this")],
        );
        assert!(both.has_response());
        assert!(both.has_forward());
    }
}
