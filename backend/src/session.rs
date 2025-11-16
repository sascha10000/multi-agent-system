use crate::message::Message;
use std::collections::VecDeque;
use std::time::SystemTime;

/// Represents a single entry in a session (message + optional response)
#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub message: Message,
    pub response: Option<String>,
    pub timestamp: SystemTime,
}

impl SessionEntry {
    /// Creates a new session entry with a message
    pub fn new(message: Message) -> Self {
        SessionEntry {
            message,
            response: None,
            timestamp: SystemTime::now(),
        }
    }

    /// Creates a new session entry with a message and response
    pub fn with_response(message: Message, response: String) -> Self {
        SessionEntry {
            message,
            response: Some(response),
            timestamp: SystemTime::now(),
        }
    }

    /// Sets the response for this session entry
    pub fn set_response(&mut self, response: String) {
        self.response = Some(response);
    }
}

/// Represents a session containing all messages and responses
#[derive(Debug)]
pub struct Session {
    pub id: String,
    entries: Vec<SessionEntry>,
    message_stack: VecDeque<Message>,
    created_at: SystemTime,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Session {
            id: self.id.clone(),
            entries: self.entries.clone(),
            message_stack: self.message_stack.clone(),
            created_at: self.created_at,
            join_handle: None, // JoinHandle cannot be cloned
        }
    }
}

impl Session {
    /// Creates a new session
    pub fn new(id: String) -> Self {
        Session {
            id,
            entries: Vec::new(),
            message_stack: VecDeque::new(),
            created_at: SystemTime::now(),
            join_handle: None,
        }
    }

    /// Adds a message to the session
    pub fn add_message(&mut self, message: Message) {
        let entry = SessionEntry::new(message);
        self.entries.push(entry);
    }

    /// Adds a message with a response to the session
    pub fn add_message_with_response(&mut self, message: Message, response: String) {
        let entry = SessionEntry::with_response(message, response);
        self.entries.push(entry);
    }

    /// Updates the last entry with a response
    pub fn set_last_response(&mut self, response: String) {
        if let Some(last_entry) = self.entries.last_mut() {
            last_entry.set_response(response);
        }
    }

    /// Gets all entries in the session
    pub fn get_entries(&self) -> &[SessionEntry] {
        &self.entries
    }

    /// Gets the number of entries in the session
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Gets the session creation time
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }

    /// Clears all entries from the session
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Pushes a message onto the message stack
    pub fn push_message_to_stack(&mut self, message: Message) {
        self.message_stack.push_back(message);
    }

    /// Pops the oldest message from the stack
    pub fn pop_message_from_stack(&mut self) -> Option<Message> {
        self.message_stack.pop_front()
    }

    /// Checks if the message stack is empty
    pub fn is_message_stack_empty(&self) -> bool {
        self.message_stack.is_empty()
    }

    /// Gets the number of messages in the stack
    pub fn message_stack_size(&self) -> usize {
        self.message_stack.len()
    }

    /// Sets the join handle for the session's processing task
    pub fn set_join_handle(&mut self, handle: tokio::task::JoinHandle<()>) {
        self.join_handle = Some(handle);
    }

    /// Takes the join handle, leaving None in its place
    pub fn take_join_handle(&mut self) -> Option<tokio::task::JoinHandle<()>> {
        self.join_handle.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-session".to_string());
        assert_eq!(session.id, "test-session");
        assert_eq!(session.entry_count(), 0);
    }

    #[test]
    fn test_add_message() {
        let mut session = Session::new("test-session".to_string());
        let message = Message::new(
            "Agent1".to_string(),
            "Agent2".to_string(),
            "Hello".to_string(),
        );

        session.add_message(message);
        assert_eq!(session.entry_count(), 1);
    }

    #[test]
    fn test_add_message_with_response() {
        let mut session = Session::new("test-session".to_string());
        let message = Message::new(
            "Agent1".to_string(),
            "Agent2".to_string(),
            "Hello".to_string(),
        );

        session.add_message_with_response(message, "Hi there!".to_string());
        assert_eq!(session.entry_count(), 1);

        let entries = session.get_entries();
        assert!(entries[0].response.is_some());
        assert_eq!(entries[0].response.as_ref().unwrap(), "Hi there!");
    }

    #[test]
    fn test_set_last_response() {
        let mut session = Session::new("test-session".to_string());
        let message = Message::new(
            "Agent1".to_string(),
            "Agent2".to_string(),
            "Hello".to_string(),
        );

        session.add_message(message);
        assert!(session.get_entries()[0].response.is_none());

        session.set_last_response("Response!".to_string());
        assert!(session.get_entries()[0].response.is_some());
        assert_eq!(
            session.get_entries()[0].response.as_ref().unwrap(),
            "Response!"
        );
    }
}
