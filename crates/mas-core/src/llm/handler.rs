use super::provider::{CompletionOptions, LlmMessage, LlmProvider, Role};
use crate::agent::Agent;
use crate::agent_system::{MessageHandler, RoutingHandler};
use crate::connection::ConnectionType;
use crate::conversation::ConversationStore;
use crate::decision::{parse_llm_response, HandlerDecision};
use crate::message::Message;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Defines how a routing agent should delegate to its connected agents
///
/// This determines the instructions injected into the LLM prompt about
/// when and how to forward messages to connected agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingBehavior {
    /// Forward to the single most appropriate agent based on the query
    /// This is the default behavior - LLM picks the best match
    #[default]
    Best,
    /// MUST forward to ALL connected agents and synthesize their responses
    /// Use this for panels, committees, or when diverse perspectives are needed
    All,
    /// Try to answer directly first, only forward if the agent lacks expertise
    /// Use this for agents that should handle most queries themselves
    DirectFirst,
}

/// A message handler that uses an LLM provider to generate responses
///
/// This handler supports two modes:
/// 1. Simple mode (MessageHandler) - LLM generates direct responses
/// 2. Routing mode (RoutingHandler) - LLM decides to respond, forward, or both
///
/// In routing mode, the handler:
/// 1. Builds an enhanced prompt with available connections
/// 2. Instructs the LLM to return JSON routing decisions
/// 3. Parses the response into HandlerDecision
/// 4. Can synthesize multiple forwarded responses
pub struct LlmHandler {
    provider: Arc<dyn LlmProvider>,
    model: Option<String>,
    options: Option<CompletionOptions>,
    /// Optional shared conversation store for context
    conversation_store: Option<Arc<RwLock<ConversationStore>>>,
    /// Whether to enable routing mode (JSON output, connection info in prompt)
    routing_enabled: bool,
    /// How the agent should delegate to connected agents
    routing_behavior: RoutingBehavior,
    /// Descriptions for connected tools (name -> description)
    /// Used to enrich the routing prompt with tool capabilities
    tool_descriptions: std::collections::HashMap<String, String>,
}

