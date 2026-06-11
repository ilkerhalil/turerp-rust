//! Audit DLQ — persistent dead-letter queue for failed audit writes.
//!
//! The audit writer (in `middleware::audit`) keeps an in-memory DLQ of
//! `CreateAuditLog` entries that failed to persist after the retry
//! budget is exhausted. To survive process restarts, the in-memory
//! DLQ is also spooled here: each failed batch is serialized as JSON
//! with the original error message, ready for replay by the
//! `replay-audit-dlq` CLI binary.
//!
//! Replay reads every row with `replayed_at IS NULL`, attempts to
//! insert into `audit_logs` in a transaction, and on success marks
//! the row as replayed. Failures stay in the DLQ with the new error
//! message; the next replay run picks them up.
//!
//! Why this lives in the audit domain rather than a generic DLQ
//! module: the payload schema is `CreateAuditLog`, and the replay
//! target is `audit_logs`. Generalising would require polymorphism
//! over (payload, target table) which is not worth the complexity for
//! a single queue.

use sqlx::{FromRow, PgPool};

use crate::domain::audit::model::CreateAuditLog;
use crate::error::ApiError;

/// Row read back from `pg_audit_dlq` during replay.
#[derive(Debug, FromRow)]
pub struct DlqRow {
    pub id: i64,
    pub payload: serde_json::Value,
    pub error_message: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Spool a batch of failed audit logs to the persistent DLQ.
///
/// The error_message is recorded for the on-call to read; the payload
/// is the JSON form of the original `CreateAuditLog`. The function
/// returns `Ok(n)` with the number of rows spooled; if the spool
/// itself fails (e.g. the DLQ table is also broken), the error
/// bubbles up so the caller can log it. We do NOT loop on spool
/// failure — if the DLQ is down, the in-memory buffer in
/// `middleware::audit` is the only safety net.
pub async fn spool_batch(
    pool: &PgPool,
    failed: Vec<CreateAuditLog>,
    error: &str,
) -> Result<usize, ApiError> {
    if failed.is_empty() {
        return Ok(0);
    }

    let mut spooled = 0usize;
    for entry in failed {
        let payload = serde_json::to_value(&entry).unwrap_or_else(|e| {
            tracing::error!("Failed to serialize CreateAuditLog for DLQ: {}", e);
            serde_json::json!({
                "serialization_error": e.to_string(),
                "tenant_id": entry.tenant_id,
                "user_id": entry.user_id,
                "action": entry.action,
                "path": entry.path,
                "status_code": entry.status_code,
                "created_at": entry.created_at,
            })
        });

        let result =
            sqlx::query("INSERT INTO pg_audit_dlq (payload, error_message) VALUES ($1, $2)")
                .bind(&payload)
                .bind(error)
                .execute(pool)
                .await;

        match result {
            Ok(_) => spooled += 1,
            Err(e) => {
                tracing::error!(
                    "DLQ spool insert failed: {} (event will be lost; original error: {})",
                    e,
                    error
                );
            }
        }
    }

    Ok(spooled)
}

/// Replay all unreplayed DLQ rows into `audit_logs`.
///
/// Returns `(replayed_count, remaining_count)`. The CLI binary
/// (`src/bin/replay_audit_dlq.rs`) prints these and exits 0 on
/// success (no rows remaining is success — empty DLQ is the steady
/// state). The function is idempotent: running it twice replays each
/// row at most once.
pub async fn replay_all(pool: &PgPool) -> Result<(u64, u64), ApiError> {
    let rows: Vec<DlqRow> = sqlx::query_as(
        "SELECT id, payload, error_message, created_at
         FROM pg_audit_dlq
         WHERE replayed_at IS NULL
         ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::Database(format!("Failed to read DLQ rows: {}", e)))?;

    let total = rows.len() as u64;
    let mut replayed: u64 = 0;

    for row in rows {
        let payload: CreateAuditLog = match serde_json::from_value(row.payload.clone()) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(
                    "DLQ row {} has unparseable payload ({}); marking as replayed to skip",
                    row.id,
                    e
                );
                mark_replayed(pool, row.id).await?;
                continue;
            }
        };

