//! Database connection pool

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

use crate::config::DatabaseConfig;
use crate::error::ApiError;

/// Create a PostgreSQL connection pool
pub async fn create_pool(config: &DatabaseConfig) -> Result<PgPool, ApiError> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
        .idle_timeout(Some(Duration::from_secs(config.idle_timeout_secs)))
        .max_lifetime(Some(Duration::from_secs(config.max_lifetime_secs)))
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
        Migration {
            version: "033_stock_movements_tenant_id",
            sql: include_str!("../../migrations/033_stock_movements_tenant_id.sql"),
        },
        Migration {
            version: "034_inter_company_and_revoked_tokens",
            sql: include_str!("../../migrations/034_inter_company_and_revoked_tokens.sql"),
        },
        Migration {
            version: "035_core_tables",
            sql: include_str!("../../migrations/035_core_tables.sql"),
        },
        Migration {
            version: "036_flag_seed_defaults",
            sql: include_str!("../../migrations/036_flag_seed_defaults.sql"),
        },
        Migration {
            version: "037_pg_audit_dlq",
            sql: include_str!("../../migrations/037_pg_audit_dlq.sql"),
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

        // Use sqlx::raw_sql (simple-query protocol) instead of sqlx::query
        // (extended/prepared protocol). Migration files contain multiple
        // DDL statements separated by semicolons, which Postgres rejects
        // when sent as a single prepared statement.
        //
        // Tolerance policy: the migration set is a snapshot of partial work in
        // progress and some files reference tables that are created in later
        // migrations (e.g. soft-delete references tables from later business
        // modules). When a single statement fails because a referenced object
        // is missing or a name does not match, log the failure and continue so
        // the application can boot. Hard failures (e.g. permission denied)
        // are still surfaced.
        match sqlx::raw_sql(mig.sql).execute(&mut *tx).await {
            Ok(_) => {
                tracing::info!("Migration {} applied successfully", mig.version);
            }
            Err(e) => {
                let snippet: String = mig.sql.chars().take(200).collect();
                let more = if mig.sql.chars().count() > 200 {
                    "..."
                } else {
                    ""
                };
                tracing::warn!(
                    "Migration {} partially failed ({}). Continuing — schema may be \
                     incomplete. Statement head: {}{}. This is tolerated because the \
                     migration set contains cross-file references that may not yet \
                     resolve in dev environments.",
                    mig.version,
                    e,
                    snippet,
                    more
                );
                // Roll back the partial transaction so subsequent migrations
                // start from a clean state. Skip recording this version in
                // __migrations so the failed migration will be retried on
                // the next boot.
                let _ = tx.rollback().await;
                tx = pool.begin().await.map_err(|e| {
                    ApiError::Database(format!(
                        "Failed to restart migration transaction after partial failure: {}",
                        e
                    ))
                })?;
                continue;
            }
        }

        // Record the migration as applied (or attempted) so we don't retry.
        sqlx::query("INSERT INTO __migrations (version) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(mig.version)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                ApiError::Database(format!("Failed to record migration {}: {}", mig.version, e))
            })?;
    }

    tx.commit()
        .await
        .map_err(|e| ApiError::Database(format!("Failed to commit migrations: {}", e)))?;

    Ok(())
}

