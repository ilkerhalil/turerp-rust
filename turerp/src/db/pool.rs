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
        Migration {
            version: "006_settings",
            sql: include_str!("../../migrations/006_settings.sql"),
        },
        Migration {
            version: "007_soft_delete",
            sql: include_str!("../../migrations/007_soft_delete.sql"),
        },
        Migration {
            version: "008_custom_fields",
            sql: include_str!("../../migrations/008_custom_fields.sql"),
        },
        Migration {
            version: "009_chart_of_accounts",
            sql: include_str!("../../migrations/009_chart_of_accounts.sql"),
        },
        Migration {
            version: "010_webhooks",
            sql: include_str!("../../migrations/010_webhooks.sql"),
        },
        Migration {
            version: "011_edefter",
            sql: include_str!("../../migrations/011_edefter.sql"),
        },
        Migration {
            version: "012_tax_engine",
            sql: include_str!("../../migrations/012_tax_engine.sql"),
        },
        Migration {
            version: "013_efatura",
            sql: include_str!("../../migrations/013_efatura.sql"),
        },
        Migration {
            version: "014_api_keys",
            sql: include_str!("../../migrations/014_api_keys.sql"),
        },
        Migration {
            version: "015_currency",
            sql: include_str!("../../migrations/015_currency.sql"),
        },
        Migration {
            version: "015_mfa",
            sql: include_str!("../../migrations/015_mfa.sql"),
        },
        Migration {
            version: "016_full_text_search",
            sql: include_str!("../../migrations/016_full_text_search.sql"),
        },
        Migration {
            version: "017_notifications",
            sql: include_str!("../../migrations/017_notifications.sql"),
        },
        Migration {
            version: "018_jobs",
            sql: include_str!("../../migrations/018_jobs.sql"),
        },
        Migration {
            version: "019_soft_delete_users_tenants",
            sql: include_str!("../../migrations/019_soft_delete_users_tenants.sql"),
        },
        Migration {
            version: "020_soft_delete_complete",
            sql: include_str!("../../migrations/020_soft_delete_complete.sql"),
        },
        Migration {
            version: "021_files_table",
            sql: include_str!("../../migrations/021_files_table.sql"),
        },
        Migration {
            version: "021_outbox",
            sql: include_str!("../../migrations/021_outbox.sql"),
        },
        Migration {
            version: "022_cdc_triggers",
            sql: include_str!("../../migrations/022_cdc_triggers.sql"),
        },
        Migration {
            version: "023_companies",
            sql: include_str!("../../migrations/023_companies.sql"),
        },
        Migration {
            version: "023_cost_centers",
            sql: include_str!("../../migrations/023_cost_centers.sql"),
        },
        Migration {
            version: "024_workflows",
            sql: include_str!("../../migrations/024_workflows.sql"),
        },
        Migration {
            version: "025_bank_integration",
            sql: include_str!("../../migrations/025_bank_integration.sql"),
        },
        Migration {
            version: "026_subscriptions",
            sql: include_str!("../../migrations/026_subscriptions.sql"),
        },
        Migration {
            version: "027_observability",
            sql: include_str!("../../migrations/027_observability.sql"),
        },
        Migration {
            version: "028_missing_repos",
            sql: include_str!("../../migrations/028_missing_repos.sql"),
        },
        Migration {
            version: "029_brute_force_protection",
            sql: include_str!("../../migrations/029_brute_force_protection.sql"),
        },
        Migration {
            version: "030_ldap_configs",
            sql: include_str!("../../migrations/030_ldap_configs.sql"),
        },
        Migration {
            version: "031_sgk_tables",
            sql: include_str!("../../migrations/031_sgk_tables.sql"),
        },
        Migration {
            version: "032_blockchain_tables",
            sql: include_str!("../../migrations/032_blockchain_tables.sql"),
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
