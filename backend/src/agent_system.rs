use crate::errors::AgentError;
use crate::message::Message;
use crate::{agent::Agent, errors::SessionError};
use futures::future::join_all;
use std::collections::{HashMap, HashSet};
use std::error::Error;

/// Multi-agent system manager
pub struct AgentSystem {
    agents: HashMap<String, Agent>,
    session_ids: HashSet<String>,
    active_session: Option<String>, // Single active session for the entire system
}

impl AgentSystem {
    /// Creates a new agent system
    pub fn new() -> Self {
        AgentSystem {
            agents: HashMap::new(),
            session_ids: HashSet::new(),
            active_session: None,
        }
    }

    /// Adds an agent to the system
    pub fn add_agent(&mut self, agent: Agent) -> Result<(), AgentError> {
        if self.agents.contains_key(&agent.name) {
            return Err(AgentError::Exists(agent.name));
        }
        self.agents.insert(agent.name.clone(), agent);
        Ok(())
    }

    /// Removes an agent from the system
    pub fn remove_agent(&mut self, name: &str) -> Result<Agent, AgentError> {
        if !self.agents.contains_key(name) {
            return Err(AgentError::NotFound(name.to_string()));
        }

        // Remove all connections to this agent
        self.remove_connections(name);

        self.agents
            .remove(name)
            .ok_or_else(|| AgentError::NotFound(name.to_string()))
    }

    /// Removes all connections to a specific agent
    fn remove_connections(&mut self, agent_name: &str) {
        // Get all other agents and disconnect them from the target agent
        for (_, agent) in self.agents.iter() {
            if agent.name != agent_name {
                agent.disconnect_from(agent_name);
            }
        }
    }

    /// Gets an agent by name
    pub fn get_agent(&self, name: &str) -> Option<&Agent> {
        self.agents.get(name)
    }

    /// Connects two agents bidirectionally
    pub fn connect_agents(
        &mut self,
        agent1_name: &str,
        agent2_name: &str,
    ) -> Result<(), AgentError> {
        if !self.agents.contains_key(agent1_name) {
            return Err(AgentError::NotFound(agent1_name.to_string()));
        }
        if !self.agents.contains_key(agent2_name) {
            return Err(AgentError::NotFound(agent2_name.to_string()));
        }

        if let Some(agent1) = self.agents.get(agent1_name) {
            agent1.connect_to(agent2_name);
        }
        if let Some(agent2) = self.agents.get(agent2_name) {
            agent2.connect_to(agent1_name);
        }

        Ok(())
    }

    /// Disconnects two agents bidirectionally
    pub fn disconnect_agents(
        &mut self,
        agent1_name: &str,
        agent2_name: &str,
    ) -> Result<(), String> {
        if let Some(agent1) = self.agents.get(agent1_name) {
            agent1.disconnect_from(agent2_name);
        }
        if let Some(agent2) = self.agents.get(agent2_name) {
            agent2.disconnect_from(agent1_name);
        }
        Ok(())
    }

    /// Sends a message from one agent to another (only if connected)
    pub fn send_message(
        &self,
        from: &str,
        to: &str,
        content: String,
    ) -> Result<Message, Box<dyn Error>> {
        let sender = self
            .agents
            .get(from)
            .ok_or_else(|| AgentError::NotFound(from.to_string()))?;

        let recipient = self
            .agents
            .get(to)
            .ok_or_else(|| AgentError::NotFound(to.to_string()))?;

        if !sender.is_connected_to(to) {
            return Err(Box::new(AgentError::NotConnected(
                from.to_string(),
                to.to_string(),
            )));
        }

        // Get the recipient's active session
        let session_id = self
            .get_active_session()
            .ok_or_else(|| AgentError::NoActiveSession(to.to_string()))?;

        let message = Message::new(from.to_string(), to.to_string(), content);

        // Trigger the recipient's send_message handler with the active session
        recipient.send_message(&session_id, message.clone())?;

        Ok(message)
    }

    /// Broadcasts a message from one agent to all its connected agents
    pub fn send_broadcast(&self, from: &str, content: String) -> Result<Vec<Message>, String> {
        let sender = self
            .agents
            .get(from)
            .ok_or_else(|| format!("Sender agent '{}' not found", from))?;

        let connections = sender.get_connections();
        let mut sent_messages = Vec::new();

        for recipient_name in connections {
            if let Some(recipient) = self.agents.get(&recipient_name) {
                // Get the recipient's active session
                if let Some(session_id) = self.get_active_session() {
                    let message =
                        Message::new(from.to_string(), recipient_name.clone(), content.clone());

                    // Only send if the recipient has an active session
                    if recipient.send_message(&session_id, message.clone()).is_ok() {
                        sent_messages.push(message);
                    }
                }
            }
        }

        Ok(sent_messages)
    }

