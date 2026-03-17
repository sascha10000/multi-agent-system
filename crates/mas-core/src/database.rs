//! Database configuration types for SQL database connections
//!
//! Databases are SQL endpoints that agents can send queries to.
//! The agent sends a SQL query as a message, and the database executes it
//! and returns the results as a CSV-formatted string.
//!
//! # Example Configuration
//!
//! ```json
//! {
//!   "name": "AppDB",
//!   "description": "Application database with user and order data",
//!   "connection_string": "sqlite://app.db",
//!   "database_type": "sqlite",
//!   "read_only": true,
//!   "max_connections": 5,
//!   "timeout_secs": 30
//! }
//! ```

use serde::{Deserialize, Serialize};

/// Supported database types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    #[default]
    Sqlite,
    Postgres,
    Mysql,
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseType::Sqlite => write!(f, "sqlite"),
            DatabaseType::Postgres => write!(f, "postgres"),
            DatabaseType::Mysql => write!(f, "mysql"),
        }
    }
}

/// Complete configuration for a database connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Unique name for the database (used for routing)
    pub name: String,

    /// Human-readable description of the database
    #[serde(default)]
    pub description: String,

    /// Connection string (e.g., "sqlite://data.db", "postgres://user:pass@host/db")
    pub connection_string: String,

    /// Database type hint (auto-detected from connection string if omitted)
    #[serde(default)]
    pub database_type: DatabaseType,

    /// Maximum number of connections in the pool
    #[serde(default)]
    pub max_connections: Option<u32>,

    /// Query timeout in seconds
    #[serde(default)]
    pub timeout_secs: Option<u64>,

    /// Safety: only allow SELECT/WITH queries when true
    #[serde(default)]
    pub read_only: Option<bool>,
}

impl DatabaseConfig {
    pub fn new(name: impl Into<String>, connection_string: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            connection_string: connection_string.into(),
            database_type: DatabaseType::default(),
            max_connections: None,
            timeout_secs: None,
            read_only: Some(true),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = Some(read_only);
        self
    }
}

/// Runtime representation of a database (config + any runtime state)
#[derive(Debug, Clone)]
pub struct Database {
    pub config: DatabaseConfig,
}

impl Database {
    pub fn new(config: DatabaseConfig) -> Self {
        Self { config }
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn description(&self) -> &str {
        &self.config.description
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_database_config() {
        let json = r#"{
            "name": "AppDB",
            "description": "Application database",
            "connection_string": "sqlite://app.db",
            "database_type": "sqlite",
            "read_only": true,
            "max_connections": 5,
            "timeout_secs": 30
        }"#;

        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "AppDB");
        assert_eq!(config.database_type, DatabaseType::Sqlite);
        assert_eq!(config.read_only, Some(true));
        assert_eq!(config.max_connections, Some(5));
    }

    #[test]
    fn test_parse_minimal_database_config() {
        let json = r#"{
            "name": "DB",
            "connection_string": "postgres://localhost/mydb"
        }"#;

        let config: DatabaseConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "DB");
        assert_eq!(config.database_type, DatabaseType::Sqlite); // default
        assert!(config.read_only.is_none());
    }

    #[test]
    fn test_database_type_display() {
        assert_eq!(DatabaseType::Sqlite.to_string(), "sqlite");
        assert_eq!(DatabaseType::Postgres.to_string(), "postgres");
        assert_eq!(DatabaseType::Mysql.to_string(), "mysql");
    }
}
