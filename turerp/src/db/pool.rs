//! Database connection pool

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

use crate::config::DatabaseConfig;
use crate::error::ApiError;

/// Fixed key for the migration advisory lock (see `run_migrations` /
/// `run_migrations_down`). Packed from the ASCII bytes of `"TURERP"` so it
/// is distinctive and unlikely to collide with any app-level advisory lock
/// (none currently exist in the codebase). A single int8 key is shared by
/// the up and down paths so a concurrent up-boot and down-replay are also
/// mutually exclusive.
const MIGRATION_ADVISORY_LOCK_KEY: i64 = 0x5455_5245_5250; // "TURERP"

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
///
/// By default (`tolerate = false`) the first migration failure aborts boot
/// and returns an error. Set `tolerate = true` only in dev/test environments
/// where the migration snapshot may contain cross-file references that do not
/// yet resolve; in that case failed migrations are rolled back, logged, and
/// retried on the next boot.
pub async fn run_migrations(pool: &PgPool, tolerate: bool) -> Result<(), ApiError> {
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
        Migration {
            version: "038_employees_updated_at_not_null",
            sql: include_str!("../../migrations/038_employees_updated_at_not_null.sql"),
        },
        Migration {
            version: "039_cari_financial_columns_numeric",
            sql: include_str!("../../migrations/039_cari_financial_columns_numeric.sql"),
        },
        Migration {
            version: "040_tax_periods_soft_delete",
            sql: include_str!("../../migrations/040_tax_periods_soft_delete.sql"),
        },
        Migration {
            version: "041_quotations_company_id",
            sql: include_str!("../../migrations/041_quotations_company_id.sql"),
        },
        Migration {
            version: "042_purchase_goods_company_id",
            sql: include_str!("../../migrations/042_purchase_goods_company_id.sql"),
        },
        Migration {
            version: "043_payrolls_company_id",
            sql: include_str!("../../migrations/043_payrolls_company_id.sql"),
        },
        Migration {
            version: "044_reconciliation_rules_soft_delete",
            sql: include_str!("../../migrations/044_reconciliation_rules_soft_delete.sql"),
        },
        Migration {
            version: "045_subscriptions_soft_delete",
            sql: include_str!("../../migrations/045_subscriptions_soft_delete.sql"),
        },
        Migration {
            version: "046_workflow_templates_soft_delete",
            sql: include_str!("../../migrations/046_workflow_templates_soft_delete.sql"),
        },
        Migration {
            version: "047_warehouses_updated_at_and_enable",
            sql: include_str!("../../migrations/047_warehouses_updated_at_and_enable.sql"),
        },
        Migration {
            version: "048_hr_attendance_leave_requests_tenant_id",
            sql: include_str!("../../migrations/048_hr_attendance_leave_requests_tenant_id.sql"),
        },
        Migration {
            version: "049_stock_levels_tenant_id",
            sql: include_str!("../../migrations/049_stock_levels_tenant_id.sql"),
        },
        Migration {
            version: "050_manufacturing_children_tenant_id",
            sql: include_str!("../../migrations/050_manufacturing_children_tenant_id.sql"),
        },
        Migration {
            version: "051_invoice_lines_tenant_id",
            sql: include_str!("../../migrations/051_invoice_lines_tenant_id.sql"),
        },
        Migration {
            version: "052_maintenance_records_tenant_id",
            sql: include_str!("../../migrations/052_maintenance_records_tenant_id.sql"),
        },
        Migration {
            version: "053_journal_lines_tenant_id",
            sql: include_str!("../../migrations/053_journal_lines_tenant_id.sql"),
        },
        Migration {
            version: "054_purchase_order_lines_tenant_id",
            sql: include_str!("../../migrations/054_purchase_order_lines_tenant_id.sql"),
        },
        Migration {
            version: "055_goods_receipt_lines_tenant_id",
            sql: include_str!("../../migrations/055_goods_receipt_lines_tenant_id.sql"),
        },
        Migration {
            version: "056_purchase_request_lines_tenant_id",
            sql: include_str!("../../migrations/056_purchase_request_lines_tenant_id.sql"),
        },
        Migration {
            version: "057_sales_order_lines_tenant_id",
            sql: include_str!("../../migrations/057_sales_order_lines_tenant_id.sql"),
        },
        Migration {
            version: "058_quotation_lines_tenant_id",
            sql: include_str!("../../migrations/058_quotation_lines_tenant_id.sql"),
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

    // Multi-replica guard: serialize migration application across
    // concurrently-booting replicas. Without this, two replicas can both
    // see `already_applied = false` for a NON-idempotent migration (e.g.
    // 048-058 `ADD CONSTRAINT fk_..._tenant` — Postgres has no IF NOT EXISTS
    // for ADD CONSTRAINT): both run the DDL, the loser's tx fails with
    // "constraint already exists", the tolerance policy below rolls it back
    // and continues WITHOUT recording it, so the loser retries on every boot
    // → perpetual warn-on-boot and a migration that is never recorded. With
    // the lock, replica B blocks until replica A finishes, then B finds every
    // migration `already_applied` and skips. We use a TRANSACTION-scoped
    // advisory lock (`pg_advisory_xact_lock`) held in a dedicated outer tx:
    // it auto-releases when that tx commits at the end OR is rolled back on
    // any early-return `?` error path (Drop), so the lock can never leak even
    // if this function aborts mid-run. The per-migration DDL txs below use
    // separate pool connections; the lock is cluster-wide keyed by the int,
    // so it blocks other replicas regardless of which connection they use.
    let mut lock_tx = pool.begin().await.map_err(|e| {
        ApiError::Database(format!("Failed to start migration advisory-lock tx: {}", e))
    })?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(MIGRATION_ADVISORY_LOCK_KEY)
        .execute(&mut *lock_tx)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to acquire migration advisory lock: {}", e))
        })?;

    for mig in MIGRATIONS {
        // Each migration runs in its OWN transaction. A failure rolls back only
        // that migration's partial DDL — prior migrations stay committed. The
        // previous single-transaction design held every migration's DDL and
        // __migrations record in one tx, so when a later migration failed the
        // rollback wiped ALL already-applied migrations (a single bad file took
        // the whole schema down). Per-migration txs make the apply atomic per
        // file and isolate failures.
        let already_applied: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM __migrations WHERE version = $1)")
                .bind(mig.version)
                .fetch_one(pool)
                .await
                .map_err(|e| {
                    ApiError::Database(format!("Failed to check migration {}: {}", mig.version, e))
                })?;

        if already_applied {
            tracing::info!("Migration {} already applied, skipping", mig.version);
            continue;
        }

        let mut tx = pool.begin().await.map_err(|e| {
            ApiError::Database(format!(
                "Failed to start migration transaction for {}: {}",
                mig.version, e
            ))
        })?;

        // raw_sql (simple-query protocol) because migration files contain
        // multiple DDL statements separated by semicolons, which Postgres
        // rejects as a single prepared statement.
        //
        // Tolerance policy (only when `tolerate = true`): the migration set is
        // a snapshot of partial work in progress and some files reference tables
        // created in later migrations. When a statement fails because a referenced
        // object is missing, log and continue so the application can boot. The
        // failed migration is NOT recorded in __migrations, so it retries on the
        // next boot. In production `tolerate` is false: a failed migration aborts
        // boot so the operator is forced to fix the schema rather than running
        // with an incomplete database.
        match sqlx::raw_sql(mig.sql).execute(&mut *tx).await {
            Ok(_) => {
                // Record in the SAME tx as the DDL so the record and the schema
                // change commit atomically: either the migration is fully
                // applied + recorded, or neither.
                sqlx::query(
                    "INSERT INTO __migrations (version) VALUES ($1) ON CONFLICT DO NOTHING",
                )
                .bind(mig.version)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    ApiError::Database(format!("Failed to record migration {}: {}", mig.version, e))
                })?;
                tx.commit().await.map_err(|e| {
                    ApiError::Database(format!("Failed to commit migration {}: {}", mig.version, e))
                })?;
                tracing::info!("Migration {} applied successfully", mig.version);
            }
            Err(e) => {
                let snippet: String = mig.sql.chars().take(200).collect();
                let more = if mig.sql.chars().count() > 200 {
                    "..."
                } else {
                    ""
                };
                // Roll back ONLY this migration's partial work; prior
                // migrations are already committed in their own txs.
                let _ = tx.rollback().await;

                if tolerate {
                    tracing::warn!(
                        "Migration {} partially failed ({}). Continuing — schema may be \
                         incomplete. Statement head: {}{}. Tolerance is enabled; the \
                         migration will retry on the next boot.",
                        mig.version,
                        e,
                        snippet,
                        more
                    );
                    continue;
                }

                tracing::error!(
                    "Migration {} failed ({}). Boot aborted. Statement head: {}{}.",
                    mig.version,
                    e,
                    snippet,
                    more
                );
                return Err(ApiError::Database(format!(
                    "Migration {} failed: {}. Statement head: {}{}",
                    mig.version, e, snippet, more
                )));
            }
        }
    }

    // Release the advisory lock by committing the outer lock tx. On any
    // early-return `?` error path above, `lock_tx` is dropped → rolled back
    // → the xact lock is released there, so this is the only success path.
    let _ = lock_tx.commit().await;

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
            version: "058_quotation_lines_tenant_id",
            sql: include_str!("../../migrations/down/058_quotation_lines_tenant_id.down.sql"),
        },
        Migration {
            version: "057_sales_order_lines_tenant_id",
            sql: include_str!("../../migrations/down/057_sales_order_lines_tenant_id.down.sql"),
        },
        Migration {
            version: "056_purchase_request_lines_tenant_id",
            sql: include_str!(
                "../../migrations/down/056_purchase_request_lines_tenant_id.down.sql"
            ),
        },
        Migration {
            version: "055_goods_receipt_lines_tenant_id",
            sql: include_str!("../../migrations/down/055_goods_receipt_lines_tenant_id.down.sql"),
        },
        Migration {
            version: "054_purchase_order_lines_tenant_id",
            sql: include_str!("../../migrations/down/054_purchase_order_lines_tenant_id.down.sql"),
        },
        Migration {
            version: "053_journal_lines_tenant_id",
            sql: include_str!("../../migrations/down/053_journal_lines_tenant_id.down.sql"),
        },
        Migration {
            version: "052_maintenance_records_tenant_id",
            sql: include_str!("../../migrations/down/052_maintenance_records_tenant_id.down.sql"),
        },
        Migration {
            version: "051_invoice_lines_tenant_id",
            sql: include_str!("../../migrations/down/051_invoice_lines_tenant_id.down.sql"),
        },
        Migration {
            version: "050_manufacturing_children_tenant_id",
            sql: include_str!(
                "../../migrations/down/050_manufacturing_children_tenant_id.down.sql"
            ),
        },
        Migration {
            version: "049_stock_levels_tenant_id",
            sql: include_str!("../../migrations/down/049_stock_levels_tenant_id.down.sql"),
        },
        Migration {
            version: "048_hr_attendance_leave_requests_tenant_id",
            sql: include_str!(
                "../../migrations/down/048_hr_attendance_leave_requests_tenant_id.down.sql"
            ),
        },
        Migration {
            version: "047_warehouses_updated_at_and_enable",
            sql: include_str!(
                "../../migrations/down/047_warehouses_updated_at_and_enable.down.sql"
            ),
        },
        Migration {
            version: "046_workflow_templates_soft_delete",
            sql: include_str!("../../migrations/down/046_workflow_templates_soft_delete.down.sql"),
        },
        Migration {
            version: "045_subscriptions_soft_delete",
            sql: include_str!("../../migrations/down/045_subscriptions_soft_delete.down.sql"),
        },
        Migration {
            version: "044_reconciliation_rules_soft_delete",
            sql: include_str!(
                "../../migrations/down/044_reconciliation_rules_soft_delete.down.sql"
            ),
        },
        Migration {
            version: "043_payrolls_company_id",
            sql: include_str!("../../migrations/down/043_payrolls_company_id.down.sql"),
        },
        Migration {
            version: "042_purchase_goods_company_id",
            sql: include_str!("../../migrations/down/042_purchase_goods_company_id.down.sql"),
        },
        Migration {
            version: "041_quotations_company_id",
            sql: include_str!("../../migrations/down/041_quotations_company_id.down.sql"),
        },
        Migration {
            version: "040_tax_periods_soft_delete",
            sql: include_str!("../../migrations/down/040_tax_periods_soft_delete.down.sql"),
        },
        Migration {
            version: "039_cari_financial_columns_numeric",
            sql: include_str!("../../migrations/down/039_cari_financial_columns_numeric.down.sql"),
        },
        Migration {
            version: "038_employees_updated_at_not_null",
            sql: include_str!("../../migrations/down/038_employees_updated_at_not_null.down.sql"),
        },
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

    // Multi-replica guard (mirror of `run_migrations`): serialize down-replay
    // across concurrently-booting replicas AND against a concurrent up-boot.
    // Down-replay is destructive (drops tenant_id columns/FKs), so two
    // concurrent down-replays racing on the same `__migrations` rows would
    // both `DELETE` + run the same `down.sql`; an up-boot racing a down-replay
    // could re-`ADD` a column the down is dropping. The same int8 key is used
    // so up and down are mutually exclusive. Transaction-scoped lock held in a
    // dedicated outer tx → auto-released on commit (success path below) or on
    // Drop-rollback (the early-return `?` error paths in this loop), so it can
    // never leak. Down-replay is operator-initiated (`MIGRATIONS_DOWN=1`, exits
    // after) and prod-gated by the confirm env (main.rs), so contention is
    // rare; the lock makes the rare case safe.
    let mut lock_tx = pool.begin().await.map_err(|e| {
        ApiError::Database(format!(
            "Failed to start down-replay advisory-lock tx: {}",
            e
        ))
    })?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(MIGRATION_ADVISORY_LOCK_KEY)
        .execute(&mut *lock_tx)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to acquire down-replay advisory lock: {}",
                e
            ))
        })?;

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
    // Release the down-replay advisory lock (success path). Early-return `?`
    // error paths in the loop drop `lock_tx` → rollback → auto-release.
    let _ = lock_tx.commit().await;
    Ok(replayed)
}
