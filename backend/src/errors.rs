use std::{
    error::Error,
    fmt::{Display, Formatter},
};

#[derive(Debug)]
pub enum AgentError {
    NotFound(String),
    Exists(String),
    NotConnected(String, String),
    NoActiveSession(String),
}

impl Display for AgentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            AgentError::NotFound(name) => write!(f, "Agent {} not found!", name),
            AgentError::Exists(name) => write!(f, "Agent {} already exists!", name),
            AgentError::NotConnected(from, to) => {
                write!(f, "Agent {} not connected to Agent {}", from, to)
            }
            AgentError::NoActiveSession(name) => {
                write!(f, "No active session for agent '{}'", name)
            }
        }
    }
}

impl From<AgentError> for String {
    fn from(value: AgentError) -> Self {
        format!("{:}", value)
    }
}

impl Error for AgentError {}

#[derive(Debug)]
pub enum SessionError {
    NotFound(String),
    Stopped(String),
    Running(String),
    Exists(String),
}

impl Display for SessionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            SessionError::NotFound(id) => write!(f, "Session {} not found!", id),
            SessionError::Stopped(id) => write!(f, "Session {} stopped!", id),
            SessionError::Running(id) => write!(f, "Session {} still running!", id),
            SessionError::Exists(id) => write!(f, "Session {} already exists!", id),
        }
    }
}

impl From<SessionError> for String {
    fn from(value: SessionError) -> Self {
        format!("{:}", value)
    }
}

impl Error for SessionError {}
