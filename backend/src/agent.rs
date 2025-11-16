use crate::message::Message;
use crate::session::Session;
use crate::{LLMChat, OllamaChat};
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

    pub fn start_session(&self, session_id: &str) -> tokio::task::JoinHandle<()> {
        let c_session_id = session_id.to_string().clone();
        let sessions = Arc::clone(&self.sessions);
        let agent_name = self.name.clone();
        let agent_role = self.role.clone();

        tokio::spawn(async move {
            loop {
                // Lock, check for message, and release lock immediately
                let message_opt = {
                    let mut sessions_guard = sessions.lock().unwrap();
                    if let Some(session) = sessions_guard.get_mut(&c_session_id) {
                        session.pop_message_from_stack()
                    } else {
                        // Session doesn't exist, exit the loop
                        break;
                    }
                };

                // Process message outside the lock
                if let Some(message) = message_opt {
                    println!(
                        "[{}] Received message from {}: {}",
                        agent_name, message.from, message.content
                    );

                    // Create LLM client and process message
                    let llm = OllamaChat::new(
                        String::from("http://localhost:11434"),
                        String::from("gemma3:4b"),
                    );

                    let result = llm
                        .send_message_with_system(&agent_role, &message.content)
                        .await;

                    match result {
                        Ok(response) => {
                            println!("[{}] Generated response: {}", agent_name, response);
                            // Store the message and response in the session
                            let mut sessions_guard = sessions.lock().unwrap();
                            if let Some(session) = sessions_guard.get_mut(&c_session_id) {
                                session.add_message_with_response(message, response);
                            }
                        }
                        Err(e) => {
                            eprintln!("[{}] Error processing message: {}", agent_name, e);
                        }
                    }
                }

                // Sleep to prevent busy-waiting
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        })
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

        session.push_message_to_stack(message);
        drop(sessions);

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

    /// Sets the join handle for a session's processing task
    pub fn set_session_join_handle(
        &self,
        session_id: &str,
        handle: tokio::task::JoinHandle<()>,
    ) -> Result<(), String> {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            session.set_join_handle(handle);
            Ok(())
        } else {
            Err(format!("Session '{}' not found", session_id))
        }
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
