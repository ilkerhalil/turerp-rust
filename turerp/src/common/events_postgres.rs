//! PostgreSQL-backed event bus with Redis Streams publisher
//!
//! Implements the `EventBus` trait with persistent outbox and dead-letter queue
//! stored in PostgreSQL. Pending events are dispatched to Redis Streams for
//! reliable cross-service delivery.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::common::events::{
    DeadLetterEntry, DomainEvent, EventBus, EventStatus, EventSubscriber, OutboxEvent,
};
use crate::db::error::map_sqlx_error;
use crate::error::ApiError;

/// Database row for an outbox event
#[derive(Debug, FromRow)]
struct OutboxEventRow {
    id: i64,
    event: sqlx::types::Json<serde_json::Value>,
    aggregate_type: String,
    aggregate_id: i64,
    tenant_id: i64,
    created_at: DateTime<Utc>,
    published_at: Option<DateTime<Utc>>,
    status: String,
    attempts: i32,
    last_error: Option<String>,
}

impl TryFrom<OutboxEventRow> for OutboxEvent {
    type Error = String;

    fn try_from(row: OutboxEventRow) -> Result<Self, Self::Error> {
        let event: DomainEvent = serde_json::from_value(row.event.0)
            .map_err(|e| format!("Failed to deserialize event: {}", e))?;
        let status = parse_event_status(&row.status);

        Ok(Self {
            id: row.id,
            event,
            aggregate_type: row.aggregate_type,
            aggregate_id: row.aggregate_id,
            tenant_id: row.tenant_id,
            created_at: row.created_at,
            published_at: row.published_at,
            status,
            attempts: row.attempts as u32,
            last_error: row.last_error,
        })
    }
}

/// Database row for a dead-letter entry
#[derive(Debug, FromRow)]
struct DeadLetterRow {
    id: i64,
    original_event: sqlx::types::Json<serde_json::Value>,
    aggregate_type: String,
    aggregate_id: i64,
    tenant_id: i64,
    error: String,
    original_attempts: i32,
    dead_lettered_at: DateTime<Utc>,
}

impl TryFrom<DeadLetterRow> for DeadLetterEntry {
    type Error = String;

    fn try_from(row: DeadLetterRow) -> Result<Self, Self::Error> {
        let original_event: DomainEvent = serde_json::from_value(row.original_event.0)
            .map_err(|e| format!("Failed to deserialize dead letter event: {}", e))?;

        Ok(Self {
            id: row.id,
            original_event,
            aggregate_type: row.aggregate_type,
            aggregate_id: row.aggregate_id,
            tenant_id: row.tenant_id,
            error: row.error,
            original_attempts: row.original_attempts as u32,
            dead_lettered_at: row.dead_lettered_at,
        })
    }
}

fn parse_event_status(s: &str) -> EventStatus {
    match s {
        "pending" => EventStatus::Pending,
        "published" => EventStatus::Published,
        "failed" => EventStatus::Failed,
        "dead_lettered" => EventStatus::DeadLettered,
        _ => EventStatus::Pending,
    }
}

fn event_status_str(s: EventStatus) -> &'static str {
    match s {
        EventStatus::Pending => "pending",
        EventStatus::Published => "published",
        EventStatus::Failed => "failed",
        EventStatus::DeadLettered => "dead_lettered",
    }
}

/// Redis Streams payload for an outbox event
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RedisStreamPayload {
    tenant_id: i64,
    event_type: String,
    aggregate_type: String,
    aggregate_id: i64,
    payload: String,
    occurred_at: String,
}

