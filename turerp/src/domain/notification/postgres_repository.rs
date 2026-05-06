//! PostgreSQL notification repository implementations

#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use chrono::{DateTime, Utc};
#[cfg(feature = "postgres")]
use sqlx::{FromRow, PgPool};
#[cfg(feature = "postgres")]
use std::sync::Arc;

#[cfg(feature = "postgres")]
use crate::common::PaginatedResult;
#[cfg(feature = "postgres")]
use crate::db::error::map_sqlx_error;
#[cfg(feature = "postgres")]
use crate::domain::notification::model::{
    InAppNotification, Notification, NotificationChannel, NotificationPreference,
    NotificationStatus, UpdatePreference,
};
#[cfg(feature = "postgres")]
use crate::domain::notification::repository::{
    InAppNotificationRepository, NotificationPreferenceRepository, NotificationRepository,
};
#[cfg(feature = "postgres")]
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// NotificationRow / Notification conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
#[derive(Debug, FromRow)]
struct NotificationRow {
    id: i64,
    tenant_id: i64,
    user_id: Option<i64>,
    channel: String,
    priority: String,
    status: String,
    notification_type: String,
    subject: Option<String>,
    body: Option<String>,
    recipient: String,
    template_key: Option<String>,
    template_vars: Option<serde_json::Value>,
    provider_message_id: Option<String>,
    created_at: DateTime<Utc>,
    sent_at: Option<DateTime<Utc>>,
    read_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    attempts: i32,
    job_id: Option<i64>,
}

#[cfg(feature = "postgres")]
impl From<NotificationRow> for Notification {
    fn from(row: NotificationRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            user_id: row.user_id,
            channel: row.channel.parse().unwrap_or(NotificationChannel::Email),
            priority: row.priority.parse().unwrap_or_default(),
            status: row.status.parse().unwrap_or(NotificationStatus::Queued),
            notification_type: row.notification_type,
            subject: row.subject.unwrap_or_default(),
            body: row.body.unwrap_or_default(),
            recipient: row.recipient,
            template_key: row.template_key,
            template_vars: row.template_vars,
            provider_message_id: row.provider_message_id,
            created_at: row.created_at,
            sent_at: row.sent_at,
            read_at: row.read_at,
            last_error: row.last_error,
            attempts: row.attempts as u32,
            job_id: row.job_id,
        }
    }
}

#[cfg(feature = "postgres")]
#[derive(Debug, FromRow)]
struct NotificationRowWithTotal {
    id: i64,
    tenant_id: i64,
    user_id: Option<i64>,
    channel: String,
    priority: String,
    status: String,
    notification_type: String,
    subject: Option<String>,
    body: Option<String>,
    recipient: String,
    template_key: Option<String>,
    template_vars: Option<serde_json::Value>,
    provider_message_id: Option<String>,
    created_at: DateTime<Utc>,
    sent_at: Option<DateTime<Utc>>,
    read_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    attempts: i32,
    job_id: Option<i64>,
    total_count: i64,
}

// ---------------------------------------------------------------------------
// PostgresNotificationRepository
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
pub struct PostgresNotificationRepository {
    pool: Arc<PgPool>,
}

