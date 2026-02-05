use crate::message::Message;
use std::collections::HashMap;

/// A conversation between two agents
/// Conversations are stored per agent-pair, not per direction
#[derive(Debug, Default)]
pub struct Conversation {
    messages: Vec<Message>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

/// Storage for all conversations in the system
/// Key is a normalized pair (alphabetically sorted) so (A,B) and (B,A) map to same conversation
#[derive(Debug, Default)]
pub struct ConversationStore {
    conversations: HashMap<(String, String), Conversation>,
}

impl ConversationStore {
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
        }
    }

    /// Get the normalized key for an agent pair (alphabetically sorted)
    fn normalize_key(agent1: &str, agent2: &str) -> (String, String) {
        if agent1 <= agent2 {
            (agent1.to_string(), agent2.to_string())
        } else {
            (agent2.to_string(), agent1.to_string())
        }
    }

    /// Get or create a conversation between two agents
    pub fn get_or_create(&mut self, agent1: &str, agent2: &str) -> &mut Conversation {
        let key = Self::normalize_key(agent1, agent2);
        self.conversations.entry(key).or_insert_with(Conversation::new)
    }

    /// Get a conversation between two agents (if it exists)
    pub fn get(&self, agent1: &str, agent2: &str) -> Option<&Conversation> {
        let key = Self::normalize_key(agent1, agent2);
        self.conversations.get(&key)
    }

    /// Add a message and store it in the appropriate conversation
    pub fn add_message(&mut self, message: Message) {
        let conversation = self.get_or_create(&message.from, &message.to);
        conversation.add_message(message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_storage() {
        let mut store = ConversationStore::new();

        // Messages in both directions should go to same conversation
        let msg1 = Message::new("Alice", "Bob", "Hello Bob");
        let msg2 = Message::new("Bob", "Alice", "Hi Alice");

        store.add_message(msg1);
        store.add_message(msg2);

        // Both orderings should return the same conversation
        let conv1 = store.get("Alice", "Bob").unwrap();
        let conv2 = store.get("Bob", "Alice").unwrap();

        assert_eq!(conv1.len(), 2);
        assert_eq!(conv2.len(), 2);
    }

    #[test]
    fn test_separate_conversations() {
        let mut store = ConversationStore::new();

        store.add_message(Message::new("A", "B", "msg1"));
        store.add_message(Message::new("A", "C", "msg2"));

        // A-B and A-C should be separate conversations
        assert_eq!(store.get("A", "B").unwrap().len(), 1);
        assert_eq!(store.get("A", "C").unwrap().len(), 1);
    }
}
