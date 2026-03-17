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
    /// Message content to send (defaults to empty string if LLM omits it)
    #[serde(default)]
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

/// A single turn in a multi-turn agent conversation
#[derive(Debug, Clone)]
pub struct ConversationTurn {
    /// The agent that was spoken to
    pub agent: String,
    /// The message sent to the agent
    pub message_sent: String,
    /// The response received from the agent
    pub response: String,
    /// Which turn number this was (0-indexed)
    pub turn_number: u16,
}

/// Decision from the evaluation step of a multi-turn conversation
#[derive(Debug, Clone, PartialEq)]
pub enum EvaluationDecision {
    /// The responses are sufficient; here is the final answer
    Satisfied { response: String },
    /// Need to ask follow-up questions to agents
    FollowUp { targets: Vec<ForwardTarget> },
}

/// JSON structure for parsing LLM evaluation responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationJson {
    /// Whether the agent is satisfied with the responses
    #[serde(default)]
    pub satisfied: bool,
    /// Final response (when satisfied)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    /// Follow-up messages (when not satisfied)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_up: Option<Vec<ForwardTarget>>,
}

/// Parse an LLM evaluation response into an EvaluationDecision
pub fn parse_evaluation_response(response: &str) -> EvaluationDecision {
    // Try to extract JSON from the response
    if let Some(json_str) = extract_json_from_evaluation(response) {
        if let Ok(eval) = serde_json::from_str::<EvaluationJson>(&json_str) {
            if eval.satisfied {
                return EvaluationDecision::Satisfied {
                    response: eval.response.unwrap_or_default(),
                };
            }
            if let Some(targets) = eval.follow_up {
                if !targets.is_empty() {
                    return EvaluationDecision::FollowUp { targets };
                }
            }
        }
    }

    // Fallback: treat as satisfied with the raw text as response
    EvaluationDecision::Satisfied {
        response: response.trim().to_string(),
    }
}

/// Extract a JSON object from an evaluation response (similar to extract_json_from_response)
fn extract_json_from_evaluation(response: &str) -> Option<String> {
    let response = response.trim();
    let mut search_start = 0;

    while let Some(start) = response[search_start..].find('{') {
        let start = search_start + start;
        let mut depth = 0;
        let mut end = None;

        for (i, c) in response[start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + i);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(end_pos) = end {
            let json_str = &response[start..=end_pos];
            if serde_json::from_str::<EvaluationJson>(json_str).is_ok() {
                return Some(json_str.to_string());
            }
            search_start = end_pos + 1;
        } else {
            break;
        }
    }

    None
}

/// Try to extract and merge JSON from LLM response that may contain extra text
///
/// LLMs sometimes wrap JSON in markdown code blocks, add explanatory text,
/// or output multiple JSON objects. This function attempts to extract and
/// merge all JSON objects into one.
pub fn extract_json_from_response(response: &str) -> Option<String> {
    let response = response.trim();

    // Try to find all JSON objects in the response
    let mut json_objects: Vec<LlmDecisionJson> = Vec::new();
    let mut search_start = 0;

    while let Some(start) = response[search_start..].find('{') {
        let start = search_start + start;
        // Find the matching closing brace by counting braces
        let mut depth = 0;
        let mut end = None;

        for (i, c) in response[start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + i);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(end_pos) = end {
            let json_str = &response[start..=end_pos];
            if let Ok(parsed) = serde_json::from_str::<LlmDecisionJson>(json_str) {
                json_objects.push(parsed);
            }
            search_start = end_pos + 1;
        } else {
            break;
        }
    }

    // If we found multiple JSON objects, merge them
    if json_objects.is_empty() {
        return None;
    }

    if json_objects.len() == 1 {
        // Single object - just return it as JSON string
        return serde_json::to_string(&json_objects.remove(0)).ok();
    }

    // Merge multiple objects
    let mut merged = LlmDecisionJson::default();
    for obj in json_objects {
        if obj.response.is_some() {
            merged.response = obj.response;
        }
        if let Some(forwards) = obj.forward_to {
            if let Some(ref mut existing) = merged.forward_to {
                existing.extend(forwards);
            } else {
                merged.forward_to = Some(forwards);
            }
        }
    }

    serde_json::to_string(&merged).ok()
}

/// Parse an LLM response into a HandlerDecision
///
/// This handles various edge cases:
/// - Pure JSON response
/// - JSON wrapped in markdown code blocks
/// - JSON with surrounding text
/// - Multiple JSON objects (merged together)
/// - Fallback to treating entire response as direct response
pub fn parse_llm_response(response: &str) -> HandlerDecision {
    // First, try to extract and parse JSON (handles multiple objects)
    if let Some(json_str) = extract_json_from_response(response) {
        if let Ok(decision_json) = LlmDecisionJson::parse(&json_str) {
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

    #[test]
    fn test_parse_multiple_json_objects() {
        // LLM sometimes returns two separate JSON objects instead of one combined
        let response = r#"{ "response": "I can help with that." }
{ "forward_to": [{ "agent": "Researcher", "message": "Look this up" }] }"#;
        let decision = parse_llm_response(response);
        assert_eq!(
            decision,
            HandlerDecision::ResponseAndForward {
                content: "I can help with that.".to_string(),
                targets: vec![ForwardTarget::new("Researcher", "Look this up")]
            }
        );
    }

    #[test]
    fn test_parse_multiple_json_objects_with_text() {
        let response = r#"Here's my decision:
{ "response": "General info here." }
And also forwarding:
{ "forward_to": [{ "agent": "Analyst", "message": "Analyze this" }] }"#;
        let decision = parse_llm_response(response);
        assert_eq!(
            decision,
            HandlerDecision::ResponseAndForward {
                content: "General info here.".to_string(),
                targets: vec![ForwardTarget::new("Analyst", "Analyze this")]
            }
        );
    }
}