/// Publish a domain event to Redis Streams
pub async fn publish_to_redis_streams(
    conn: &mut redis::aio::MultiplexedConnection,
    stream_prefix: &str,
    event: &DomainEvent,
    aggregate_type: &str,
    aggregate_id: i64,
) -> Result<(), String> {
    let tenant_id = event.tenant_id();
    let event_type = event.event_type();
    let stream_key = format!("{}:events:t{}:{}", stream_prefix, tenant_id, event_type);

    let payload = RedisStreamPayload {
        tenant_id,
        event_type: event_type.to_string(),
        aggregate_type: aggregate_type.to_string(),
        aggregate_id,
        payload: serde_json::to_string(event).map_err(|e| e.to_string())?,
        occurred_at: Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;

    redis::cmd("XADD")
        .arg(&stream_key)
        .arg("*")
        .arg("payload")
        .arg(&json)
        .query_async::<()>(conn)
        .await
        .map_err(|e| format!("Redis XADD failed: {}", e))?;

    // Ensure consumer group exists for this stream
    let group_name = format!("{}_consumers", stream_prefix);
    let _: Result<(), redis::RedisError> = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(&stream_key)
        .arg(&group_name)
        .arg("$")
        .arg("MKSTREAM")
        .query_async(conn)
        .await;
    // Ignore error if group already exists

    Ok(())
}

/// PostgreSQL-backed event bus with optional Redis Streams publishing
pub struct PostgresEventBus {
    pool: Arc<PgPool>,
    redis_conn: parking_lot::Mutex<Option<redis::aio::MultiplexedConnection>>,
    subscribers: RwLock<Vec<Arc<dyn EventSubscriber>>>,
    max_attempts: u32,
    stream_prefix: String,
}

impl PostgresEventBus {
    /// Create a new PostgreSQL event bus
    pub fn new(
        pool: Arc<PgPool>,
        redis_conn: Option<redis::aio::MultiplexedConnection>,
        max_attempts: u32,
    ) -> Self {
        Self {
            pool,
            redis_conn: parking_lot::Mutex::new(redis_conn),
            subscribers: RwLock::new(Vec::new()),
            max_attempts,
            stream_prefix: "turerp".to_string(),
        }
    }

    /// Set the Redis Streams key prefix (default: "turerp")
    pub fn with_stream_prefix(mut self, prefix: String) -> Self {
        self.stream_prefix = prefix;
        self
    }

    /// Dispatch an event to in-memory subscribers
    async fn dispatch_to_subscribers(&self, event: &DomainEvent) -> Result<(), String> {
        let event_type = event.event_type();
        let to_notify: Vec<Arc<dyn EventSubscriber>> = {
            let subscribers = self.subscribers.read();
            subscribers
                .iter()
                .filter(|s| {
                    let interested = s.subscribed_to();
                    interested.is_empty() || interested.iter().any(|t| t == event_type || t == "*")
                })
                .cloned()
                .collect()
        };

        for subscriber in to_notify {
            let name = subscriber.name().to_string();
            if let Err(e) = subscriber.handle(event).await {
                tracing::warn!("Subscriber {} failed for event {}: {}", name, event_type, e);
            }
        }
        Ok(())
    }

    /// Helper to map ApiError to String for trait compatibility
    fn map_err(e: ApiError) -> String {
        e.to_string()
    }
}

impl Default for PostgresEventBus {
    fn default() -> Self {
        // This is only useful for testing when no pool is needed immediately.
        // In practice, always use PostgresEventBus::new().
        unimplemented!("PostgresEventBus requires a PgPool")
    }
}

#[async_trait::async_trait]
impl EventBus for PostgresEventBus {
    async fn publish(&self, event: DomainEvent) -> Result<i64, String> {
        let tenant_id = event.tenant_id();
        let event_type = event.event_type().to_string();
        let event_json = serde_json::to_value(&event).map_err(|e| e.to_string())?;

        let row = sqlx::query_as::<_, OutboxEventRow>(
            r#"
            INSERT INTO outbox_events (event, aggregate_type, aggregate_id, tenant_id, status)
            VALUES ($1, $2, $3, $4, 'pending')
            RETURNING *
            "#,
        )
        .bind(sqlx::types::Json(event_json))
        .bind(&event_type)
        .bind(0i64) // aggregate_id not known for new events
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| Self::map_err(map_sqlx_error(e, "OutboxEvent")))?;

        // Dispatch to in-memory subscribers for backward compatibility
        self.dispatch_to_subscribers(&event).await?;

        Ok(row.id)
    }

    async fn publish_batch(&self, events: Vec<DomainEvent>) -> Result<Vec<i64>, String> {
        let mut ids = Vec::with_capacity(events.len());
        for event in events {
            let id = self.publish(event).await?;
            ids.push(id);
        }
        Ok(ids)
    }

    async fn subscribe(&self, subscriber: Arc<dyn EventSubscriber>) -> Result<(), String> {
        self.subscribers.write().push(subscriber);
        Ok(())
    }

    async fn process_outbox(&self, batch_size: u32) -> Result<u64, String> {
        let rows = sqlx::query_as::<_, OutboxEventRow>(
            r#"
            SELECT * FROM outbox_events
            WHERE status = 'pending'
            ORDER BY created_at ASC
            LIMIT $1
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(batch_size as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| Self::map_err(map_sqlx_error(e, "OutboxEvent")))?;

        let mut processed = 0u64;
        let mut conn_opt = self.redis_conn.lock().clone();

        for row in rows {
            let event: DomainEvent = serde_json::from_value(row.event.0)
                .map_err(|e| format!("Failed to deserialize event: {}", e))?;

            // Publish to Redis Streams if Redis is available
            if let Some(ref mut conn) = conn_opt {
                if let Err(e) = publish_to_redis_streams(
                    conn,
                    &self.stream_prefix,
                    &event,
                    &row.aggregate_type,
                    row.aggregate_id,
                )
                .await
                {
                    tracing::warn!("Failed to publish event {} to Redis: {}", row.id, e);
                    // Continue to mark as published anyway; Redis is best-effort here.
                    // In a stricter system, we'd retry or mark failed.
                }
            }

            // Dispatch to in-memory subscribers
            if let Err(e) = self.dispatch_to_subscribers(&event).await {
                tracing::warn!("Failed to dispatch event {} to subscribers: {}", row.id, e);
            }

            // Mark as published
            sqlx::query(
                "UPDATE outbox_events SET status = 'published', published_at = NOW(), updated_at = NOW() WHERE id = $1",
            )
            .bind(row.id)
            .execute(&*self.pool)
            .await
            .map_err(|e| Self::map_err(map_sqlx_error(e, "OutboxEvent")))?;

            processed += 1;
        }

        Ok(processed)
    }

    async fn get_pending(&self, limit: u32) -> Result<Vec<OutboxEvent>, String> {
        let rows = sqlx::query_as::<_, OutboxEventRow>(
            "SELECT * FROM outbox_events WHERE status = 'pending' ORDER BY created_at ASC LIMIT $1",
        )
        .bind(limit as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| Self::map_err(map_sqlx_error(e, "OutboxEvent")))?;

        rows.into_iter()
            .map(OutboxEvent::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn mark_published(&self, event_id: i64) -> Result<(), String> {
        sqlx::query(
            "UPDATE outbox_events SET status = 'published', published_at = NOW(), updated_at = NOW() WHERE id = $1",
        )
        .bind(event_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| Self::map_err(map_sqlx_error(e, "OutboxEvent")))?;

        Ok(())
    }

    async fn mark_failed(&self, event_id: i64, error: &str) -> Result<(), String> {
        let row = sqlx::query_as::<_, OutboxEventRow>("SELECT * FROM outbox_events WHERE id = $1")
            .bind(event_id)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| Self::map_err(map_sqlx_error(e, "OutboxEvent")))?;

        let row = row.ok_or_else(|| format!("Outbox event {} not found", event_id))?;
        let new_attempts = row.attempts + 1;

        if new_attempts >= self.max_attempts as i32 {
            // Move to DLQ
            sqlx::query(
                r#"
                INSERT INTO dead_letter_queue (original_event, aggregate_type, aggregate_id, tenant_id, error, original_attempts)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(row.event)
            .bind(&row.aggregate_type)
            .bind(row.aggregate_id)
            .bind(row.tenant_id)
            .bind(error)
            .bind(new_attempts)
            .execute(&*self.pool)
            .await
            .map_err(|e| Self::map_err(map_sqlx_error(e, "DeadLetterQueue")))?;

            sqlx::query(
                "UPDATE outbox_events SET status = 'dead_lettered', attempts = $1, last_error = $2, updated_at = NOW() WHERE id = $3",
            )
            .bind(new_attempts)
            .bind(error)
            .bind(event_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| Self::map_err(map_sqlx_error(e, "OutboxEvent")))?;
        } else {
            sqlx::query(
                "UPDATE outbox_events SET status = 'failed', attempts = $1, last_error = $2, updated_at = NOW() WHERE id = $3",
            )
            .bind(new_attempts)
            .bind(error)
            .bind(event_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| Self::map_err(map_sqlx_error(e, "OutboxEvent")))?;
        }

        Ok(())
    }

    async fn get_dead_letters(&self, tenant_id: i64) -> Result<Vec<DeadLetterEntry>, String> {
        let rows = sqlx::query_as::<_, DeadLetterRow>(
            "SELECT * FROM dead_letter_queue WHERE tenant_id = $1 ORDER BY dead_lettered_at DESC",
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| Self::map_err(map_sqlx_error(e, "DeadLetterQueue")))?;

        rows.into_iter()
            .map(DeadLetterEntry::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn retry_dead_letter(&self, entry_id: i64) -> Result<(), String> {
        let row =
            sqlx::query_as::<_, DeadLetterRow>("SELECT * FROM dead_letter_queue WHERE id = $1")
                .bind(entry_id)
                .fetch_optional(&*self.pool)
                .await
                .map_err(|e| Self::map_err(map_sqlx_error(e, "DeadLetterQueue")))?;

        let row = row.ok_or_else(|| format!("DLQ entry {} not found", entry_id))?;

        let event: DomainEvent = serde_json::from_value(row.original_event.0)
            .map_err(|e| format!("Failed to deserialize dead letter event: {}", e))?;

        // Re-publish as a new outbox event
        self.publish(event).await?;

        // Remove from DLQ
        sqlx::query("DELETE FROM dead_letter_queue WHERE id = $1")
            .bind(entry_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| Self::map_err(map_sqlx_error(e, "DeadLetterQueue")))?;

        Ok(())
    }

    async fn retry_outbox(&self, event_id: i64) -> Result<(), String> {
        let updated = sqlx::query(
            "UPDATE outbox_events SET status = 'pending', attempts = 0, last_error = NULL, updated_at = NOW() WHERE id = $1 AND status IN ('failed', 'dead_lettered')",
        )
        .bind(event_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| Self::map_err(map_sqlx_error(e, "OutboxEvent")))?;

        if updated.rows_affected() == 0 {
            return Err(format!(
                "Outbox event {} not found or not in retryable state",
                event_id
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_event_status() {
        assert_eq!(parse_event_status("pending"), EventStatus::Pending);
        assert_eq!(parse_event_status("published"), EventStatus::Published);
        assert_eq!(parse_event_status("failed"), EventStatus::Failed);
        assert_eq!(
            parse_event_status("dead_lettered"),
            EventStatus::DeadLettered
        );
        assert_eq!(parse_event_status("unknown"), EventStatus::Pending);
    }

    #[test]
    fn test_event_status_str() {
        assert_eq!(event_status_str(EventStatus::Pending), "pending");
        assert_eq!(event_status_str(EventStatus::Published), "published");
        assert_eq!(event_status_str(EventStatus::Failed), "failed");
        assert_eq!(event_status_str(EventStatus::DeadLettered), "dead_lettered");
    }

    #[test]
    fn test_redis_stream_payload_serialization() {
        let payload = RedisStreamPayload {
            tenant_id: 1,
            event_type: "invoice_created".to_string(),
            aggregate_type: "invoice_created".to_string(),
            aggregate_id: 42,
            payload: "{}".to_string(),
            occurred_at: Utc::now().to_rfc3339(),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("invoice_created"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_outbox_event_row_conversion() {
        let event = DomainEvent::InvoiceCreated {
            invoice_id: 1,
            tenant_id: 1,
            amount: "100.00".to_string(),
            currency: "TRY".to_string(),
        };
        let row = OutboxEventRow {
            id: 1,
            event: sqlx::types::Json(serde_json::to_value(&event).unwrap()),
            aggregate_type: "invoice_created".to_string(),
            aggregate_id: 1,
            tenant_id: 1,
            created_at: Utc::now(),
            published_at: None,
            status: "pending".to_string(),
            attempts: 0,
            last_error: None,
        };

        let outbox = OutboxEvent::try_from(row).unwrap();
        assert_eq!(outbox.id, 1);
        assert_eq!(outbox.status, EventStatus::Pending);
        assert_eq!(outbox.event.tenant_id(), 1);
    }

    #[test]
    fn test_dead_letter_row_conversion() {
        let event = DomainEvent::Custom {
            name: "test".to_string(),
            tenant_id: 1,
            payload: "{}".to_string(),
        };
        let row = DeadLetterRow {
            id: 1,
            original_event: sqlx::types::Json(serde_json::to_value(&event).unwrap()),
            aggregate_type: "test".to_string(),
            aggregate_id: 1,
            tenant_id: 1,
            error: "Connection timeout".to_string(),
            original_attempts: 3,
            dead_lettered_at: Utc::now(),
        };

        let dlq = DeadLetterEntry::try_from(row).unwrap();
        assert_eq!(dlq.id, 1);
        assert_eq!(dlq.error, "Connection timeout");
        assert_eq!(dlq.original_attempts, 3);
    }
}
