use std::time::Duration;

/// System-wide configuration for the multi-agent system
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// Default timeout for blocking connections (when no per-connection override exists)
    pub global_timeout: Duration,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            global_timeout: Duration::from_secs(30),
        }
    }
}

impl SystemConfig {
    pub fn new(global_timeout: Duration) -> Self {
        Self { global_timeout }
    }

    /// Create config with timeout in seconds (convenience method)
    pub fn with_timeout_secs(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }
}
