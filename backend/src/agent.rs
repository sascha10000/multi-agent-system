use crate::message::Message;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

/// Represents an agent with a name and role (prompt)
#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub role: String,
    connections: Arc<Mutex<HashSet<String>>>,
    message_stack: Arc<Mutex<VecDeque<Message>>>,
}

impl Agent {
    /// Creates a new agent with the given name and role
    pub fn new(name: String, role: String) -> Self {
        Agent {
            name,
            role,
            connections: Arc::new(Mutex::new(HashSet::new())),
            message_stack: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Connects this agent to another agent
    pub fn connect_to(&self, other_agent_name: &str) {
        let mut connections = self.connections.lock().unwrap();
        connections.insert(other_agent_name.to_string());
    }

    /// Disconnects this agent from another agent
    pub fn disconnect_from(&self, other_agent_name: &str) {
        let mut connections = self.connections.lock().unwrap();
        connections.remove(other_agent_name);
    }

    /// Checks if this agent is connected to another agent
    pub fn is_connected_to(&self, other_agent_name: &str) -> bool {
        let connections = self.connections.lock().unwrap();
        connections.contains(other_agent_name)
    }

    /// Gets all connected agent names
    pub fn get_connections(&self) -> Vec<String> {
        let connections = self.connections.lock().unwrap();
        connections.iter().cloned().collect()
    }

    /// Sends a message to this agent, managing the message stack
    pub fn send_message(&self, message: Message) {
        let mut stack = self.message_stack.lock().unwrap();

        if stack.is_empty() {
            // Stack is empty, process message directly without adding to stack
            drop(stack); // Release the lock before calling on_message
            self.on_message(&message);
        } else {
            // Stack has messages, add new message to back
            stack.push_back(message);
            // Take oldest message from front
            if let Some(oldest_message) = stack.pop_front() {
                drop(stack); // Release the lock before calling on_message
                self.on_message(&oldest_message);
            }
        }
    }

    /// Handles incoming messages - to be implemented later (private)
    /// In this function the basic logic will be triggered. Meaning how the agents talk to each
    /// other and if they talk to each other.
    async fn on_message(&self, message: &Message) {
        // Placeholder implementation
        println!(
            "[{}] Received message from {}: {}",
            self.name, message.from, message.content
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new("TestAgent".to_string(), "Test role".to_string());
        assert_eq!(agent.name, "TestAgent");
        assert_eq!(agent.role, "Test role");
    }

    #[test]
    fn test_agent_connection() {
        let agent1 = Agent::new("Agent1".to_string(), "Role1".to_string());
        let agent2_name = "Agent2";

        agent1.connect_to(agent2_name);
        assert!(agent1.is_connected_to(agent2_name));
    }

    #[test]
    fn test_agent_disconnection() {
        let agent = Agent::new("Agent1".to_string(), "Role1".to_string());
        agent.connect_to("Agent2");
        assert!(agent.is_connected_to("Agent2"));

        agent.disconnect_from("Agent2");
        assert!(!agent.is_connected_to("Agent2"));
    }
}
