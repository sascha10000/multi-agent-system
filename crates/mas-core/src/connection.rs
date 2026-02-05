use std::time::Duration;

/// Defines how a connection behaves when sending messages
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    /// Sender waits for a response (with timeout)
    Blocking,
    /// Fire-and-forget - sender continues immediately, no response expected
    Notify,
}

/// A connection from one agent to another
#[derive(Debug, Clone)]
pub struct Connection {
    /// The type of connection (Blocking or Notify)
    pub connection_type: ConnectionType,
    /// Per-connection timeout override (takes priority over global timeout)
    pub timeout: Option<Duration>,
    /// Role/description of the target agent (for routing decisions)
    pub target_role: Option<String>,
}

impl Connection {
    /// Create a new blocking connection with optional timeout
    pub fn blocking(timeout: Option<Duration>) -> Self {
        Self {
            connection_type: ConnectionType::Blocking,
            timeout,
            target_role: None,
        }
    }

    /// Create a new blocking connection with target role info
    pub fn blocking_with_role(timeout: Option<Duration>, role: impl Into<String>) -> Self {
        Self {
            connection_type: ConnectionType::Blocking,
            timeout,
            target_role: Some(role.into()),
        }
    }

    /// Create a new notify (fire-and-forget) connection
    pub fn notify() -> Self {
        Self {
            connection_type: ConnectionType::Notify,
            timeout: None,
            target_role: None,
        }
    }

    /// Create a new notify connection with target role info
    pub fn notify_with_role(role: impl Into<String>) -> Self {
        Self {
            connection_type: ConnectionType::Notify,
            timeout: None,
            target_role: Some(role.into()),
        }
    }

    /// Set the target role
    pub fn with_target_role(mut self, role: impl Into<String>) -> Self {
        self.target_role = Some(role.into());
        self
    }

    /// Check if this is a blocking connection
    pub fn is_blocking(&self) -> bool {
        matches!(self.connection_type, ConnectionType::Blocking)
    }

    /// Get the effective timeout, falling back to provided default
    pub fn effective_timeout(&self, default: Duration) -> Duration {
        self.timeout.unwrap_or(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocking_connection() {
        let conn = Connection::blocking(Some(Duration::from_secs(5)));
        assert!(conn.is_blocking());
        assert_eq!(conn.timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn test_notify_connection() {
        let conn = Connection::notify();
        assert!(!conn.is_blocking());
        assert_eq!(conn.timeout, None);
    }

    #[test]
    fn test_effective_timeout() {
        let default = Duration::from_secs(10);

        let conn_with_override = Connection::blocking(Some(Duration::from_secs(5)));
        assert_eq!(conn_with_override.effective_timeout(default), Duration::from_secs(5));

        let conn_without_override = Connection::blocking(None);
        assert_eq!(conn_without_override.effective_timeout(default), Duration::from_secs(10));
    }
}