impl LlmHandler {
    /// Create a new LLM handler with the given provider
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider,
            model: None,
            options: None,
            conversation_store: None,
            routing_enabled: false,
            routing_behavior: RoutingBehavior::default(),
            tool_descriptions: std::collections::HashMap::new(),
        }
    }

    /// Set a specific model to use (overrides provider default)
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set completion options
    pub fn with_options(mut self, options: CompletionOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Set a shared conversation store for context
    pub fn with_conversation_store(mut self, store: Arc<RwLock<ConversationStore>>) -> Self {
        self.conversation_store = Some(store);
        self
    }

    /// Enable routing mode for dynamic LLM-based routing
    ///
    /// When enabled, the handler will:
    /// - Include connection info in the system prompt
    /// - Instruct the LLM to return JSON routing decisions
    /// - Parse responses as HandlerDecision
    pub fn with_routing(mut self) -> Self {
        self.routing_enabled = true;
        self
    }

    /// Set the routing behavior for this handler
    ///
    /// Controls how the LLM should delegate to connected agents:
    /// - `Best` (default): Forward to the single most appropriate agent
    /// - `All`: MUST forward to ALL connected agents and synthesize responses
    /// - `DirectFirst`: Try to answer directly, only forward if lacking expertise
    pub fn with_routing_behavior(mut self, behavior: RoutingBehavior) -> Self {
        self.routing_behavior = behavior;
        self
    }

    /// Set descriptions for tools connected to this agent
    ///
    /// These descriptions are included in the routing prompt so the LLM knows
    /// what each tool does and can make informed routing decisions.
    pub fn with_tool_descriptions(mut self, descriptions: std::collections::HashMap<String, String>) -> Self {
        self.tool_descriptions = descriptions;
        self
    }

    /// Build the routing instructions to append to the system prompt
    fn build_routing_instructions(&self, agent: &Agent) -> String {
        // Collect blocking connections (these are the ones LLM can forward to)
        let blocking_connections: Vec<(&String, &crate::connection::Connection)> = agent
            .connections
            .iter()
            .filter(|(_, conn)| conn.connection_type == ConnectionType::Blocking)
            .collect();

        if blocking_connections.is_empty() {
            // No connections to forward to - simpler instructions
            return r#"

You must respond with JSON in this exact format:
{ "response": "your response here" }

Do not include any text outside the JSON object."#
                .to_string();
        }

        // Build agent/tool list for routing decisions
        // Include descriptions for tools so LLM knows what they do
        let mut agent_list = String::new();
        for (name, _conn) in &blocking_connections {
            if let Some(description) = self.tool_descriptions.get(*name) {
                // This is a tool - include its description
                agent_list.push_str(&format!("- {} (Tool): {}\n", name, description));
            } else {
                // This is an agent - just show the name
                agent_list.push_str(&format!("- {}\n", name));
            }
        }

        // Build behavior-specific instructions
        let behavior_instructions = match self.routing_behavior {
            RoutingBehavior::All => {
                // Build the list of all agents for the forward_to example
                let all_agents_example: String = blocking_connections
                    .iter()
                    .map(|(name, _)| format!("{{ \"agent\": \"{}\", \"message\": \"<relevant question for this agent>\" }}", name))
                    .collect::<Vec<_>>()
                    .join(", ");

                format!(
                    r#"
IMPORTANT: You MUST forward to ALL connected agents for EVERY request.
Your role is to gather perspectives from all agents and synthesize their responses.
Never answer directly without consulting all agents first.

Available agents you MUST forward to:
{}
You MUST respond with JSON in this format - include ALL agents:

{{ "forward_to": [{}] }}

Tailor the message to each agent's expertise. Only include valid JSON in your response."#,
                    agent_list, all_agents_example
                )
            }
            RoutingBehavior::DirectFirst => {
                format!(
                    r#"
You should try to answer questions directly using your own knowledge.
Only forward to another agent if the question is outside your expertise.

Available agents you can forward to if needed:
{}
Respond with JSON in one of these formats:

Direct response (preferred when you can answer):
{{ "response": "your response here" }}

Forward to agent (only if you lack expertise):
{{ "forward_to": [{{ "agent": "AgentName", "message": "what to ask them" }}] }}

Prefer answering directly. Only include valid JSON in your response."#,
                    agent_list
                )
            }
            RoutingBehavior::Best => {
                // Check if we have any tools
                let has_tools = !self.tool_descriptions.is_empty();
                let tool_instructions = if has_tools {
                    // Build concrete examples for each tool so the LLM knows the exact parameter format
                    let mut examples = String::new();
                    for (name, _desc) in &self.tool_descriptions {
                        examples.push_str(&format!(
                            "\nTo use {name}: {{ \"forward_to\": [{{ \"agent\": \"{name}\", \"message\": \"{{\\\"query\\\": \\\"search terms\\\"}}\" }}] }}",
                        ));
                    }
                    format!(
                        r#"

TOOL USAGE: When forwarding to a Tool, the message MUST be a JSON string with the tool's parameters.{examples}"#
                    )
                } else {
                    String::new()
                };

                format!(
                    r#"

YOU ARE A ROUTER. You MUST output ONLY a single JSON object. No explanations, no text, no markdown — ONLY JSON.

If an available tool or agent matches the user's request, you MUST forward to it. Only respond directly if NOTHING matches.

Available agents/tools you can forward to:
{}
{tool_instructions}

JSON formats — pick ONE:

Forward to tool/agent:
{{ "forward_to": [{{ "agent": "NAME", "message": "the question or JSON params" }}] }}

Direct response (ONLY if nothing above matches):
{{ "response": "your answer" }}

RULES:
- Output ONLY valid JSON. No other text.
- If the user's request relates to any available tool, you MUST forward to it.
- Never answer a question yourself if a tool can answer it."#,
                    agent_list
                )
            }
        };

        behavior_instructions
    }

    /// Build LLM messages for simple mode (no routing)
    async fn build_messages(&self, message: &Message, agent: &Agent) -> Vec<LlmMessage> {
        let mut messages = Vec::new();

        // System prompt from agent
        if !agent.system_prompt.is_empty() {
            messages.push(LlmMessage::system(&agent.system_prompt));
        }

        // Add conversation history if available
        if let Some(store) = &self.conversation_store {
            let store = store.read().await;
            if let Some(conversation) = store.get(&message.from, &message.to) {
                for msg in conversation.messages() {
                    // Skip the current message (it's added below)
                    if msg.id == message.id {
                        continue;
                    }

                    let role = if msg.from == agent.name {
                        Role::Assistant
                    } else {
                        Role::User
                    };

                    messages.push(LlmMessage {
                        role,
                        content: msg.content.clone(),
                    });
                }
            }
        }

        // Add the current incoming message
        messages.push(LlmMessage::user(&message.content));

        messages
    }

    /// Build LLM messages for routing mode (with JSON instructions)
    async fn build_routing_messages(&self, message: &Message, agent: &Agent) -> Vec<LlmMessage> {
        let mut messages = Vec::new();

        // Enhanced system prompt with routing instructions
        let mut system_prompt = agent.system_prompt.clone();
        system_prompt.push_str(&self.build_routing_instructions(agent));
        messages.push(LlmMessage::system(&system_prompt));

        // Add conversation history if available
        if let Some(store) = &self.conversation_store {
            let store = store.read().await;
            if let Some(conversation) = store.get(&message.from, &message.to) {
                for msg in conversation.messages() {
                    if msg.id == message.id {
                        continue;
                    }

                    let role = if msg.from == agent.name {
                        Role::Assistant
                    } else {
                        Role::User
                    };

                    messages.push(LlmMessage {
                        role,
                        content: msg.content.clone(),
                    });
                }
            }
        }

        // Add the current incoming message
        messages.push(LlmMessage::user(&message.content));

        messages
    }

    /// Build LLM messages for synthesis (combining forwarded responses)
    fn build_synthesis_messages(
        &self,
        original_message: &Message,
        forwarded_responses: &[(String, String)],
        agent: &Agent,
    ) -> Vec<LlmMessage> {
        let mut messages = Vec::new();

        // System prompt for synthesis
        let synthesis_prompt = format!(
            "{}\n\nYou received responses from other agents. Synthesize them into a coherent response for the original sender. Be concise and helpful.",
            agent.system_prompt
        );
        messages.push(LlmMessage::system(&synthesis_prompt));

        // Original request
        messages.push(LlmMessage::user(&format!(
            "Original request from {}: {}",
            original_message.from, original_message.content
        )));

        // Forwarded responses
        let mut responses_text = String::from("Responses received:\n");
        for (agent_name, response) in forwarded_responses {
            responses_text.push_str(&format!("\n[{}]: {}\n", agent_name, response));
        }
        messages.push(LlmMessage::user(&responses_text));

        // Ask for synthesis
        messages.push(LlmMessage::user(
            "Please synthesize these responses into a single, coherent answer.",
        ));

        messages
    }

    /// Enforce "all" routing behavior by ensuring all blocking connections are forwarded to
    ///
    /// When routing_behavior is All, we must forward to every blocking connection regardless
    /// of what the LLM decided. This handles cases where smaller LLMs don't follow instructions.
    fn enforce_all_routing(
        &self,
        decision: HandlerDecision,
        message: &Message,
        agent: &Agent,
    ) -> HandlerDecision {
        use crate::decision::ForwardTarget;

        // Get all blocking connection names
        let blocking_agents: Vec<String> = agent
            .connections
            .iter()
            .filter(|(_, conn)| conn.connection_type == ConnectionType::Blocking)
            .map(|(name, _)| name.clone())
            .collect();

        if blocking_agents.is_empty() {
            // No blocking connections - nothing to enforce
            return decision;
        }

        // Build the complete set of forward targets for all blocking connections
        let build_all_targets = |base_message: &str| -> Vec<ForwardTarget> {
            blocking_agents
                .iter()
                .map(|name| ForwardTarget::new(name.clone(), base_message.to_string()))
                .collect()
        };

        match decision {
            HandlerDecision::Response { content: _ } => {
                // LLM tried to respond directly when it should forward to ALL
                // Convert to Forward with the original message for all agents
                info!(
                    "[{}] 'all' routing: LLM responded directly, converting to forward all",
                    agent.name
                );
                HandlerDecision::Forward {
                    targets: build_all_targets(&message.content),
                }
            }
            HandlerDecision::Forward { targets } => {
                // Check if all blocking agents are covered
                let covered: std::collections::HashSet<_> =
                    targets.iter().map(|t| t.agent.as_str()).collect();
                let all_covered = blocking_agents.iter().all(|name| covered.contains(name.as_str()));

                if all_covered {
                    // All agents are covered - keep the LLM's tailored messages
                    HandlerDecision::Forward { targets }
                } else {
                    // Missing some agents - rebuild with all targets using original message
                    info!(
                        "[{}] 'all' routing: LLM only forwarded to {:?}, enforcing all {:?}",
                        agent.name,
                        covered.iter().collect::<Vec<_>>(),
                        blocking_agents
                    );
                    HandlerDecision::Forward {
                        targets: build_all_targets(&message.content),
                    }
                }
            }
            HandlerDecision::ResponseAndForward { content, targets } => {
                // Check if all blocking agents are covered
                let covered: std::collections::HashSet<_> =
                    targets.iter().map(|t| t.agent.as_str()).collect();
                let all_covered = blocking_agents.iter().all(|name| covered.contains(name.as_str()));

                if all_covered {
                    // All agents covered - keep LLM's version
                    HandlerDecision::ResponseAndForward { content, targets }
                } else {
                    // Missing some agents - rebuild forward list
                    info!(
                        "[{}] 'all' routing: LLM only forwarded to {:?}, enforcing all {:?}",
                        agent.name,
                        covered.iter().collect::<Vec<_>>(),
                        blocking_agents
                    );
                    // Keep the response part, but fix the forward targets
                    HandlerDecision::ResponseAndForward {
                        content,
                        targets: build_all_targets(&message.content),
                    }
                }
            }
            HandlerDecision::None => {
                // LLM returned nothing - forward to all agents
                info!(
                    "[{}] 'all' routing: LLM returned no decision, forwarding to all",
                    agent.name
                );
                HandlerDecision::Forward {
                    targets: build_all_targets(&message.content),
                }
            }
        }
    }

    /// Call the LLM provider
    async fn call_llm(&self, messages: &[LlmMessage]) -> Result<String, String> {
        let model = self.model.as_deref();
        let options = self.options.clone();

        match self.provider.complete(messages, model, options).await {
            Ok(response) => {
                info!(
                    "LLM response ({} tokens)",
                    response.usage.map(|u| u.total_tokens).unwrap_or(0)
                );
                Ok(response.content)
            }
            Err(e) => {
                error!("LLM error: {}", e);
                Err(format!("Error generating response: {}", e))
            }
        }
    }
}

