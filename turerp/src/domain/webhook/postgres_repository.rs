//! PostgreSQL webhook repository implementations

#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use chrono::{DateTime, Utc};
#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};
#[cfg(feature = "postgres")]
use std::sync::Arc;

#[cfg(feature = "postgres")]
use crate::common::pagination::PaginatedResult;
#[cfg(feature = "postgres")]
use crate::db::error::map_sqlx_error;
#[cfg(feature = "postgres")]
use crate::domain::webhook::model::{
    CreateWebhook, DeliveryStatus, UpdateWebhook, Webhook, WebhookDelivery, WebhookStatus,
};
#[cfg(feature = "postgres")]
use crate::domain::webhook::repository::{WebhookDeliveryRepository, WebhookRepository};
#[cfg(feature = "postgres")]
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// WebhookRow / Webhook conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
#[derive(Debug, FromRow)]
struct WebhookRow {
    id: i64,
    tenant_id: i64,
    url: String,
    description: Option<String>,
    event_types: Vec<String>,
    secret: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
}

#[cfg(feature = "postgres")]
impl From<WebhookRow> for Webhook {
    fn from(row: WebhookRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid WebhookStatus '{}': {}, defaulting to Active",
                row.status,
                e
            );
            WebhookStatus::Active
        });
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            url: row.url,
            description: row.description,
            event_types: row.event_types,
            secret: row.secret,
            status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

// ---------------------------------------------------------------------------
// WebhookDeliveryRow / WebhookDelivery conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
#[derive(Debug, FromRow)]
struct WebhookDeliveryRow {
    id: i64,
    webhook_id: i64,
    tenant_id: i64,
    event_type: String,
    payload: String,
    status: String,
    http_status: Option<i32>,
    response_body: Option<String>,
    error_message: Option<String>,
    attempt_count: i32,
    scheduled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    delivered_at: Option<DateTime<Utc>>,
}

#[cfg(feature = "postgres")]
impl From<WebhookDeliveryRow> for WebhookDelivery {
    fn from(row: WebhookDeliveryRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid DeliveryStatus '{}': {}, defaulting to Pending",
                row.status,
                e
            );
            DeliveryStatus::Pending
        });
        Self {
            id: row.id,
            webhook_id: row.webhook_id,
            tenant_id: row.tenant_id,
            event_type: row.event_type,
            payload: row.payload,
            status,
            http_status: row.http_status,
            response_body: row.response_body,
            error_message: row.error_message,
            attempt_count: row.attempt_count,
            scheduled_at: row.scheduled_at,
            created_at: row.created_at,
            delivered_at: row.delivered_at,
        }
    }
}

#[cfg(feature = "postgres")]
#[derive(Debug, FromRow)]
struct WebhookDeliveryRowWithTotal {
    id: i64,
    webhook_id: i64,
    tenant_id: i64,
    event_type: String,
    payload: String,
    status: String,
    http_status: Option<i32>,
    response_body: Option<String>,
    error_message: Option<String>,
    attempt_count: i32,
    scheduled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    delivered_at: Option<DateTime<Utc>>,
    total_count: i64,
}

// ---------------------------------------------------------------------------
// PostgresWebhookRepository
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
pub struct PostgresWebhookRepository {
    pool: Arc<PgPool>,
}

