//! Database connection pool

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::DatabaseConfig;
use crate::error::ApiError;

/// Create a PostgreSQL connection pool
pub async fn create_pool(config: &DatabaseConfig) -> Result<PgPool, ApiError> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect(&config.url)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to create connection pool: {}", e)))
}

/// Run database migrations
pub async fn run_migrations(pool: &PgPool) -> Result<(), ApiError> {
    // For now, we'll use embedded SQL migrations
    // In production, consider using sqlx-cli or a migration tool

    sqlx::query(include_str!("../../migrations/001_initial_schema.sql"))
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to run migrations: {}", e)))?;

    Ok(())
}