#[async_trait]
impl MessageHandler for LlmHandler {
    async fn handle(&self, message: &Message, agent: &Agent) -> Option<String> {
        debug!(
            "[{}] Processing message from {}: {}",
            agent.name, message.from, message.content
        );

        let messages = self.build_messages(message, agent).await;

        match self.call_llm(&messages).await {
            Ok(content) => Some(content),
            Err(e) => Some(e),
        }
    }
}

#[async_trait]
impl RoutingHandler for LlmHandler {
    async fn handle(&self, message: &Message, agent: &Agent) -> HandlerDecision {
        debug!(
            "[{}] Processing message with routing from {}: {}",
            agent.name, message.from, message.content
        );

        let messages = if self.routing_enabled {
            self.build_routing_messages(message, agent).await
        } else {
            self.build_messages(message, agent).await
        };

        match self.call_llm(&messages).await {
            Ok(content) => {
                if self.routing_enabled {
                    // Log raw LLM output so we can debug routing issues
                    info!("[{}] Raw LLM response: {}", agent.name, content);
                    // Parse JSON decision
                    let mut decision = parse_llm_response(&content);
                    info!("[{}] Routing decision: {:?}", agent.name, decision);

                    // Enforce "all" routing behavior at the system level
                    // If routing_behavior is All, we MUST forward to ALL blocking connections
                    // regardless of what the LLM decided
                    if self.routing_behavior == RoutingBehavior::All {
                        decision = self.enforce_all_routing(decision, message, agent);
                        debug!("[{}] Routing decision (after 'all' enforcement): {:?}", agent.name, decision);
                    }

                    decision
                } else {
                    // Simple mode - just return as response
                    HandlerDecision::response(content)
                }
            }
            Err(e) => {
                warn!("[{}] LLM error, returning error response: {}", agent.name, e);
                HandlerDecision::response(e)
            }
        }
    }