#[cfg(feature = "postgres")]
impl PostgresWebhookRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> Arc<dyn WebhookRepository> {
        Arc::new(self)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl WebhookRepository for PostgresWebhookRepository {
    async fn create(&self, tenant_id: i64, webhook: CreateWebhook) -> Result<Webhook, ApiError> {
        let secret = webhook.secret.unwrap_or_else(generate_secret);
        let row: WebhookRow = sqlx::query_as(
            r#"
            INSERT INTO webhooks (tenant_id, url, description, event_types, secret, status)
            VALUES ($1, $2, $3, $4, $5, 'active')
            RETURNING id, tenant_id, url, description, event_types, secret, status,
                created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(tenant_id)
        .bind(&webhook.url)
        .bind(&webhook.description)
        .bind(&webhook.event_types)
        .bind(&secret)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Webhook"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Webhook>, ApiError> {
        let row = sqlx::query_as::<_, WebhookRow>(
            r#"
            SELECT id, tenant_id, url, description, event_types, secret, status,
                created_at, updated_at, deleted_at, deleted_by
            FROM webhooks
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Webhook"))?;

        Ok(row.map(Into::into))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Webhook>, ApiError> {
        let rows = sqlx::query_as::<_, WebhookRow>(
            r#"
            SELECT id, tenant_id, url, description, event_types, secret, status,
                created_at, updated_at, deleted_at, deleted_by
            FROM webhooks
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Webhook"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_active_by_event(
        &self,
        tenant_id: i64,
        event_type: &str,
    ) -> Result<Vec<Webhook>, ApiError> {
        let rows = sqlx::query_as::<_, WebhookRow>(
            r#"
            SELECT id, tenant_id, url, description, event_types, secret, status,
                created_at, updated_at, deleted_at, deleted_by
            FROM webhooks
            WHERE tenant_id = $1 AND deleted_at IS NULL AND status = 'active'
                AND (cardinality(event_types) = 0 OR $2 = ANY(event_types) OR '*' = ANY(event_types))
            "#,
        )
        .bind(tenant_id)
        .bind(event_type)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Webhook"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        dto: UpdateWebhook,
    ) -> Result<Webhook, ApiError> {
        let row: WebhookRow = sqlx::query_as(
            r#"
            UPDATE webhooks
            SET url = COALESCE($3, url),
                description = COALESCE($4, description),
                event_types = COALESCE($5, event_types),
                status = COALESCE($6, status),
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            RETURNING id, tenant_id, url, description, event_types, secret, status,
                created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(&dto.url)
        .bind(&dto.description)
        .bind(&dto.event_types)
        .bind(dto.status.map(|s| s.to_string()))
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Webhook"))?
        .ok_or_else(|| ApiError::NotFound(format!("Webhook {} not found", id)))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query("DELETE FROM webhooks WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Webhook"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Webhook {} not found", id)));
        }
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            "UPDATE webhooks SET deleted_at = NOW(), deleted_by = $3 WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL",
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Webhook"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Webhook {} not found", id)));
        }
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            "UPDATE webhooks SET deleted_at = NULL, deleted_by = NULL WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL",
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Webhook"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted webhook {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Webhook>, ApiError> {
        let rows = sqlx::query_as::<_, WebhookRow>(
            r#"
            SELECT id, tenant_id, url, description, event_types, secret, status,
                created_at, updated_at, deleted_at, deleted_by
            FROM webhooks
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Webhook"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            "DELETE FROM webhooks WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL",
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Webhook"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted webhook {} not found",
                id
            )));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PostgresWebhookDeliveryRepository
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
pub struct PostgresWebhookDeliveryRepository {
    pool: Arc<PgPool>,
}

#[cfg(feature = "postgres")]
impl PostgresWebhookDeliveryRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> Arc<dyn WebhookDeliveryRepository> {
        Arc::new(self)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl WebhookDeliveryRepository for PostgresWebhookDeliveryRepository {
    async fn create(&self, delivery: WebhookDelivery) -> Result<WebhookDelivery, ApiError> {
        let row: WebhookDeliveryRow = sqlx::query_as(
            r#"
            INSERT INTO webhook_deliveries
                (webhook_id, tenant_id, event_type, payload, status, attempt_count, scheduled_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, webhook_id, tenant_id, event_type, payload, status, http_status,
                response_body, error_message, attempt_count, scheduled_at, created_at, delivered_at
            "#,
        )
        .bind(delivery.webhook_id)
        .bind(delivery.tenant_id)
        .bind(&delivery.event_type)
        .bind(&delivery.payload)
        .bind(delivery.status.to_string())
        .bind(delivery.attempt_count)
        .bind(delivery.scheduled_at)
        .bind(delivery.created_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WebhookDelivery"))?;

        Ok(row.into())
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<WebhookDelivery>, ApiError> {
        let row = sqlx::query_as::<_, WebhookDeliveryRow>(
            r#"
            SELECT id, webhook_id, tenant_id, event_type, payload, status, http_status,
                response_body, error_message, attempt_count, scheduled_at, created_at, delivered_at
            FROM webhook_deliveries
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WebhookDelivery"))?;

        Ok(row.map(Into::into))
    }

    async fn find_by_webhook(
        &self,
        webhook_id: i64,
        tenant_id: i64,
        page: i64,
        per_page: i64,
    ) -> Result<PaginatedResult<WebhookDelivery>, ApiError> {
        let offset = (page - 1) * per_page;

        let rows = sqlx::query_as::<_, WebhookDeliveryRowWithTotal>(
            r#"
            SELECT id, webhook_id, tenant_id, event_type, payload, status, http_status,
                response_body, error_message, attempt_count, scheduled_at, created_at, delivered_at,
                COUNT(*) OVER() as total_count
            FROM webhook_deliveries
            WHERE webhook_id = $1 AND tenant_id = $2
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(webhook_id)
        .bind(tenant_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WebhookDelivery"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items = rows
            .into_iter()
            .map(|r| WebhookDelivery {
                id: r.id,
                webhook_id: r.webhook_id,
                tenant_id: r.tenant_id,
                event_type: r.event_type,
                payload: r.payload,
                status: r.status.parse().unwrap_or(DeliveryStatus::Pending),
                http_status: r.http_status,
                response_body: r.response_body,
                error_message: r.error_message,
                attempt_count: r.attempt_count,
                scheduled_at: r.scheduled_at,
                created_at: r.created_at,
                delivered_at: r.delivered_at,
            })
            .collect();

        Ok(PaginatedResult::new(
            items,
            page as u32,
            per_page as u32,
            total,
        ))
    }

    async fn find_pending_retries(
        &self,
        before: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<WebhookDelivery>, ApiError> {
        let rows = sqlx::query_as::<_, WebhookDeliveryRow>(
            r#"
            SELECT id, webhook_id, tenant_id, event_type, payload, status, http_status,
                response_body, error_message, attempt_count, scheduled_at, created_at, delivered_at
            FROM webhook_deliveries
            WHERE status IN ('pending', 'retrying')
                AND (scheduled_at IS NULL OR scheduled_at <= $1)
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(before)
        .bind(limit)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WebhookDelivery"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: DeliveryStatus,
        http_status: Option<i32>,
        response_body: Option<String>,
        error: Option<String>,
    ) -> Result<(), ApiError> {
        let delivered_at = if status == DeliveryStatus::Delivered {
            Some(Utc::now())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET status = $3, http_status = $4, response_body = $5,
                error_message = $6, delivered_at = COALESCE($7, delivered_at)
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(status.to_string())
        .bind(http_status)
        .bind(response_body)
        .bind(error)
        .bind(delivered_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WebhookDelivery"))?;

        Ok(())
    }

    async fn increment_attempt(
        &self,
        id: i64,
        tenant_id: i64,
        next_retry: Option<DateTime<Utc>>,
    ) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            UPDATE webhook_deliveries
            SET attempt_count = attempt_count + 1, scheduled_at = $3, status = 'retrying'
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(next_retry)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WebhookDelivery"))?;

        Ok(())
    }
}

#[cfg(feature = "postgres")]
fn generate_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect()
}
