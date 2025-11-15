use crate::message::Message;
use crate::session::Session;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

/// Represents an agent with a name and role (prompt)
#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub role: String,
    connections: Arc<Mutex<HashSet<String>>>,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
}

impl Agent {
    /// Creates a new agent with the given name and role
    pub fn new(name: String, role: String) -> Self {
        Agent {
            name,
            role,
            connections: Arc::new(Mutex::new(HashSet::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
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

    /// Sends a message to this agent, managing the message stack for the given session
    pub fn send_message(&self, session_id: &str, message: Message) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("Session '{}' not found", session_id))?;

        if session.is_message_stack_empty() {
            // Stack is empty, process message directly without adding to stack
            drop(sessions); // Release the lock before calling on_message
            self.on_message(&message);
        } else {
            // Stack has messages, add new message to back
            session.push_message_to_stack(message);
            // Take oldest message from front
            if let Some(oldest_message) = session.pop_message_from_stack() {
                drop(sessions); // Release the lock before calling on_message
                self.on_message(&oldest_message);
            }
        }

        Ok(())
    }

    /// Creates a new session for this agent
    pub fn create_session(&self, session_id: String) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        if sessions.contains_key(&session_id) {
            return Err(format!("Session '{}' already exists", session_id));
        }
        sessions.insert(session_id.clone(), Session::new(session_id));
        Ok(())
    }

    /// Gets a session by ID (returns a clone)
    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(session_id).cloned()
    }

    /// Adds a message to a session
    fn add_message_to_session(&self, session_id: &str, message: Message) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            session.add_message(message);
            Ok(())
        } else {
            Err(format!("Session '{}' not found", session_id))
        }
    }

    /// Adds a message with response to a session
    fn add_message_with_response_to_session(
        &self,
        session_id: &str,
        message: Message,
        response: String,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            session.add_message_with_response(message, response);
            Ok(())
        } else {
            Err(format!("Session '{}' not found", session_id))
        }
    }

    /// Lists all session IDs
    pub fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.lock().unwrap();
        sessions.keys().cloned().collect()
    }

    /// Removes a session
    pub fn remove_session(&self, session_id: &str) -> Result<Session, String> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions
            .remove(session_id)
            .ok_or_else(|| format!("Session '{}' not found", session_id))
    }

    /// Handles incoming messages - to be implemented later (private)
    /// In this function the basic logic will be triggered. Meaning how the agents talk to each
    /// other and if they talk to each other.
    fn on_message(&self, message: &Message) {
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