    async fn synthesize(
        &self,
        original_message: &Message,
        forwarded_responses: &[(String, String)],
        agent: &Agent,
    ) -> Option<String> {
        if forwarded_responses.is_empty() {
            return None;
        }

        // Single response: pass through directly (no synthesis needed)
        // Whether from a tool or agent, a single response is already complete
        if forwarded_responses.len() == 1 {
            let (responder_name, response) = &forwarded_responses[0];

            // Try to unwrap JSON response format if present
            let unwrapped = unwrap_json_response(response);

            debug!(
                "[{}] Single response pass-through from {} (skipping synthesis)",
                agent.name, responder_name
            );
            return Some(unwrapped);
        }

        // Multiple responses: synthesize to combine them
        debug!(
            "[{}] Synthesizing {} forwarded responses",
            agent.name,
            forwarded_responses.len()
        );

        let messages = self.build_synthesis_messages(original_message, forwarded_responses, agent);

        match self.call_llm(&messages).await {
            Ok(content) => {
                // For synthesis, we want plain text, not JSON
                // So we just return the content directly
                Some(content)
            }
            Err(e) => {
                error!("[{}] Synthesis failed: {}", agent.name, e);
                // Fallback: concatenate responses
                let fallback = forwarded_responses
                    .iter()
                    .map(|(name, resp)| format!("[{}]: {}", name, resp))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                Some(fallback)
            }
        }
    }
}

