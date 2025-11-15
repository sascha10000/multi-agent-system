use std::collections::HashMap;
use crate::agent::Agent;
use crate::message::Message;

/// Multi-agent system manager
pub struct AgentSystem {
    agents: HashMap<String, Agent>,
}

impl AgentSystem {
    /// Creates a new agent system
    pub fn new() -> Self {
        AgentSystem {
            agents: HashMap::new(),
        }
    }

    /// Adds an agent to the system
    pub fn add_agent(&mut self, agent: Agent) -> Result<(), String> {
        if self.agents.contains_key(&agent.name) {
            return Err(format!("Agent with name '{}' already exists", agent.name));
        }
        self.agents.insert(agent.name.clone(), agent);
        Ok(())
    }

    /// Removes an agent from the system
    pub fn remove_agent(&mut self, name: &str) -> Result<Agent, String> {
        self.agents
            .remove(name)
            .ok_or_else(|| format!("Agent '{}' not found", name))
    }

    /// Gets an agent by name
    pub fn get_agent(&self, name: &str) -> Option<&Agent> {
        self.agents.get(name)
    }

    /// Connects two agents bidirectionally
    pub fn connect_agents(&mut self, agent1_name: &str, agent2_name: &str) -> Result<(), String> {
        if !self.agents.contains_key(agent1_name) {
            return Err(format!("Agent '{}' not found", agent1_name));
        }
        if !self.agents.contains_key(agent2_name) {
            return Err(format!("Agent '{}' not found", agent2_name));
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
    pub fn disconnect_agents(&mut self, agent1_name: &str, agent2_name: &str) -> Result<(), String> {
        if let Some(agent1) = self.agents.get(agent1_name) {
            agent1.disconnect_from(agent2_name);
        }
        if let Some(agent2) = self.agents.get(agent2_name) {
            agent2.disconnect_from(agent1_name);
        }
        Ok(())
    }

    /// Sends a message from one agent to another (only if connected)
    pub fn send_message(&self, from: &str, to: &str, content: String) -> Result<Message, String> {
        let sender = self.agents.get(from)
            .ok_or_else(|| format!("Sender agent '{}' not found", from))?;

        let recipient = self.agents.get(to)
            .ok_or_else(|| format!("Recipient agent '{}' not found", to))?;

        if !sender.is_connected_to(to) {
            return Err(format!("Agent '{}' is not connected to '{}'", from, to));
        }

        let message = Message::new(from.to_string(), to.to_string(), content);

        // Trigger the recipient's on_message handler
        recipient.on_message(&message);

        Ok(message)
    }

    /// Broadcasts a message from one agent to all its connected agents
    pub fn send_broadcast(&self, from: &str, content: String) -> Result<Vec<Message>, String> {
        let sender = self.agents.get(from)
            .ok_or_else(|| format!("Sender agent '{}' not found", from))?;

        let connections = sender.get_connections();
        let mut sent_messages = Vec::new();

        for recipient_name in connections {
            if let Some(recipient) = self.agents.get(&recipient_name) {
                let message = Message::new(from.to_string(), recipient_name.clone(), content.clone());
                recipient.on_message(&message);
                sent_messages.push(message);
            }
        }

        Ok(sent_messages)
    }

    /// Lists all agents in the system
    pub fn list_agents(&self) -> Vec<&Agent> {
        self.agents.values().collect()
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

    #[test]
    fn test_message_requires_connection() {
        let mut system = AgentSystem::new();
        let agent1 = Agent::new("Agent1".to_string(), "Role1".to_string());
        let agent2 = Agent::new("Agent2".to_string(), "Role2".to_string());

        system.add_agent(agent1).unwrap();
        system.add_agent(agent2).unwrap();

        // Should fail - not connected
        assert!(system.send_message("Agent1", "Agent2", "Hello".to_string()).is_err());

        // Connect and try again
        system.connect_agents("Agent1", "Agent2").unwrap();
        assert!(system.send_message("Agent1", "Agent2", "Hello".to_string()).is_ok());
    }

    #[test]
    fn test_remove_agent() {
        let mut system = AgentSystem::new();
        let agent = Agent::new("Agent1".to_string(), "Role1".to_string());
        system.add_agent(agent).unwrap();

        assert!(system.remove_agent("Agent1").is_ok());
        assert!(system.get_agent("Agent1").is_none());
    }
}
