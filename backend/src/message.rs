/// Message struct for agent communication
#[derive(Debug, Clone)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub content: String,
}

impl Message {
    /// Creates a new message
    pub fn new(from: String, to: String, content: String) -> Self {
        Message { from, to, content }
    }
}
