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

/// Migration version and SQL content.
struct Migration {
    version: &'static str,
    sql: &'static str,
}

/// Run database migrations inside a transaction with idempotency tracking.
pub async fn run_migrations(pool: &PgPool) -> Result<(), ApiError> {
    const MIGRATIONS: &[Migration] = &[
        Migration {
            version: "001_initial_schema",
            sql: include_str!("../../migrations/001_initial_schema.sql"),
        },
        Migration {
            version: "002_add_tenant_db_name",
            sql: include_str!("../../migrations/002_add_tenant_db_name.sql"),
        },
        Migration {
            version: "003_business_modules",
            sql: include_str!("../../migrations/003_business_modules.sql"),
        },
        Migration {
            version: "004_composite_indexes",
            sql: include_str!("../../migrations/004_composite_indexes.sql"),
        },
        Migration {
            version: "005_audit_logs",
            sql: include_str!("../../migrations/005_audit_logs.sql"),
        },
    ];

    // Ensure migrations tracking table exists (outside transaction).
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS __migrations (
            version VARCHAR(64) PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| ApiError::Database(format!("Failed to create migrations table: {}", e)))?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ApiError::Database(format!("Failed to start migration transaction: {}", e)))?;

    for mig in MIGRATIONS {
        let already_applied: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM __migrations WHERE version = $1)")
                .bind(mig.version)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| {
                    ApiError::Database(format!("Failed to check migration {}: {}", mig.version, e))
                })?;

        if already_applied {
            tracing::info!("Migration {} already applied, skipping", mig.version);
            continue;
        }

        sqlx::query(mig.sql).execute(&mut *tx).await.map_err(|e| {
            ApiError::Database(format!("Failed to run migration {}: {}", mig.version, e))
        })?;

        sqlx::query("INSERT INTO __migrations (version) VALUES ($1)")
            .bind(mig.version)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                ApiError::Database(format!("Failed to record migration {}: {}", mig.version, e))
            })?;

        tracing::info!("Migration {} applied successfully", mig.version);
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::Database(format!("Failed to commit migrations: {}", e)))?;

    Ok(())
}