    /// Lists all agents in the system
    pub fn list_agents(&self) -> Vec<&Agent> {
        self.agents.values().collect()
    }

    /// Creates a session with the same ID for all agents
    /// Sets this as the active session for the entire system
    pub fn create_session(&mut self, session_id: String) -> Result<(), String> {
        // Check if session already exists
        if self.session_ids.contains(&session_id) {
            return Err(format!("Session '{}' already exists", session_id));
        }

        // Create session for each existing agent with the same session_id
        for (_agent_name, agent) in self.agents.iter() {
            // Create session in the agent with the same ID
            agent.create_session(session_id.clone())?;
            // Start the async message processing loop for this agent's session
            let join_handle = agent.start_session(&session_id);
            let _ = agent.set_session_join_handle(&session_id, join_handle);
        }

        // Add to system-wide session list (only once since all agents share it)
        self.session_ids.insert(session_id.clone());

        // Set as the active session for the entire system if no active session exists
        if self.active_session.is_none() {
            self.active_session = Some(session_id);
        }

        Ok(())
    }

    /// Sets the active session for the entire system
    pub fn set_active_session(&mut self, session_id: String) -> Result<(), SessionError> {
        // Verify that the session exists
        if !self.session_ids.contains(&session_id) {
            return Err(SessionError::NotFound(session_id.to_string()));
        }

        self.active_session = Some(session_id);
        Ok(())
    }

    /// Gets the active session base ID
    pub fn get_active_session(&self) -> Option<&String> {
        self.active_session.as_ref()
    }

    /// Lists all session IDs in the system
    pub fn list_all_sessions(&self) -> Vec<String> {
        self.session_ids.iter().cloned().collect()
    }

    /// Removes a session from all agents
    pub fn remove_session(&mut self, session_id: &str) -> Result<(), String> {
        // Remove session from each agent
        for (_agent_name, agent) in self.agents.iter() {
            // Try to remove the session (ignore if it doesn't exist)
            let _ = agent.remove_session(session_id);
        }

        // Remove from system-wide session list
        self.session_ids.remove(session_id);

        // If this was the active session, clear it
        if let Some(active) = &self.active_session {
            if active == session_id {
                self.active_session = None;
            }
        }

        Ok(())
    }

    /// Waits for all session processing tasks to complete
    /// Takes ownership of the JoinHandles, removes the session (signaling tasks to exit),
    /// and awaits them concurrently
    pub async fn wait_for_session_tasks(&mut self, session_id: &str) -> Result<(), String> {
        // Collect all join handles for this session BEFORE removing it
        let mut handles = Vec::new();
        for agent in self.agents.values() {
            if let Some(handle) = agent.take_session_join_handle(session_id) {
                handles.push(handle);
            }
        }

        if handles.is_empty() {
            return Err(format!(
                "No active processing tasks found for session '{}'",
                session_id
            ));
        }

        println!("Waiting for {} processing tasks...", handles.len());

        // Remove the session to signal tasks to exit
        // TODO: The problem here is that the threads get basically killed eventhough it may be
        // possible that there is still some message in the queue. This should just happen on exit.
        self.remove_session(session_id)?;

        // Wait for all handles
        let results = join_all(handles).await;

        // Check if any tasks panicked
        let mut had_errors = false;
        for (i, result) in results.into_iter().enumerate() {
            if let Err(e) = result {
                eprintln!("Task {} panicked: {:?}", i, e);
                had_errors = true;
            }
        }

        if had_errors {
            Err("Some tasks panicked".to_string())
        } else {
            println!("All tasks completed successfully");
            Ok(())
        }
    }
}

