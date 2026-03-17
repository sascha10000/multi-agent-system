//! Database handler that executes SQL queries and returns CSV-formatted results
//!
//! This handler implements `MessageHandler` so it can be registered in the agent system.
//! When an agent sends a message to a database node, the message content is treated as
//! a SQL query. The handler executes the query and returns results as CSV.

use crate::agent::Agent;
use crate::agent_system::MessageHandler;
use crate::database::Database;
use crate::message::Message;

use async_trait::async_trait;
use sqlx::any::AnyRow;
use sqlx::{Column, Pool, Row, any::Any};
use std::sync::Arc;
use tracing::{debug, warn};

/// Handler that executes SQL queries against a database pool
pub struct DatabaseHandler {
    database: Arc<Database>,
    pool: Pool<Any>,
}

impl DatabaseHandler {
    /// Create a new database handler with an active connection pool
    pub async fn new(database: Arc<Database>) -> Result<Self, String> {
        // Install the default drivers for sqlx::Any
        sqlx::any::install_default_drivers();

        let max_conns = database.config.max_connections.unwrap_or(5);

        let pool = sqlx::pool::PoolOptions::<Any>::new()
            .max_connections(max_conns)
            .connect(&database.config.connection_string)
            .await
            .map_err(|e| format!("Failed to connect to database '{}': {}", database.name(), e))?;

        debug!(
            "Connected to database '{}' (max_connections={})",
            database.name(),
            max_conns
        );

        Ok(Self { database, pool })
    }

    /// Format query results as CSV
    fn format_as_csv(columns: &[String], rows: &[Vec<String>]) -> String {
        let mut output = String::new();

        // Header row
        for (i, col) in columns.iter().enumerate() {
            if i > 0 {
                output.push(',');
            }
            output.push_str(&Self::csv_escape(col));
        }

        // Data rows
        for row in rows {
            output.push('\n');
            for (i, val) in row.iter().enumerate() {
                if i > 0 {
                    output.push(',');
                }
                output.push_str(&Self::csv_escape(val));
            }
        }

        output
    }

    /// Escape a value for CSV (RFC 4180)
    fn csv_escape(value: &str) -> String {
        if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r')
        {
            format!("\"{}\"", value.replace('"', "\"\""))
        } else {
            value.to_string()
        }
    }

    /// Extract a column value from an AnyRow as a String
    fn extract_value(row: &AnyRow, index: usize) -> String {
        // Try types in order of likelihood
        if let Ok(v) = row.try_get::<String, _>(index) {
            return v;
        }
        if let Ok(v) = row.try_get::<i64, _>(index) {
            return v.to_string();
        }
        if let Ok(v) = row.try_get::<i32, _>(index) {
            return v.to_string();
        }
        if let Ok(v) = row.try_get::<f64, _>(index) {
            return v.to_string();
        }
        if let Ok(v) = row.try_get::<f32, _>(index) {
            return v.to_string();
        }
        if let Ok(v) = row.try_get::<bool, _>(index) {
            return v.to_string();
        }
        // Check for NULL
        if let Ok(v) = row.try_get::<Option<String>, _>(index) {
            return v.unwrap_or_else(|| "NULL".to_string());
        }
        if let Ok(v) = row.try_get::<Option<i64>, _>(index) {
            return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".to_string());
        }
        if let Ok(v) = row.try_get::<Option<f64>, _>(index) {
            return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".to_string());
        }
        "NULL".to_string()
    }
}

#[async_trait]
impl MessageHandler for DatabaseHandler {
    async fn handle(&self, message: &Message, _agent: &Agent) -> Option<String> {
        let sql = message.content.trim();

        if sql.is_empty() {
            return Some("Error: Empty SQL query.".to_string());
        }

        // Read-only enforcement
        if self.database.config.read_only.unwrap_or(false) {
            let upper = sql.to_uppercase();
            let first_keyword = upper.split_whitespace().next().unwrap_or("");
            if !matches!(first_keyword, "SELECT" | "WITH" | "EXPLAIN" | "SHOW" | "DESCRIBE" | "PRAGMA") {
                return Some(
                    "Error: This database connection is read-only. Only SELECT, WITH, EXPLAIN, SHOW, DESCRIBE, and PRAGMA queries are allowed."
                        .to_string(),
                );
            }
        }

        debug!(
            "[DB:{}] Executing query: {}",
            self.database.name(),
            &sql[..sql.len().min(200)]
        );

        // Execute the query
        match sqlx::query(sql).fetch_all(&self.pool).await {
            Ok(rows) => {
                if rows.is_empty() {
                    return Some("Query returned 0 rows.".to_string());
                }

                // Extract column names
                let columns: Vec<String> = rows[0]
                    .columns()
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect();

                // Extract values
                let data_rows: Vec<Vec<String>> = rows
                    .iter()
                    .map(|row| {
                        (0..columns.len())
                            .map(|i| Self::extract_value(row, i))
                            .collect()
                    })
                    .collect();

                let row_count = data_rows.len();
                let csv = Self::format_as_csv(&columns, &data_rows);

                debug!(
                    "[DB:{}] Query returned {} rows, {} columns",
                    self.database.name(),
                    row_count,
                    columns.len()
                );

                Some(csv)
            }
            Err(e) => {
                warn!("[DB:{}] SQL Error: {}", self.database.name(), e);
                Some(format!("SQL Error: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_escape_plain() {
        assert_eq!(DatabaseHandler::csv_escape("hello"), "hello");
    }

    #[test]
    fn test_csv_escape_comma() {
        assert_eq!(DatabaseHandler::csv_escape("hello, world"), "\"hello, world\"");
    }

    #[test]
    fn test_csv_escape_quotes() {
        assert_eq!(DatabaseHandler::csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn test_csv_escape_newline() {
        assert_eq!(DatabaseHandler::csv_escape("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn test_format_csv() {
        let columns = vec!["name".to_string(), "age".to_string()];
        let rows = vec![
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];
        let csv = DatabaseHandler::format_as_csv(&columns, &rows);
        assert_eq!(csv, "name,age\nAlice,30\nBob,25");
    }
}
