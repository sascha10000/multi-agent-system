//! Database initialization and migration

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use tracing::info;

/// Create a SQLite connection pool and run migrations
pub async fn create_pool(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(db_path).parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }

    let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    info!("Connected to SQLite database at {}", db_path);

    Ok(pool)
}

/// Run database migrations
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Read and execute the migration file
    let migration_sql = include_str!("../migrations/001_initial.sql");

    sqlx::raw_sql(migration_sql).execute(pool).await?;

    info!("Database migrations applied successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_in_memory_pool() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        // Verify tables exist
        let result: (i64,) =
            sqlx::query_as("SELECT count(*) FROM sqlite_master WHERE type='table' AND name='users'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(result.0, 1);
    }
}