impl Default for AgentSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_agent() {
        let mut system = AgentSystem::new();
        let agent = Agent::new("Agent1".to_string(), "Role1".to_string());
        assert!(system.add_agent(agent).is_ok());
    }

    #[test]
    fn test_duplicate_agent() {
        let mut system = AgentSystem::new();
        let agent1 = Agent::new("Agent1".to_string(), "Role1".to_string());
        let agent2 = Agent::new("Agent1".to_string(), "Role2".to_string());

        system.add_agent(agent1).unwrap();
        assert!(system.add_agent(agent2).is_err());
    }

    #[tokio::test]
    async fn test_message_requires_connection() {
        let mut system = AgentSystem::new();
        let agent1 = Agent::new("Agent1".to_string(), "Role1".to_string());
        let agent2 = Agent::new("Agent2".to_string(), "Role2".to_string());

        system.add_agent(agent1).unwrap();
        system.add_agent(agent2).unwrap();

        // Create a session for the agents
        system.create_session("test-session".to_string()).unwrap();

        // Should fail - not connected
        assert!(system
            .send_message("Agent1", "Agent2", "Hello".to_string())
            .is_err());

        // Connect and try again
        system.connect_agents("Agent1", "Agent2").unwrap();
        assert!(system
            .send_message("Agent1", "Agent2", "Hello".to_string())
            .is_ok());
    }

    #[test]
    fn test_remove_agent() {
        let mut system = AgentSystem::new();
        let agent = Agent::new("Agent1".to_string(), "Role1".to_string());
        system.add_agent(agent).unwrap();

        assert!(system.remove_agent("Agent1").is_ok());
        assert!(system.get_agent("Agent1").is_none());
    }

    #[tokio::test]
    async fn test_send_broadcast() {
        let mut system = AgentSystem::new();
        let broadcaster = Agent::new("Broadcaster".to_string(), "Role1".to_string());
        let agent1 = Agent::new("Agent1".to_string(), "Role2".to_string());
        let agent2 = Agent::new("Agent2".to_string(), "Role3".to_string());
        let agent3 = Agent::new("Agent3".to_string(), "Role4".to_string());

        system.add_agent(broadcaster).unwrap();
        system.add_agent(agent1).unwrap();
        system.add_agent(agent2).unwrap();
        system.add_agent(agent3).unwrap();

        // Create a session for the agents
        system.create_session("test-session".to_string()).unwrap();

        // Connect broadcaster to agent1 and agent2, but not agent3
        system.connect_agents("Broadcaster", "Agent1").unwrap();
        system.connect_agents("Broadcaster", "Agent2").unwrap();

        // Broadcast message
        let result = system.send_broadcast("Broadcaster", "Hello everyone!".to_string());
        assert!(result.is_ok());

        let messages = result.unwrap();
        assert_eq!(messages.len(), 2);

        // Verify messages were sent to connected agents
        assert!(messages.iter().any(|m| m.to == "Agent1"));
        assert!(messages.iter().any(|m| m.to == "Agent2"));
        assert!(!messages.iter().any(|m| m.to == "Agent3"));

        // Verify all messages have correct sender and content
        for msg in messages {
            assert_eq!(msg.from, "Broadcaster");
            assert_eq!(msg.content, "Hello everyone!");
        }
    }

    #[test]
    fn test_broadcast_no_connections() {
        let mut system = AgentSystem::new();
        let agent = Agent::new("LoneAgent".to_string(), "Role1".to_string());
        system.add_agent(agent).unwrap();

        let result = system.send_broadcast("LoneAgent", "Anyone there?".to_string());
        assert!(result.is_ok());

        let messages = result.unwrap();
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_broadcast_nonexistent_sender() {
        let system = AgentSystem::new();
        let result = system.send_broadcast("NonExistent", "Hello".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_agent_cleans_connections() {
        let mut system = AgentSystem::new();
        let agent1 = Agent::new("Agent1".to_string(), "Role1".to_string());
        let agent2 = Agent::new("Agent2".to_string(), "Role2".to_string());
        let agent3 = Agent::new("Agent3".to_string(), "Role3".to_string());

        system.add_agent(agent1).unwrap();
        system.add_agent(agent2).unwrap();
        system.add_agent(agent3).unwrap();

        // Connect agents
        system.connect_agents("Agent1", "Agent2").unwrap();
        system.connect_agents("Agent1", "Agent3").unwrap();

        // Verify connections exist
        let agent2 = system.get_agent("Agent2").unwrap();
        let agent3 = system.get_agent("Agent3").unwrap();
        assert!(agent2.is_connected_to("Agent1"));
        assert!(agent3.is_connected_to("Agent1"));

        // Remove Agent1
        assert!(system.remove_agent("Agent1").is_ok());

        // Verify connections were cleaned up
        let agent2 = system.get_agent("Agent2").unwrap();
        let agent3 = system.get_agent("Agent3").unwrap();
        assert!(!agent2.is_connected_to("Agent1"));
        assert!(!agent3.is_connected_to("Agent1"));
    }
}
