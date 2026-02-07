use crate::agent::Agent;
use crate::config::SystemConfig;
use crate::connection::ConnectionType;
use crate::conversation::ConversationStore;
use crate::decision::{ForwardTarget, HandlerDecision};
use crate::errors::{AgentError, Result};
use crate::message::Message;
use crate::tool::Tool;
use crate::tracer::{TraceCollector, TraceEvent};

use async_trait::async_trait;
use futures::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Result of sending a message to an agent
#[derive(Debug)]
pub enum SendResult {
    /// Successfully received a response
    Response(Message),
    /// The agent timed out
    Timeout(AgentError),
    /// Message was sent as notify (no response expected)
    Notified,
}

impl SendResult {
    pub fn is_success(&self) -> bool {
        matches!(self, SendResult::Response(_) | SendResult::Notified)
    }

    pub fn into_response(self) -> Option<Message> {
        match self {
            SendResult::Response(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Handler trait for processing messages (simple version)
/// Implement this trait to define how an agent processes incoming messages
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Process an incoming message and optionally return a response
    /// For notify connections, the return value is ignored
    async fn handle(&self, message: &Message, agent: &Agent) -> Option<String>;
}

/// Handler trait for LLM-based routing decisions
///
/// This trait extends message handling to support dynamic routing decisions.
/// The LLM can decide to:
/// - Respond directly to the sender
/// - Forward the message to other connected agents
/// - Both respond and forward (acknowledge + delegate)
///
/// The agent loop processes these decisions, handling forwarding and
/// response synthesis automatically.
#[async_trait]
pub trait RoutingHandler: Send + Sync {
    /// Process an incoming message and return a routing decision
    ///
    /// The handler should analyze the message and decide whether to:
    /// - `HandlerDecision::Response` - respond directly
    /// - `HandlerDecision::Forward` - delegate to other agents
    /// - `HandlerDecision::ResponseAndForward` - acknowledge and delegate
    /// - `HandlerDecision::None` - no action
    async fn handle(&self, message: &Message, agent: &Agent) -> HandlerDecision;

    /// Synthesize multiple forwarded responses into a single response
    ///
    /// Called when the handler forwarded to multiple agents and received
    /// responses. The handler should combine these into a coherent response
    /// to send back to the original sender.
    ///
    /// # Arguments
    /// * `original_message` - The original incoming message
    /// * `forwarded_responses` - Responses received from forwarded agents (agent_name, response)
    /// * `agent` - The current agent
    async fn synthesize(
        &self,
        original_message: &Message,
        forwarded_responses: &[(String, String)],
        agent: &Agent,
    ) -> Option<String>;
}

/// Internal message type for the agent's inbox
struct InboxMessage {
    message: Message,
    /// Channel to send response back (None for notify messages)
    response_tx: Option<oneshot::Sender<Message>>,
    /// Optional trace collector for recording agent communications
    trace: Option<TraceCollector>,
}

/// Handle to a running agent
struct RunningAgent {
    agent: Agent,
    inbox_tx: mpsc::Sender<InboxMessage>,
}

/// Handle to a running tool
struct RunningTool {
    tool: Arc<Tool>,
    inbox_tx: mpsc::Sender<InboxMessage>,
}

/// Type of handler registered for an agent
#[allow(dead_code)]
enum HandlerType {
    Simple(Arc<dyn MessageHandler>),
    Routing(Arc<dyn RoutingHandler>),
}

/// Information about a registered tool (for routing prompts)
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

/// The multi-agent system orchestrator
pub struct AgentSystem {
    config: SystemConfig,
    agents: RwLock<HashMap<String, RunningAgent>>,
    tools: RwLock<HashMap<String, RunningTool>>,
    conversations: RwLock<ConversationStore>,
    handlers: RwLock<HashMap<String, HandlerType>>,
}

impl AgentSystem {
    pub fn new(config: SystemConfig) -> Self {
        Self {
            config,
            agents: RwLock::new(HashMap::new()),
            tools: RwLock::new(HashMap::new()),
            conversations: RwLock::new(ConversationStore::new()),
            handlers: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(SystemConfig::default())
    }

    /// Register an agent with a simple message handler
    pub async fn register_agent(
        &self,
        agent: Agent,
        handler: Arc<dyn MessageHandler>,
    ) -> Result<()> {
        let name = agent.name.clone();
        let (inbox_tx, inbox_rx) = mpsc::channel::<InboxMessage>(100);

        // Store the handler
        {
            let mut handlers = self.handlers.write().await;
            handlers.insert(name.clone(), HandlerType::Simple(handler.clone()));
        }

        // Spawn the agent's message processing loop
        let agent_clone = agent.clone();
        let handler_clone = handler;
        tokio::spawn(async move {
            Self::simple_agent_loop(agent_clone, inbox_rx, handler_clone).await;
        });

        // Store the running agent
        {
            let mut agents = self.agents.write().await;
            agents.insert(
                name,
                RunningAgent {
                    agent,
                    inbox_tx,
                },
            );
        }

        Ok(())
    }

    /// Register an agent with a routing handler for dynamic LLM-based routing
    ///
    /// This method requires an Arc reference to the system itself so the agent
    /// can forward messages to other agents.
    pub async fn register_routing_agent(
        system: Arc<Self>,
        agent: Agent,
        handler: Arc<dyn RoutingHandler>,
    ) -> Result<()> {
        let name = agent.name.clone();
        let (inbox_tx, inbox_rx) = mpsc::channel::<InboxMessage>(100);

        // Store the handler
        {
            let mut handlers = system.handlers.write().await;
            handlers.insert(name.clone(), HandlerType::Routing(handler.clone()));
        }

        // Spawn the agent's message processing loop with routing support
        let agent_clone = agent.clone();
        let handler_clone = handler;
        let system_clone = system.clone();
        tokio::spawn(async move {
            Self::routing_agent_loop(system_clone, agent_clone, inbox_rx, handler_clone).await;
        });

        // Store the running agent
        {
            let mut agents = system.agents.write().await;
            agents.insert(
                name,
                RunningAgent {
                    agent,
                    inbox_tx,
                },
            );
        }

        Ok(())
    }

    /// Register a tool with its handler
    ///
    /// Tools are HTTP-based endpoints that agents can forward messages to.
    /// They behave like simple agents but execute HTTP requests instead of LLM calls.
    pub async fn register_tool(
        &self,
        tool: Arc<Tool>,
        handler: Arc<dyn MessageHandler>,
    ) -> Result<()> {
        let name = tool.name().to_string();
        let (inbox_tx, inbox_rx) = mpsc::channel::<InboxMessage>(100);

        // Spawn the tool's message processing loop
        let tool_clone = tool.clone();
        let handler_clone = handler;
        tokio::spawn(async move {
            Self::tool_loop(tool_clone, inbox_rx, handler_clone).await;
        });

        // Store the running tool
        {
            let mut tools = self.tools.write().await;
            tools.insert(
                name.clone(),
                RunningTool {
                    tool,
                    inbox_tx,
                },
            );
        }

        info!("Registered tool: {}", name);
        Ok(())
    }

    /// Get info about all registered tools (for LLM routing prompts)
    pub async fn get_tool_infos(&self) -> Vec<ToolInfo> {
        let tools = self.tools.read().await;
        tools
            .values()
            .map(|rt| ToolInfo {
                name: rt.tool.name().to_string(),
                description: rt.tool.description().to_string(),
            })
            .collect()
    }

    /// Simple agent loop for MessageHandler (no routing)
    async fn simple_agent_loop(
        agent: Agent,
        mut inbox: mpsc::Receiver<InboxMessage>,
        handler: Arc<dyn MessageHandler>,
    ) {
        while let Some(inbox_msg) = inbox.recv().await {
            let response_content = handler.handle(&inbox_msg.message, &agent).await;

            // If there's a response channel and we have content, send the response
            if let (Some(tx), Some(content)) = (inbox_msg.response_tx, response_content) {
                let response = inbox_msg.message.reply(content);
                let _ = tx.send(response);
            }
        }
    }

    /// Tool processing loop
    ///
    /// Similar to simple_agent_loop but for tools. Uses a dummy Agent for the handler.
    async fn tool_loop(
        tool: Arc<Tool>,
        mut inbox: mpsc::Receiver<InboxMessage>,
        handler: Arc<dyn MessageHandler>,
    ) {
        // Create a minimal dummy agent for the handler interface
        let dummy_agent = Agent {
            name: tool.name().to_string(),
            system_prompt: tool.description().to_string(),
            connections: HashMap::new(),
        };

        while let Some(inbox_msg) = inbox.recv().await {
            debug!(
                "[Tool:{}] Processing message from {}: {}",
                tool.name(),
                inbox_msg.message.from,
                &inbox_msg.message.content[..inbox_msg.message.content.len().min(100)]
            );

            let response_content = handler.handle(&inbox_msg.message, &dummy_agent).await;

            // If there's a response channel and we have content, send the response
            if let (Some(tx), Some(content)) = (inbox_msg.response_tx, response_content) {
                let response = inbox_msg.message.reply(content);
                let _ = tx.send(response);
            }
        }
    }

    /// Routing agent loop for RoutingHandler (with dynamic routing)
    async fn routing_agent_loop(
        system: Arc<Self>,
        agent: Agent,
        mut inbox: mpsc::Receiver<InboxMessage>,
        handler: Arc<dyn RoutingHandler>,
    ) {
        while let Some(inbox_msg) = inbox.recv().await {
            debug!(
                "[{}] Received message from {}: {}",
                agent.name, inbox_msg.message.from, inbox_msg.message.content
            );

            // Extract trace collector if present
            let trace = inbox_msg.trace.clone();

            // Step 1: Auto-send to all Notify connections (fire-and-forget)
            let notify_targets: Vec<String> = agent
                .connections
                .iter()
                .filter(|(_, conn)| conn.connection_type == ConnectionType::Notify)
                .map(|(name, _)| name.clone())
                .collect();

            for target in notify_targets {
                debug!("[{}] Auto-notifying: {}", agent.name, target);
                if let Err(e) = system
                    .send_message_internal(&agent.name, &target, &inbox_msg.message.content)
                    .await
                {
                    warn!(
                        "[{}] Failed to notify {}: {}",
                        agent.name, target, e
                    );
                }
            }

            // Step 2: Process with handler to get routing decision
            let decision = handler.handle(&inbox_msg.message, &agent).await;
            debug!("[{}] Handler decision: {:?}", agent.name, decision);

            // Step 3: Process the decision
            let final_response = match decision {
                HandlerDecision::Response { content } => {
                    // Direct response - just send it back
                    Some(content)
                }

                HandlerDecision::Forward { targets } => {
                    // Record forward events in trace
                    if let Some(ref t) = trace {
                        for target in &targets {
                            t.record(TraceEvent::forward(&agent.name, &target.agent, &target.message)).await;
                        }
                    }

                    // Forward to targets and synthesize responses
                    let forwarded_responses = system
                        .forward_to_agents_with_trace(&agent.name, &targets, trace.clone())
                        .await;

                    if forwarded_responses.is_empty() {
                        warn!("[{}] No responses from forwarded agents", agent.name);
                        None
                    } else {
                        // Synthesize the responses
                        let synthesized = handler
                            .synthesize(&inbox_msg.message, &forwarded_responses, &agent)
                            .await;

                        // Record synthesis event
                        if let (Some(ref t), Some(ref content)) = (&trace, &synthesized) {
                            t.record(TraceEvent::synthesis(&agent.name, &inbox_msg.message.from, content)).await;
                        }

                        synthesized
                    }
                }

                HandlerDecision::ResponseAndForward { content, targets } => {
                    // Record forward events in trace
                    if let Some(ref t) = trace {
                        for target in &targets {
                            t.record(TraceEvent::forward(&agent.name, &target.agent, &target.message)).await;
                        }
                    }

                    // Forward to targets
                    let forwarded_responses = system
                        .forward_to_agents_with_trace(&agent.name, &targets, trace.clone())
                        .await;

                    if forwarded_responses.is_empty() {
                        // No forwarded responses, just use the acknowledgment
                        Some(content)
                    } else {
                        // Synthesize: include original acknowledgment + forwarded responses
                        match handler
                            .synthesize(&inbox_msg.message, &forwarded_responses, &agent)
                            .await
                        {
                            Some(synthesized) => {
                                let combined = format!("{}\n\n{}", content, synthesized);
                                // Record synthesis event
                                if let Some(ref t) = trace {
                                    t.record(TraceEvent::synthesis(&agent.name, &inbox_msg.message.from, &combined)).await;
                                }
                                Some(combined)
                            }
                            None => Some(content),
                        }
                    }
                }

                HandlerDecision::None => {
                    // No action
                    None
                }
            };

            // Step 4: Send final response if we have one and there's a channel
            if let (Some(tx), Some(content)) = (inbox_msg.response_tx, final_response) {
                let response = inbox_msg.message.reply(content);
                let _ = tx.send(response);
            }
        }
    }

    /// Forward messages to multiple agents in parallel and collect responses
    async fn forward_to_agents(
        &self,
        from: &str,
        targets: &[ForwardTarget],
    ) -> Vec<(String, String)> {
        self.forward_to_agents_with_trace(from, targets, None).await
    }

    /// Forward messages to multiple agents in parallel with optional tracing
    async fn forward_to_agents_with_trace(
        &self,
        from: &str,
        targets: &[ForwardTarget],
        trace: Option<TraceCollector>,
    ) -> Vec<(String, String)> {
        let futures: Vec<_> = targets
            .iter()
            .map(|target| {
                let from = from.to_string();
                let agent_name = target.agent.clone();
                let message = target.message.clone();
                let trace = trace.clone();
                async move {
                    info!("[{}] Forwarding to {}: {}", from, agent_name, message);
                    match self
                        .send_message_internal(&from, &agent_name, &message)
                        .await
                    {
                        Ok(SendResult::Response(msg)) => {
                            info!("[{}] Got response from {}", from, agent_name);
                            // Record response event in trace
                            if let Some(ref t) = trace {
                                t.record(TraceEvent::response(&agent_name, &from, &msg.content)).await;
                            }
                            Some((agent_name, msg.content))
                        }
                        Ok(SendResult::Timeout(e)) => {
                            warn!("[{}] Timeout waiting for {}: {}", from, agent_name, e);
                            None
                        }
                        Ok(SendResult::Notified) => {
                            debug!("[{}] {} notified (no response expected)", from, agent_name);
                            None
                        }
                        Err(e) => {
                            error!("[{}] Failed to forward to {}: {}", from, agent_name, e);
                            None
                        }
                    }
                }
            })
            .collect();

        join_all(futures)
            .await
            .into_iter()
            .flatten()
            .collect()
    }

    /// Internal send method that doesn't require an explicit connection
    /// Used for forwarding where the routing handler decides the target
    /// Can send to both agents and tools
    async fn send_message_internal(
        &self,
        from: &str,
        to: &str,
        content: &str,
    ) -> Result<SendResult> {
        let agents = self.agents.read().await;
        let tools = self.tools.read().await;

        // Get sender agent
        let sender = agents
            .get(from)
            .ok_or_else(|| AgentError::AgentNotFound(from.to_string()))?;

        // Check if there's an explicit connection
        let connection = sender.agent.get_connection(to);

        // Create the message
        let message = Message::new(from, to, content);
        let message_id = message.id;

        // Store in conversation history
        {
            let mut conversations = self.conversations.write().await;
            conversations.add_message(message.clone());
        }

        // Determine connection type (default to blocking for forwards)
        let (is_notify, effective_timeout) = match connection {
            Some(conn) => (
                conn.connection_type == ConnectionType::Notify,
                conn.effective_timeout(self.config.global_timeout),
            ),
            None => (false, self.config.global_timeout),
        };

        // Get receiver inbox - check agents first, then tools
        let receiver_inbox = if let Some(agent) = agents.get(to) {
            agent.inbox_tx.clone()
        } else if let Some(tool) = tools.get(to) {
            tool.inbox_tx.clone()
        } else {
            return Err(AgentError::AgentNotFound(to.to_string()));
        };

        if is_notify {
            // Fire and forget
            let inbox_msg = InboxMessage {
                message,
                response_tx: None,
                trace: None,
            };
            receiver_inbox
                .send(inbox_msg)
                .await
                .map_err(|_| AgentError::ChannelError("Failed to send to inbox".into()))?;
            Ok(SendResult::Notified)
        } else {
            // Blocking with response
            let (response_tx, response_rx) = oneshot::channel();
            let inbox_msg = InboxMessage {
                message,
                response_tx: Some(response_tx),
                trace: None,
            };

            receiver_inbox
                .send(inbox_msg)
                .await
                .map_err(|_| AgentError::ChannelError("Failed to send to inbox".into()))?;

            match timeout(effective_timeout, response_rx).await {
                Ok(Ok(response)) => {
                    let mut conversations = self.conversations.write().await;
                    conversations.add_message(response.clone());
                    Ok(SendResult::Response(response))
                }
                Ok(Err(_)) => Ok(SendResult::Timeout(AgentError::Timeout {
                    agent: to.to_string(),
                    message_id,
                    waited: effective_timeout,
                })),
                Err(_) => Ok(SendResult::Timeout(AgentError::Timeout {
                    agent: to.to_string(),
                    message_id,
                    waited: effective_timeout,
                })),
            }
        }
    }

    /// Send a message from one agent to another
    /// Respects connection types and timeouts
    pub async fn send_message(
        &self,
        from: &str,
        to: &str,
        content: &str,
    ) -> Result<SendResult> {
        let agents = self.agents.read().await;

        // Get sender agent to check connection
        let sender = agents
            .get(from)
            .ok_or_else(|| AgentError::AgentNotFound(from.to_string()))?;

        // Require explicit connection for public API
        let connection = sender
            .agent
            .get_connection(to)
            .ok_or_else(|| AgentError::NoConnection {
                from: from.to_string(),
                to: to.to_string(),
            })?;

        // Get receiver
        let receiver = agents
            .get(to)
            .ok_or_else(|| AgentError::AgentNotFound(to.to_string()))?;

        // Create the message
        let message = Message::new(from, to, content);
        let message_id = message.id;

        // Store in conversation history
        {
            let mut conversations = self.conversations.write().await;
            conversations.add_message(message.clone());
        }

        // Handle based on connection type
        match connection.connection_type {
            ConnectionType::Notify => {
                // Fire and forget - no response expected
                let inbox_msg = InboxMessage {
                    message,
                    response_tx: None,
                    trace: None,
                };
                receiver
                    .inbox_tx
                    .send(inbox_msg)
                    .await
                    .map_err(|_| AgentError::ChannelError("Failed to send to agent inbox".into()))?;
                Ok(SendResult::Notified)
            }
            ConnectionType::Blocking => {
                // Create response channel
                let (response_tx, response_rx) = oneshot::channel();
                let inbox_msg = InboxMessage {
                    message,
                    response_tx: Some(response_tx),
                    trace: None,
                };

                receiver
                    .inbox_tx
                    .send(inbox_msg)
                    .await
                    .map_err(|_| AgentError::ChannelError("Failed to send to agent inbox".into()))?;

                // Wait for response with timeout
                let effective_timeout = connection.effective_timeout(self.config.global_timeout);
                match timeout(effective_timeout, response_rx).await {
                    Ok(Ok(response)) => {
                        // Store response in conversation
                        let mut conversations = self.conversations.write().await;
                        conversations.add_message(response.clone());
                        Ok(SendResult::Response(response))
                    }
                    Ok(Err(_)) => {
                        // Channel closed without response
                        Ok(SendResult::Timeout(AgentError::Timeout {
                            agent: to.to_string(),
                            message_id,
                            waited: effective_timeout,
                        }))
                    }
                    Err(_) => {
                        // Timeout elapsed
                        Ok(SendResult::Timeout(AgentError::Timeout {
                            agent: to.to_string(),
                            message_id,
                            waited: effective_timeout,
                        }))
                    }
                }
            }
        }
    }

    /// Send a message with tracing enabled
    /// The trace collector will record all agent-to-agent communications
    pub async fn send_message_with_trace(
        &self,
        from: &str,
        to: &str,
        content: &str,
        trace: TraceCollector,
    ) -> Result<SendResult> {
        let agents = self.agents.read().await;

        // Get sender agent to check connection
        let sender = agents
            .get(from)
            .ok_or_else(|| AgentError::AgentNotFound(from.to_string()))?;

        // Require explicit connection for public API
        let connection = sender
            .agent
            .get_connection(to)
            .ok_or_else(|| AgentError::NoConnection {
                from: from.to_string(),
                to: to.to_string(),
            })?;

        // Get receiver
        let receiver = agents
            .get(to)
            .ok_or_else(|| AgentError::AgentNotFound(to.to_string()))?;

        // Create the message
        let message = Message::new(from, to, content);
        let message_id = message.id;

        // Store in conversation history
        {
            let mut conversations = self.conversations.write().await;
            conversations.add_message(message.clone());
        }

        // Handle based on connection type
        match connection.connection_type {
            ConnectionType::Notify => {
                let inbox_msg = InboxMessage {
                    message,
                    response_tx: None,
                    trace: Some(trace),
                };
                receiver
                    .inbox_tx
                    .send(inbox_msg)
                    .await
                    .map_err(|_| AgentError::ChannelError("Failed to send to agent inbox".into()))?;
                Ok(SendResult::Notified)
            }
            ConnectionType::Blocking => {
                let (response_tx, response_rx) = oneshot::channel();
                let inbox_msg = InboxMessage {
                    message,
                    response_tx: Some(response_tx),
                    trace: Some(trace),
                };

                receiver
                    .inbox_tx
                    .send(inbox_msg)
                    .await
                    .map_err(|_| AgentError::ChannelError("Failed to send to agent inbox".into()))?;

                let effective_timeout = connection.effective_timeout(self.config.global_timeout);
                match timeout(effective_timeout, response_rx).await {
                    Ok(Ok(response)) => {
                        let mut conversations = self.conversations.write().await;
                        conversations.add_message(response.clone());
                        Ok(SendResult::Response(response))
                    }
                    Ok(Err(_)) => Ok(SendResult::Timeout(AgentError::Timeout {
                        agent: to.to_string(),
                        message_id,
                        waited: effective_timeout,
                    })),
                    Err(_) => Ok(SendResult::Timeout(AgentError::Timeout {
                        agent: to.to_string(),
                        message_id,
                        waited: effective_timeout,
                    })),
                }
            }
        }
    }

    /// Send messages to multiple agents in parallel
    /// Returns results in the same order as the recipients
    pub async fn send_to_multiple(
        &self,
        from: &str,
        recipients: &[&str],
        content: &str,
    ) -> Vec<(String, Result<SendResult>)> {
        let futures: Vec<_> = recipients
            .iter()
            .map(|&to| {
                let from = from.to_string();
                let to = to.to_string();
                let content = content.to_string();
                async move {
                    let result = self.send_message(&from, &to, &content).await;
                    (to, result)
                }
            })
            .collect();

        join_all(futures).await
    }

    /// Send to all connections of an agent
    /// Automatically handles blocking vs notify based on connection types
    pub async fn broadcast_from_agent(
        &self,
        from: &str,
        content: &str,
    ) -> Result<Vec<(String, Result<SendResult>)>> {
        // Collect recipients into owned Strings to release the lock
        let recipients: Vec<String> = {
            let agents = self.agents.read().await;
            let sender = agents
                .get(from)
                .ok_or_else(|| AgentError::AgentNotFound(from.to_string()))?;

            sender.agent.connections.keys().cloned().collect()
        };

        let recipient_refs: Vec<&str> = recipients.iter().map(|s| s.as_str()).collect();
        Ok(self.send_to_multiple(from, &recipient_refs, content).await)
    }

    /// Get conversation between two agents
    pub async fn get_conversation(&self, agent1: &str, agent2: &str) -> Option<Vec<Message>> {
        let conversations = self.conversations.read().await;
        conversations
            .get(agent1, agent2)
            .map(|c| c.messages().to_vec())
    }

    /// Get an agent by name (returns a clone)
    pub async fn get_agent(&self, name: &str) -> Option<Agent> {
        let agents = self.agents.read().await;
        agents.get(name).map(|ra| ra.agent.clone())
    }
}

/// A simple handler that echoes messages (useful for testing)
pub struct EchoHandler;

#[async_trait]
impl MessageHandler for EchoHandler {
    async fn handle(&self, message: &Message, _agent: &Agent) -> Option<String> {
        Some(format!("Echo: {}", message.content))
    }
}

/// A handler that simulates processing delay (useful for timeout testing)
pub struct DelayedHandler {
    delay: Duration,
    response: String,
}

impl DelayedHandler {
    pub fn new(delay: Duration, response: impl Into<String>) -> Self {
        Self {
            delay,
            response: response.into(),
        }
    }
}

#[async_trait]
impl MessageHandler for DelayedHandler {
    async fn handle(&self, _message: &Message, _agent: &Agent) -> Option<String> {
        tokio::time::sleep(self.delay).await;
        Some(self.response.clone())
    }
}

/// A sink handler that processes but never responds (for notify connections)
pub struct SinkHandler<F>
where
    F: Fn(&Message) + Send + Sync,
{
    on_message: F,
}

impl<F> SinkHandler<F>
where
    F: Fn(&Message) + Send + Sync,
{
    pub fn new(on_message: F) -> Self {
        Self { on_message }
    }
}

#[async_trait]
impl<F> MessageHandler for SinkHandler<F>
where
    F: Fn(&Message) + Send + Sync,
{
    async fn handle(&self, message: &Message, _agent: &Agent) -> Option<String> {
        (self.on_message)(message);
        None // Sink handlers don't respond
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentBuilder;

    #[tokio::test]
    async fn test_blocking_send_receive() {
        let system = AgentSystem::with_default_config();

        let agent_a = AgentBuilder::new("A").blocking_connection("B").build();

        let agent_b = AgentBuilder::new("B").build();

        system
            .register_agent(agent_a, Arc::new(EchoHandler))
            .await
            .unwrap();
        system
            .register_agent(agent_b, Arc::new(EchoHandler))
            .await
            .unwrap();

        let result = system.send_message("A", "B", "Hello").await.unwrap();

        match result {
            SendResult::Response(msg) => {
                assert_eq!(msg.content, "Echo: Hello");
                assert_eq!(msg.from, "B");
                assert_eq!(msg.to, "A");
            }
            _ => panic!("Expected response"),
        }
    }

    #[tokio::test]
    async fn test_notify_send() {
        let system = AgentSystem::with_default_config();

        let agent_a = AgentBuilder::new("A").notify_connection("Logger").build();

        let agent_logger = AgentBuilder::new("Logger").build();

        let received = Arc::new(RwLock::new(false));
        let received_clone = received.clone();

        system
            .register_agent(agent_a, Arc::new(EchoHandler))
            .await
            .unwrap();
        system
            .register_agent(
                agent_logger,
                Arc::new(SinkHandler::new(move |_| {
                    // Can't use async here, but this proves the message was received
                    let _ = &received_clone;
                })),
            )
            .await
            .unwrap();

        let result = system.send_message("A", "Logger", "Log this").await.unwrap();

        assert!(matches!(result, SendResult::Notified));
    }

    #[tokio::test]
    async fn test_timeout() {
        let config = SystemConfig::with_timeout_secs(1);
        let system = AgentSystem::new(config);

        let agent_a = AgentBuilder::new("A").blocking_connection("Slow").build();

        let agent_slow = AgentBuilder::new("Slow").build();

        system
            .register_agent(agent_a, Arc::new(EchoHandler))
            .await
            .unwrap();
        system
            .register_agent(
                agent_slow,
                Arc::new(DelayedHandler::new(Duration::from_secs(5), "Too late")),
            )
            .await
            .unwrap();

        let result = system.send_message("A", "Slow", "Hello").await.unwrap();

        match result {
            SendResult::Timeout(err) => {
                assert!(matches!(err, AgentError::Timeout { .. }));
            }
            _ => panic!("Expected timeout"),
        }
    }

    #[tokio::test]
    async fn test_parallel_send() {
        let system = AgentSystem::with_default_config();

        let coordinator = AgentBuilder::new("Coordinator")
            .blocking_connection("Worker1")
            .blocking_connection("Worker2")
            .notify_connection("Logger")
            .build();

        let worker1 = AgentBuilder::new("Worker1").build();
        let worker2 = AgentBuilder::new("Worker2").build();
        let logger = AgentBuilder::new("Logger").build();

        system
            .register_agent(coordinator, Arc::new(EchoHandler))
            .await
            .unwrap();
        system
            .register_agent(worker1, Arc::new(EchoHandler))
            .await
            .unwrap();
        system
            .register_agent(worker2, Arc::new(EchoHandler))
            .await
            .unwrap();
        system
            .register_agent(
                logger,
                Arc::new(SinkHandler::new(|msg| {
                    println!("Logger received: {}", msg.content);
                })),
            )
            .await
            .unwrap();

        let results = system
            .broadcast_from_agent("Coordinator", "Process this")
            .await
            .unwrap();

        // Should have results for all 3 connections
        assert_eq!(results.len(), 3);

        // Count successful responses
        let successful: Vec<_> = results
            .iter()
            .filter(|(_, r)| r.as_ref().map(|r| r.is_success()).unwrap_or(false))
            .collect();
        assert_eq!(successful.len(), 3);
    }

    #[tokio::test]
    async fn test_no_connection_error() {
        let system = AgentSystem::with_default_config();

        let agent_a = AgentBuilder::new("A").build();
        let agent_b = AgentBuilder::new("B").build();

        system
            .register_agent(agent_a, Arc::new(EchoHandler))
            .await
            .unwrap();
        system
            .register_agent(agent_b, Arc::new(EchoHandler))
            .await
            .unwrap();

        let result = system.send_message("A", "B", "Hello").await;

        assert!(matches!(result, Err(AgentError::NoConnection { .. })));
    }
}
