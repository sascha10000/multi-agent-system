//! Message tracing for debugging and verbose mode
//!
//! Provides a mechanism to capture agent-to-agent communications
//! for display in verbose/debug mode.

use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A single trace event capturing an agent communication
#[derive(Debug, Clone, Serialize)]
pub struct TraceEvent {
    /// The agent sending the message
    pub from: String,
    /// The agent receiving the message
    pub to: String,
    /// The message content (truncated if too long)
    pub content: String,
    /// Type of event
    pub event_type: TraceEventType,
}

/// Types of trace events
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TraceEventType {
    /// Initial request from user/API
    Request,
    /// Response from an agent
    Response,
    /// Message forwarded to another agent
    Forward,
    /// Synthesized response combining multiple agent responses
    Synthesis,
}

impl TraceEvent {
    /// Create a new trace event
    pub fn new(from: impl Into<String>, to: impl Into<String>, content: impl Into<String>, event_type: TraceEventType) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            content: content.into(),
            event_type,
        }
    }

    pub fn request(from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(from, to, content, TraceEventType::Request)
    }

    pub fn response(from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(from, to, content, TraceEventType::Response)
    }

    pub fn forward(from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(from, to, content, TraceEventType::Forward)
    }

    pub fn synthesis(from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(from, to, content, TraceEventType::Synthesis)
    }
}

/// Collector for trace events
///
/// This is a thread-safe collector that can be shared across async tasks.
/// Clone it to share between tasks - all clones share the same underlying storage.
#[derive(Debug, Clone, Default)]
pub struct TraceCollector {
    events: Arc<RwLock<Vec<TraceEvent>>>,
}

impl TraceCollector {
    /// Create a new empty trace collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a trace event
    pub async fn record(&self, event: TraceEvent) {
        let mut events = self.events.write().await;
        events.push(event);
    }

    /// Record a request event
    pub async fn record_request(&self, from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) {
        self.record(TraceEvent::request(from, to, content)).await;
    }

    /// Record a response event
    pub async fn record_response(&self, from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) {
        self.record(TraceEvent::response(from, to, content)).await;
    }

    /// Record a forward event
    pub async fn record_forward(&self, from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) {
        self.record(TraceEvent::forward(from, to, content)).await;
    }

    /// Record a synthesis event
    pub async fn record_synthesis(&self, from: impl Into<String>, to: impl Into<String>, content: impl Into<String>) {
        self.record(TraceEvent::synthesis(from, to, content)).await;
    }

    /// Get all collected events
    pub async fn events(&self) -> Vec<TraceEvent> {
        let events = self.events.read().await;
        events.clone()
    }

    /// Clear all events
    pub async fn clear(&self) {
        let mut events = self.events.write().await;
        events.clear();
    }

    /// Check if any events have been recorded
    pub async fn is_empty(&self) -> bool {
        let events = self.events.read().await;
        events.is_empty()
    }
}
