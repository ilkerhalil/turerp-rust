//! Replay the persistent audit DLQ.
//!
//! Drains `pg_audit_dlq` rows with `replayed_at IS NULL` back into
//! `audit_logs`. Idempotent: running it twice replays each row at
//! most once. Exits 0 on success (including the "DLQ is empty"
//! steady state), 1 on any database error.
//!
//! ## Usage
//!
//! ```bash
//! # Inside the running container (RUNBOOK.md § 6):
//! docker compose exec -T turerp /app/turerp replay-audit-dlq
//! ```
//!
//! The binary reads the same `DATABASE_URL` env var as the main
//! app (`turerp::config::DatabaseConfig::from_env`). It is a
//! separate binary so the running app can keep serving traffic
//! while the replay runs against the same DB.
//!
//! ## Why a separate binary
//!
//! The replay path is a one-shot operation. Putting it in the
//! server's HTTP API would require a new admin route and an
//! audit log entry for the replay itself; the operator runs it
//! rarely enough that a CLI is the right shape. The cost is one
//! extra binary in the docker image.

use std::env;
use std::process::ExitCode;

use sqlx::postgres::PgPoolOptions;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,turerp=info")),
        )
        .init();

    let database_url = match env::var("DATABASE_URL") {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("DATABASE_URL env var is not set: {}", e);
            return ExitCode::from(1);
        }
    };

    let pool = match PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to connect to database: {}", e);
            return ExitCode::from(1);
        }
    };

    info!("Starting audit DLQ replay");
    match turerp::domain::audit::dlq::replay_all(&pool).await {
        Ok((replayed, remaining)) => {
            info!(
                "Replay complete: {} replayed, {} remaining",
                replayed, remaining
            );
            if remaining > 0 {
                tracing::warn!(
                    "{} DLQ rows still unreplayed; re-run the binary to retry",
                    remaining
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            tracing::error!("Replay failed: {}", e);
            ExitCode::from(1)
        }
    }
}
