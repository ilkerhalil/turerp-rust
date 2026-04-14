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
    // Embedded SQL migrations run in order
    // In production, consider using sqlx-cli or a migration tool

    sqlx::query(include_str!("../../migrations/001_initial_schema.sql"))
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to run migration 001: {}", e)))?;

    sqlx::query(include_str!("../../migrations/002_add_tenant_db_name.sql"))
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to run migration 002: {}", e)))?;

    sqlx::query(include_str!("../../migrations/003_business_modules.sql"))
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to run migration 003: {}", e)))?;

    sqlx::query(include_str!("../../migrations/004_composite_indexes.sql"))
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to run migration 004: {}", e)))?;

    sqlx::query(include_str!("../../migrations/005_audit_logs.sql"))
        .execute(pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to run migration 005: {}", e)))?;

    Ok(())
}