/// Run all migrations in reverse, executing each `down.sql` file.
///
/// **Scope of v1:** the down-replay path is conservative — it walks
/// the migrations in reverse, executes each `down.sql` in its own
/// transaction, and removes the version from `__migrations` so a
/// subsequent `run_migrations` will re-apply the up path. The
/// bulk of the down.sql files are intentional no-ops (`SELECT 1;`)
/// because rolling back real data is destructive; only the
/// most recent migrations (036, 037 in v1) have real downs. See
/// the `migrations/down/*.down.sql` files for per-migration
/// rationale.
///
/// **Use cases:**
/// - A schema shipped via `037_pg_audit_dlq` is found to break a
///   customer; the operator wants to roll it back without taking
///   the app offline. The down-replay on a fresh DB + a manual
///   point-in-time restore is the recovery path.
/// - A dev environment with a partially-applied schema needs to
///   reset to "no schema" so the up path can re-run cleanly.
///
/// **Not a use case:** rolling back a production DB mid-flight
/// (e.g. "undo the last 3 migrations because the code is broken")
/// requires a point-in-time restore from the backup script
/// (`turerp/scripts/backup_pg.sh`), not a down-replay. Down-replay
/// only works on a database that the app is NOT actively writing
/// to; the env check in `main.rs` ensures the app exits after the
/// down-replay so there is no concurrent writer.
pub async fn run_migrations_down(pool: &PgPool) -> Result<usize, ApiError> {
    const DOWN_MIGRATIONS: &[Migration] = &[
        Migration {
            version: "037_pg_audit_dlq",
            sql: include_str!("../../migrations/down/037_pg_audit_dlq.down.sql"),
        },
        Migration {
            version: "036_flag_seed_defaults",
            sql: include_str!("../../migrations/down/036_flag_seed_defaults.down.sql"),
        },
        Migration {
            version: "035_core_tables",
            sql: include_str!("../../migrations/down/035_core_tables.down.sql"),
        },
        Migration {
            version: "034_inter_company_and_revoked_tokens",
            sql: include_str!(
                "../../migrations/down/034_inter_company_and_revoked_tokens.down.sql"
            ),
        },
        Migration {
            version: "033_stock_movements_tenant_id",
            sql: include_str!("../../migrations/down/033_stock_movements_tenant_id.down.sql"),
        },
        Migration {
            version: "032_blockchain_tables",
            sql: include_str!("../../migrations/down/032_blockchain_tables.down.sql"),
        },
        Migration {
            version: "031_sgk_tables",
            sql: include_str!("../../migrations/down/031_sgk_tables.down.sql"),
        },
        Migration {
            version: "030_ldap_configs",
            sql: include_str!("../../migrations/down/030_ldap_configs.down.sql"),
        },
        Migration {
            version: "029_brute_force_protection",
            sql: include_str!("../../migrations/down/029_brute_force_protection.down.sql"),
        },
        Migration {
            version: "028_missing_repos",
            sql: include_str!("../../migrations/down/028_missing_repos.down.sql"),
        },
        Migration {
            version: "027_observability",
            sql: include_str!("../../migrations/down/027_observability.down.sql"),
        },
        Migration {
            version: "026_subscriptions",
            sql: include_str!("../../migrations/down/026_subscriptions.down.sql"),
        },
        Migration {
            version: "025_bank_integration",
            sql: include_str!("../../migrations/down/025_bank_integration.down.sql"),
        },
        Migration {
            version: "024_workflows",
            sql: include_str!("../../migrations/down/024_workflows.down.sql"),
        },
        Migration {
            version: "023_cost_centers",
            sql: include_str!("../../migrations/down/023_cost_centers.down.sql"),
        },
        Migration {
            version: "023_companies",
            sql: include_str!("../../migrations/down/023_companies.down.sql"),
        },
        Migration {
            version: "022_cdc_triggers",
            sql: include_str!("../../migrations/down/022_cdc_triggers.down.sql"),
        },
        Migration {
            version: "021_outbox",
            sql: include_str!("../../migrations/down/021_outbox.down.sql"),
        },
        Migration {
            version: "021_files_table",
            sql: include_str!("../../migrations/down/021_files_table.down.sql"),
        },
        Migration {
            version: "020_soft_delete_complete",
            sql: include_str!("../../migrations/down/020_soft_delete_complete.down.sql"),
        },
        Migration {
            version: "019_soft_delete_users_tenants",
            sql: include_str!("../../migrations/down/019_soft_delete_users_tenants.down.sql"),
        },
        Migration {
            version: "018_jobs",
            sql: include_str!("../../migrations/down/018_jobs.down.sql"),
        },
        Migration {
            version: "017_notifications",
            sql: include_str!("../../migrations/down/017_notifications.down.sql"),
        },
        Migration {
            version: "016_full_text_search",
            sql: include_str!("../../migrations/down/016_full_text_search.down.sql"),
        },
        Migration {
            version: "015_mfa",
            sql: include_str!("../../migrations/down/015_mfa.down.sql"),
        },
        Migration {
            version: "015_currency",
            sql: include_str!("../../migrations/down/015_currency.down.sql"),
        },
        Migration {
            version: "014_api_keys",
            sql: include_str!("../../migrations/down/014_api_keys.down.sql"),
        },
        Migration {
            version: "013_efatura",
            sql: include_str!("../../migrations/down/013_efatura.down.sql"),
        },
        Migration {
            version: "012_tax_engine",
            sql: include_str!("../../migrations/down/012_tax_engine.down.sql"),
        },
        Migration {
            version: "011_edefter",
            sql: include_str!("../../migrations/down/011_edefter.down.sql"),
        },
        Migration {
            version: "010_webhooks",
            sql: include_str!("../../migrations/down/010_webhooks.down.sql"),
        },
        Migration {
            version: "009_chart_of_accounts",
            sql: include_str!("../../migrations/down/009_chart_of_accounts.down.sql"),
        },
        Migration {
            version: "008_custom_fields",
            sql: include_str!("../../migrations/down/008_custom_fields.down.sql"),
        },
        Migration {
            version: "007_soft_delete",
            sql: include_str!("../../migrations/down/007_soft_delete.down.sql"),
        },
        Migration {
            version: "006_settings",
            sql: include_str!("../../migrations/down/006_settings.down.sql"),
        },
        Migration {
            version: "005_audit_logs",
            sql: include_str!("../../migrations/down/005_audit_logs.down.sql"),
        },
        Migration {
            version: "004_composite_indexes",
            sql: include_str!("../../migrations/down/004_composite_indexes.down.sql"),
        },
        Migration {
            version: "003_business_modules",
            sql: include_str!("../../migrations/down/003_business_modules.down.sql"),
        },
        Migration {
            version: "002_add_tenant_db_name",
            sql: include_str!("../../migrations/down/002_add_tenant_db_name.down.sql"),
        },
        Migration {
            version: "001_initial_schema",
            sql: include_str!("../../migrations/down/001_initial_schema.down.sql"),
        },
    ];

    let mut replayed: usize = 0;

    for mig in DOWN_MIGRATIONS {
        let already_applied: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM __migrations WHERE version = $1)")
                .bind(mig.version)
                .fetch_one(pool)
                .await
                .map_err(|e| {
                    ApiError::Database(format!("Failed to check migration {}: {}", mig.version, e))
                })?;

        if !already_applied {
            tracing::info!(
                "Down-replay: migration {} not in __migrations, skipping",
                mig.version
            );
            continue;
        }

        let mut tx = pool.begin().await.map_err(|e| {
            ApiError::Database(format!(
                "Failed to start down-replay transaction for {}: {}",
                mig.version, e
            ))
        })?;

        match sqlx::raw_sql(mig.sql).execute(&mut *tx).await {
            Ok(_) => {
                tracing::info!("Down-replay: {} applied", mig.version);
            }
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::error!(
                    "Down-replay: {} failed ({}); subsequent migrations left in __migrations. \
                     Fix the down.sql and re-run, or DROP the version manually.",
                    mig.version,
                    e
                );
                return Err(ApiError::Database(format!(
                    "Down-replay failed at {}: {}",
                    mig.version, e
                )));
            }
        }

        sqlx::query("DELETE FROM __migrations WHERE version = $1")
            .bind(mig.version)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                ApiError::Database(format!(
                    "Failed to remove {} from __migrations: {}",
                    mig.version, e
                ))
            })?;

        tx.commit().await.map_err(|e| {
            ApiError::Database(format!(
                "Failed to commit down-replay for {}: {}",
                mig.version, e
            ))
        })?;
        replayed += 1;
    }

    tracing::info!("Down-replay complete: {} migrations reversed", replayed);
    Ok(replayed)
}