#[cfg(feature = "postgres")]
impl PostgresNotificationRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> Arc<dyn NotificationRepository> {
        Arc::new(self)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl NotificationRepository for PostgresNotificationRepository {
    async fn create(&self, notification: Notification) -> Result<Notification, ApiError> {
        let row: NotificationRow = sqlx::query_as(
            r#"
            INSERT INTO notifications
                (tenant_id, user_id, channel, priority, status, notification_type,
                 subject, body, recipient, template_key, template_vars, attempts, job_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING id, tenant_id, user_id, channel, priority, status, notification_type,
                subject, body, recipient, template_key, template_vars, provider_message_id,
                created_at, sent_at, read_at, last_error, attempts, job_id
            "#,
        )
        .bind(notification.tenant_id)
        .bind(notification.user_id)
        .bind(notification.channel.to_string())
        .bind(notification.priority.to_string())
        .bind(notification.status.to_string())
        .bind(&notification.notification_type)
        .bind(&notification.subject)
        .bind(&notification.body)
        .bind(&notification.recipient)
        .bind(&notification.template_key)
        .bind(&notification.template_vars)
        .bind(notification.attempts as i32)
        .bind(notification.job_id)
        .bind(notification.created_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Notification"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Notification>, ApiError> {
        let row = sqlx::query_as::<_, NotificationRow>(
            r#"
            SELECT id, tenant_id, user_id, channel, priority, status, notification_type,
                subject, body, recipient, template_key, template_vars, provider_message_id,
                created_at, sent_at, read_at, last_error, attempts, job_id
            FROM notifications
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Notification"))?;

        Ok(row.map(Into::into))
    }

    async fn find_by_user(
        &self,
        tenant_id: i64,
        user_id: Option<i64>,
        channel: Option<NotificationChannel>,
        limit: i64,
        offset: i64,
    ) -> Result<PaginatedResult<Notification>, ApiError> {
        let rows = sqlx::query_as::<_, NotificationRowWithTotal>(
            r#"
            SELECT id, tenant_id, user_id, channel, priority, status, notification_type,
                subject, body, recipient, template_key, template_vars, provider_message_id,
                created_at, sent_at, read_at, last_error, attempts, job_id,
                COUNT(*) OVER() as total_count
            FROM notifications
            WHERE tenant_id = $1
                AND ($2::BIGINT IS NULL OR user_id = $2)
                AND ($3::VARCHAR IS NULL OR channel = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(channel.map(|c| c.to_string()))
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Notification"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items = rows.into_iter().map(Into::into).collect();

        Ok(PaginatedResult::new(
            items,
            (offset / limit + 1) as u32,
            limit as u32,
            total,
        ))
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: NotificationStatus,
        provider_message_id: Option<String>,
        last_error: Option<String>,
    ) -> Result<(), ApiError> {
        let sent_at =
            if status == NotificationStatus::Sent || status == NotificationStatus::Delivered {
                Some(Utc::now())
            } else {
                None
            };

        sqlx::query(
            r#"
            UPDATE notifications
            SET status = $3,
                provider_message_id = COALESCE($4, provider_message_id),
                last_error = COALESCE($5, last_error),
                sent_at = COALESCE($6, sent_at)
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(status.to_string())
        .bind(provider_message_id)
        .bind(last_error)
        .bind(sent_at)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Notification"))?;

        Ok(())
    }

    async fn increment_attempt(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            "UPDATE notifications SET attempts = attempts + 1 WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Notification"))?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// InAppNotificationRow / InAppNotification conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
#[derive(Debug, FromRow)]
struct InAppNotificationRow {
    id: i64,
    tenant_id: i64,
    user_id: i64,
    title: String,
    message: String,
    notification_type: String,
    read: bool,
    link: Option<String>,
    related_notification_id: Option<i64>,
    created_at: DateTime<Utc>,
    read_at: Option<DateTime<Utc>>,
}

#[cfg(feature = "postgres")]
impl From<InAppNotificationRow> for InAppNotification {
    fn from(row: InAppNotificationRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            user_id: row.user_id,
            title: row.title,
            message: row.message,
            notification_type: row.notification_type,
            read: row.read,
            created_at: row.created_at,
            read_at: row.read_at,
            link: row.link,
            related_notification_id: row.related_notification_id,
        }
    }
}

// ---------------------------------------------------------------------------
// PostgresInAppNotificationRepository
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
pub struct PostgresInAppNotificationRepository {
    pool: Arc<PgPool>,
}

#[cfg(feature = "postgres")]
impl PostgresInAppNotificationRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> Arc<dyn InAppNotificationRepository> {
        Arc::new(self)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl InAppNotificationRepository for PostgresInAppNotificationRepository {
    async fn create(&self, notification: InAppNotification) -> Result<InAppNotification, ApiError> {
        let row: InAppNotificationRow = sqlx::query_as(
            r#"
            INSERT INTO in_app_notifications
                (tenant_id, user_id, title, message, notification_type, read, link,
                 related_notification_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, tenant_id, user_id, title, message, notification_type, read,
                link, related_notification_id, created_at, read_at
            "#,
        )
        .bind(notification.tenant_id)
        .bind(notification.user_id)
        .bind(&notification.title)
        .bind(&notification.message)
        .bind(&notification.notification_type)
        .bind(notification.read)
        .bind(&notification.link)
        .bind(notification.related_notification_id)
        .bind(notification.created_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InAppNotification"))?;

        Ok(row.into())
    }

    async fn find_by_user(
        &self,
        tenant_id: i64,
        user_id: i64,
        unread_only: bool,
    ) -> Result<Vec<InAppNotification>, ApiError> {
        let rows = sqlx::query_as::<_, InAppNotificationRow>(
            r#"
            SELECT id, tenant_id, user_id, title, message, notification_type, read,
                link, related_notification_id, created_at, read_at
            FROM in_app_notifications
            WHERE tenant_id = $1 AND user_id = $2
                AND ($3::BOOLEAN = false OR read = false)
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(unread_only)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InAppNotification"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn mark_as_read(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            "UPDATE in_app_notifications SET read = true, read_at = NOW() WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InAppNotification"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "In-app notification {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn mark_all_as_read(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError> {
        let result = sqlx::query(
            "UPDATE in_app_notifications SET read = true, read_at = NOW() WHERE tenant_id = $1 AND user_id = $2 AND read = false",
        )
        .bind(tenant_id)
        .bind(user_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InAppNotification"))?;

        Ok(result.rows_affected())
    }

    async fn unread_count(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM in_app_notifications WHERE tenant_id = $1 AND user_id = $2 AND read = false",
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InAppNotification"))?;

        Ok(count as u64)
    }
}

// ---------------------------------------------------------------------------
// NotificationPreferenceRow / NotificationPreference conversion
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
#[derive(Debug, FromRow)]
struct NotificationPreferenceRow {
    id: i64,
    tenant_id: i64,
    user_id: i64,
    channel: String,
    notification_type: String,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[cfg(feature = "postgres")]
impl From<NotificationPreferenceRow> for NotificationPreference {
    fn from(row: NotificationPreferenceRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            user_id: row.user_id,
            channel: row.channel.parse().unwrap_or(NotificationChannel::Email),
            notification_type: row.notification_type,
            enabled: row.enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// PostgresNotificationPreferenceRepository
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
pub struct PostgresNotificationPreferenceRepository {
    pool: Arc<PgPool>,
}

#[cfg(feature = "postgres")]
impl PostgresNotificationPreferenceRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> Arc<dyn NotificationPreferenceRepository> {
        Arc::new(self)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl NotificationPreferenceRepository for PostgresNotificationPreferenceRepository {
    async fn get_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
    ) -> Result<Vec<NotificationPreference>, ApiError> {
        let rows = sqlx::query_as::<_, NotificationPreferenceRow>(
            r#"
            SELECT id, tenant_id, user_id, channel, notification_type, enabled, created_at, updated_at
            FROM notification_preferences
            WHERE tenant_id = $1 AND user_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "NotificationPreference"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn is_enabled(
        &self,
        tenant_id: i64,
        user_id: i64,
        channel: NotificationChannel,
        notification_type: &str,
    ) -> Result<bool, ApiError> {
        let enabled: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT enabled FROM notification_preferences
            WHERE tenant_id = $1 AND user_id = $2 AND channel = $3 AND notification_type = $4
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(channel.to_string())
        .bind(notification_type)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "NotificationPreference"))?;

        Ok(enabled.unwrap_or(true))
    }

    async fn update_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
        prefs: Vec<UpdatePreference>,
    ) -> Result<Vec<NotificationPreference>, ApiError> {
        let mut results = Vec::new();

        for pref in prefs {
            let channel = pref
                .channel
                .parse()
                .map_err(|e: String| ApiError::Validation(format!("Invalid channel: {}", e)))?;

            let row: NotificationPreferenceRow = sqlx::query_as(
                r#"
                INSERT INTO notification_preferences
                    (tenant_id, user_id, channel, notification_type, enabled, updated_at)
                VALUES ($1, $2, $3, $4, $5, NOW())
                ON CONFLICT (tenant_id, user_id, channel, notification_type)
                DO UPDATE SET enabled = EXCLUDED.enabled, updated_at = NOW()
                RETURNING id, tenant_id, user_id, channel, notification_type, enabled, created_at, updated_at
                "#,
            )
            .bind(tenant_id)
            .bind(user_id)
            .bind(channel.to_string())
            .bind(&pref.notification_type)
            .bind(pref.enabled)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "NotificationPreference"))?;

            results.push(row.into());
        }

        Ok(results)
    }
}