/// Builder for creating LLM handlers with fluent API
pub struct LlmHandlerBuilder {
    provider: Arc<dyn LlmProvider>,
    model: Option<String>,
    options: Option<CompletionOptions>,
    conversation_store: Option<Arc<RwLock<ConversationStore>>>,
    routing_enabled: bool,
    routing_behavior: RoutingBehavior,
    tool_descriptions: std::collections::HashMap<String, String>,
}

impl LlmHandlerBuilder {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider,
            model: None,
            options: None,
            conversation_store: None,
            routing_enabled: false,
            routing_behavior: RoutingBehavior::default(),
            tool_descriptions: std::collections::HashMap::new(),
        }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        let options = self.options.get_or_insert_with(CompletionOptions::new);
        options.temperature = Some(temp);
        self
    }

    pub fn max_tokens(mut self, max: u32) -> Self {
        let options = self.options.get_or_insert_with(CompletionOptions::new);
        options.max_tokens = Some(max);
        self
    }

    pub fn conversation_store(mut self, store: Arc<RwLock<ConversationStore>>) -> Self {
        self.conversation_store = Some(store);
        self
    }

    /// Enable routing mode for dynamic LLM-based routing
    pub fn routing(mut self) -> Self {
        self.routing_enabled = true;
        self
    }

    /// Set the routing behavior
    pub fn routing_behavior(mut self, behavior: RoutingBehavior) -> Self {
        self.routing_behavior = behavior;
        self
    }

    /// Set descriptions for connected tools
    pub fn tool_descriptions(mut self, descriptions: std::collections::HashMap<String, String>) -> Self {
        self.tool_descriptions = descriptions;
        self
    }

    pub fn build(self) -> LlmHandler {
        LlmHandler {
            provider: self.provider,
            model: self.model,
            options: self.options,
            conversation_store: self.conversation_store,
            routing_enabled: self.routing_enabled,
            routing_behavior: self.routing_behavior,
            tool_descriptions: self.tool_descriptions,
        }
    }
}