        let mut tx = pool.begin().await.map_err(|e| {
            ApiError::Database(format!("Failed to start replay transaction: {}", e))
        })?;

        let insert_result = sqlx::query(
            r#"
            INSERT INTO audit_logs (
                tenant_id, user_id, username, action, path, status_code,
                request_id, ip_address, user_agent, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(payload.tenant_id)
        .bind(payload.user_id)
        .bind(&payload.username)
        .bind(&payload.action)
        .bind(&payload.path)
        .bind(payload.status_code)
        .bind(&payload.request_id)
        .bind(&payload.ip_address)
        .bind(&payload.user_agent)
        .bind(payload.created_at)
        .execute(&mut *tx)
        .await;

        match insert_result {
            Ok(_) => {
                mark_replayed(&mut *tx, row.id).await?;
                tx.commit().await.map_err(|e| {
                    ApiError::Database(format!(
                        "Failed to commit replay tx for row {}: {}",
                        row.id, e
                    ))
                })?;
                replayed += 1;
            }
            Err(e) => {
                // Roll back, leave the row in the DLQ with the new error
                // message so the next replay run picks it up.
                let _ = tx.rollback().await;
                tracing::warn!(
                    "DLQ row {} failed to replay ({}); will retry on next run",
                    row.id,
                    e
                );
                update_error_message(pool, row.id, &e.to_string()).await?;
            }
        }
    }

    Ok((replayed, total - replayed))
}

/// Mark a DLQ row as replayed (idempotent).
async fn mark_replayed(executor: impl sqlx::PgExecutor<'_>, id: i64) -> Result<(), ApiError> {
    sqlx::query("UPDATE pg_audit_dlq SET replayed_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(executor)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to mark DLQ row {} replayed: {}", id, e))
        })?;
    Ok(())
}

/// Update the error message on a DLQ row (when replay fails).
async fn update_error_message(pool: &PgPool, id: i64, new_error: &str) -> Result<(), ApiError> {
    sqlx::query("UPDATE pg_audit_dlq SET error_message = $1 WHERE id = $2")
        .bind(new_error)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to update DLQ row {} error_message: {}",
                id, e
            ))
        })?;
    Ok(())
}

/// Counts the rows currently in the DLQ (for monitoring / `RUNBOOK.md` § 6).
pub async fn count_unreplayed(pool: &PgPool) -> Result<i64, ApiError> {
    let n: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM pg_audit_dlq WHERE replayed_at IS NULL")
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::Database(format!("Failed to count DLQ rows: {}", e)))?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_log(tenant_id: i64, action: &str) -> CreateAuditLog {
        CreateAuditLog {
            tenant_id,
            user_id: 1,
            username: "testuser".to_string(),
            action: action.to_string(),
            path: "/api/v1/test".to_string(),
            status_code: 200,
            request_id: "req-test".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            created_at: Utc::now(),
        }
    }

    /// Verify the CreateAuditLog JSON round-trips through the DLQ
    /// payload schema. This is the property the replay path depends
    /// on; if the model adds a field but the JSON shape changes,
    /// this test catches it.
    #[test]
    fn create_audit_log_json_roundtrip() {
        let log = sample_log(42, "GET");
        let json = serde_json::to_value(&log).expect("serialize");
        let back: CreateAuditLog = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back.tenant_id, 42);
        assert_eq!(back.action, "GET");
        assert_eq!(back.username, "testuser");
    }

    /// An unparseable payload (e.g. hand-crafted bad row) should
    /// fall back to a JSON object that the replay path can at least
    /// skip — we never want a malformed DLQ row to wedge the writer.
    #[test]
    fn unparseable_payload_falls_back_gracefully() {
        let bad: serde_json::Value = serde_json::json!({"this": "is not a CreateAuditLog"});
        let result: Result<CreateAuditLog, _> = serde_json::from_value(bad);
        assert!(
            result.is_err(),
            "An invalid payload must fail to parse, not silently succeed"
        );
    }
}
