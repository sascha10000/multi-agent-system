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

        // Build agent list with roles for better routing decisions
        let mut agent_list = String::new();
        for (name, conn) in &blocking_connections {
            if let Some(ref role) = conn.target_role {
                agent_list.push_str(&format!("- {} ({})\n", name, role));
            } else {
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
                format!(
                    r#"
You can either:
1. Respond directly to the sender
2. Forward the request to the BEST matching agent based on their expertise
3. Both respond AND forward (acknowledge + delegate)

Available agents you can forward to (with their expertise):
{}
IMPORTANT: Choose the agent whose expertise BEST MATCHES the question topic.
Look at each agent's role description in parentheses to decide who should handle the request.

Respond with JSON in one of these formats:

Direct response (only if no agent matches the topic):
{{ "response": "your response here" }}

Forward to the best agent:
{{ "forward_to": [{{ "agent": "AgentName", "message": "the question to ask" }}] }}

Both respond and forward:
{{
  "response": "I'll check with our specialist.",
  "forward_to": [{{ "agent": "AgentName", "message": "the question to ask" }}]
}}

Only include valid JSON in your response."#,
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
                    // Parse JSON decision
                    let decision = parse_llm_response(&content);
                    debug!("[{}] Routing decision: {:?}", agent.name, decision);
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

    pub fn build(self) -> LlmHandler {
        LlmHandler {
            provider: self.provider,
            model: self.model,
            options: self.options,
            conversation_store: self.conversation_store,
            routing_enabled: self.routing_enabled,
            routing_behavior: self.routing_behavior,
        }
    }
}
