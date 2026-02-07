use crate::connection::Connection;
use std::collections::HashMap;

/// An agent in the multi-agent system
#[derive(Debug, Clone)]
pub struct Agent {
    /// Unique identifier for this agent
    pub name: String,
    /// Base instructions for the LLM
    pub system_prompt: String,
    /// Connections to other agents (key is the target agent's name)
    pub connections: HashMap<String, Connection>,
}

impl Agent {
    pub fn new(name: impl Into<String>, system_prompt: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            system_prompt: system_prompt.into(),
            connections: HashMap::new(),
        }
    }

    /// Add a connection to another agent
    pub fn add_connection(&mut self, target: impl Into<String>, connection: Connection) -> &mut Self {
        self.connections.insert(target.into(), connection);
        self
    }

    /// Check if this agent can send to another agent
    pub fn can_send_to(&self, target: &str) -> bool {
        self.connections.contains_key(target)
    }

    /// Get the connection to a specific agent
    pub fn get_connection(&self, target: &str) -> Option<&Connection> {
        self.connections.get(target)
    }

    /// Get all connected agent names
    pub fn connected_agents(&self) -> impl Iterator<Item = &String> {
        self.connections.keys()
    }
}

/// Builder for creating agents with a fluent API
pub struct AgentBuilder {
    name: String,
    system_prompt: String,
    connections: HashMap<String, Connection>,
}

impl AgentBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            system_prompt: String::new(),
            connections: HashMap::new(),
        }
    }

    pub fn system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = system_prompt.into();
        self
    }

    /// Alias for system_prompt for backwards compatibility
    #[deprecated(since = "0.2.0", note = "Use system_prompt() instead")]
    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    pub fn connection(mut self, target: impl Into<String>, connection: Connection) -> Self {
        self.connections.insert(target.into(), connection);
        self
    }

    pub fn blocking_connection(self, target: impl Into<String>) -> Self {
        self.connection(target, Connection::blocking(None))
    }

    pub fn blocking_connection_with_timeout(
        self,
        target: impl Into<String>,
        timeout: std::time::Duration,
    ) -> Self {
        self.connection(target, Connection::blocking(Some(timeout)))
    }

    pub fn notify_connection(self, target: impl Into<String>) -> Self {
        self.connection(target, Connection::notify())
    }

    pub fn build(self) -> Agent {
        Agent {
            name: self.name,
            system_prompt: self.system_prompt,
            connections: self.connections,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_agent_builder() {
        let agent = AgentBuilder::new("Coordinator")
            .system_prompt("You are a coordinator agent.")
            .blocking_connection("Worker1")
            .blocking_connection_with_timeout("Worker2", Duration::from_secs(5))
            .notify_connection("Logger")
            .build();

        assert_eq!(agent.name, "Coordinator");
        assert!(agent.can_send_to("Worker1"));
        assert!(agent.can_send_to("Worker2"));
        assert!(agent.can_send_to("Logger"));
        assert!(!agent.can_send_to("Unknown"));

        assert!(agent.get_connection("Worker1").unwrap().is_blocking());
        assert!(!agent.get_connection("Logger").unwrap().is_blocking());
    }

    #[test]
    fn test_connection_timeout_override() {
        let agent = AgentBuilder::new("Test")
            .blocking_connection_with_timeout("Target", Duration::from_secs(5))
            .build();

        let conn = agent.get_connection("Target").unwrap();
        assert_eq!(conn.timeout, Some(Duration::from_secs(5)));
    }
}
