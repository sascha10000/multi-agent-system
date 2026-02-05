use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A message sent between agents
#[derive(Debug, Clone)]
pub struct Message {
    /// Unique identifier for this message
    pub id: Uuid,
    /// Name of the sending agent
    pub from: String,
    /// Name of the receiving agent
    pub to: String,
    /// The message content
    pub content: String,
    /// When the message was created
    pub timestamp: DateTime<Utc>,
    /// Optional ID of the message this is responding to (for implicit response channels)
    pub in_reply_to: Option<Uuid>,
}

impl Message {
    /// Create a new message
    pub fn new(from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            from: from.into(),
            to: to.into(),
            content: content.into(),
            timestamp: Utc::now(),
            in_reply_to: None,
        }
    }

    /// Create a response to this message
    pub fn reply(&self, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            from: self.to.clone(),
            to: self.from.clone(),
            content: content.into(),
            timestamp: Utc::now(),
            in_reply_to: Some(self.id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new("AgentA", "AgentB", "Hello");
        assert_eq!(msg.from, "AgentA");
        assert_eq!(msg.to, "AgentB");
        assert_eq!(msg.content, "Hello");
        assert!(msg.in_reply_to.is_none());
    }

    #[test]
    fn test_message_reply() {
        let original = Message::new("AgentA", "AgentB", "Hello");
        let reply = original.reply("Hi back!");

        assert_eq!(reply.from, "AgentB");
        assert_eq!(reply.to, "AgentA");
        assert_eq!(reply.content, "Hi back!");
        assert_eq!(reply.in_reply_to, Some(original.id));
    }
}