/// Extract content from JSON response format if present
/// e.g., {"response": "hello"} → "hello"
/// If not JSON or no "response" field, returns original string
fn unwrap_json_response(response: &str) -> String {
    // Try to parse as JSON and extract "response" field
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
        if let Some(content) = json.get("response").and_then(|v| v.as_str()) {
            return content.to_string();
        }
    }
    response.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentBuilder;
    use crate::decision::ForwardTarget;
    use crate::llm::provider::{CompletionResponse, LlmError};

    // Mock provider for testing (doesn't need to work, just needs to exist)
    struct MockProvider;

    #[async_trait]
    impl LlmProvider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        fn default_model(&self) -> &str {
            "mock-model"
        }

        async fn complete(
            &self,
            _messages: &[LlmMessage],
            _model: Option<&str>,
            _options: Option<CompletionOptions>,
        ) -> Result<CompletionResponse, LlmError> {
            Err(LlmError::ProviderError("Mock provider".to_string()))
        }

        async fn health_check(&self) -> Result<(), LlmError> {
            Ok(())
        }
    }

    fn create_test_handler_with_all_routing() -> LlmHandler {
        LlmHandler::new(Arc::new(MockProvider))
            .with_routing()
            .with_routing_behavior(RoutingBehavior::All)
    }

    fn create_test_agent_with_connections() -> Agent {
        AgentBuilder::new("Moderator")
            .system_prompt("You moderate discussions.")
            .blocking_connection("TechExpert")
            .blocking_connection("BusinessExpert")
            .notify_connection("Logger")
            .build()
    }

    fn create_test_message() -> Message {
        Message::new("user", "Moderator", "What should we do?")
    }

    #[test]
    fn test_enforce_all_routing_converts_response_to_forward() {
        let handler = create_test_handler_with_all_routing();
        let agent = create_test_agent_with_connections();
        let message = create_test_message();

        // LLM returned a direct response when it should forward to all
        let decision = HandlerDecision::Response {
            content: "I think we should...".to_string(),
        };

        let enforced = handler.enforce_all_routing(decision, &message, &agent);

        // Should be converted to Forward with both blocking connections
        match enforced {
            HandlerDecision::Forward { targets } => {
                assert_eq!(targets.len(), 2);
                let agent_names: std::collections::HashSet<_> =
                    targets.iter().map(|t| t.agent.as_str()).collect();
                assert!(agent_names.contains("TechExpert"));
                assert!(agent_names.contains("BusinessExpert"));
            }
            _ => panic!("Expected Forward decision, got {:?}", enforced),
        }
    }

    #[test]
    fn test_enforce_all_routing_expands_partial_forward() {
        let handler = create_test_handler_with_all_routing();
        let agent = create_test_agent_with_connections();
        let message = create_test_message();

        // LLM only forwarded to one agent
        let decision = HandlerDecision::Forward {
            targets: vec![ForwardTarget::new("TechExpert", "What do you think?")],
        };

        let enforced = handler.enforce_all_routing(decision, &message, &agent);

        // Should be expanded to include both blocking connections
        match enforced {
            HandlerDecision::Forward { targets } => {
                assert_eq!(targets.len(), 2);
                let agent_names: std::collections::HashSet<_> =
                    targets.iter().map(|t| t.agent.as_str()).collect();
                assert!(agent_names.contains("TechExpert"));
                assert!(agent_names.contains("BusinessExpert"));
            }
            _ => panic!("Expected Forward decision, got {:?}", enforced),
        }
    }

    #[test]
    fn test_enforce_all_routing_keeps_complete_forward() {
        let handler = create_test_handler_with_all_routing();
        let agent = create_test_agent_with_connections();
        let message = create_test_message();

        // LLM correctly forwarded to all blocking connections
        let decision = HandlerDecision::Forward {
            targets: vec![
                ForwardTarget::new("TechExpert", "Technical perspective?"),
                ForwardTarget::new("BusinessExpert", "Business perspective?"),
            ],
        };

        let enforced = handler.enforce_all_routing(decision.clone(), &message, &agent);

        // Should keep the LLM's tailored messages since all agents are covered
        match enforced {
            HandlerDecision::Forward { targets } => {
                assert_eq!(targets.len(), 2);
                // Check that messages are preserved (LLM's tailored versions)
                let tech_target = targets.iter().find(|t| t.agent == "TechExpert").unwrap();
                assert_eq!(tech_target.message, "Technical perspective?");
            }
            _ => panic!("Expected Forward decision, got {:?}", enforced),
        }
    }

    #[test]
    fn test_enforce_all_routing_converts_none_to_forward() {
        let handler = create_test_handler_with_all_routing();
        let agent = create_test_agent_with_connections();
        let message = create_test_message();

        // LLM returned no decision
        let decision = HandlerDecision::None;

        let enforced = handler.enforce_all_routing(decision, &message, &agent);

        // Should be converted to Forward with all blocking connections
        match enforced {
            HandlerDecision::Forward { targets } => {
                assert_eq!(targets.len(), 2);
            }
            _ => panic!("Expected Forward decision, got {:?}", enforced),
        }
    }

    #[test]
    fn test_unwrap_json_response_extracts_content() {
        // JSON with response field should be unwrapped
        let json_response = r#"{"response": "hello world"}"#;
        assert_eq!(super::unwrap_json_response(json_response), "hello world");
    }

    #[test]
    fn test_unwrap_json_response_returns_plain_text() {
        // Plain text should pass through unchanged
        let plain_text = "hello world";
        assert_eq!(super::unwrap_json_response(plain_text), "hello world");
    }

    #[test]
    fn test_unwrap_json_response_handles_json_without_response_field() {
        // JSON without "response" field should return original
        let json_other = r#"{"data": "something else"}"#;
        assert_eq!(super::unwrap_json_response(json_other), json_other);
    }

    #[test]
    fn test_unwrap_json_response_handles_forward_to_json() {
        // JSON with forward_to should return original (no response field)
        let forward_json = r#"{"forward_to": [{"agent": "Worker", "message": "hello"}]}"#;
        assert_eq!(super::unwrap_json_response(forward_json), forward_json);
    }
}
