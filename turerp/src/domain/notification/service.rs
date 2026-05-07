//! Notification service with async delivery via JobScheduler

use std::sync::Arc;

use chrono::Utc;

use crate::common::{BoxJobScheduler, CreateJob, JobPriority, JobType, PaginatedResult};
use crate::domain::notification::model::{
    InAppNotification, InAppNotificationResponse, Notification, NotificationChannel,
    NotificationPreferenceResponse, NotificationPriority, NotificationRequest,
    NotificationResponse, NotificationStatus, UpdatePreference,
};
use crate::domain::notification::provider::{
    EmailProvider, NoopEmailProvider, NoopSmsProvider, SmsProvider,
};
use crate::domain::notification::repository::{
    BoxInAppNotificationRepository, BoxNotificationPreferenceRepository, BoxNotificationRepository,
};
use crate::domain::notification::template::TemplateEngine;
use crate::error::ApiError;

/// Notification service handling delivery across email, SMS, and in-app channels
pub struct NotificationService {
    notification_repo: BoxNotificationRepository,
    in_app_repo: BoxInAppNotificationRepository,
    preference_repo: BoxNotificationPreferenceRepository,
    job_scheduler: BoxJobScheduler,
    email_provider: Arc<dyn EmailProvider>,
    sms_provider: Arc<dyn SmsProvider>,
    template_engine: parking_lot::RwLock<TemplateEngine>,
}

impl NotificationService {
    pub fn new(
        notification_repo: BoxNotificationRepository,
        in_app_repo: BoxInAppNotificationRepository,
        preference_repo: BoxNotificationPreferenceRepository,
        job_scheduler: BoxJobScheduler,
        email_provider: Arc<dyn EmailProvider>,
        sms_provider: Arc<dyn SmsProvider>,
    ) -> Self {
        Self {
            notification_repo,
            in_app_repo,
            preference_repo,
            job_scheduler,
            email_provider,
            sms_provider,
            template_engine: parking_lot::RwLock::new(TemplateEngine::new()),
        }
    }

    /// Create a service with no-op providers for testing
    pub fn with_noop_providers(
        notification_repo: BoxNotificationRepository,
        in_app_repo: BoxInAppNotificationRepository,
        preference_repo: BoxNotificationPreferenceRepository,
        job_scheduler: BoxJobScheduler,
    ) -> Self {
        Self::new(
            notification_repo,
            in_app_repo,
            preference_repo,
            job_scheduler,
            Arc::new(NoopEmailProvider::new()),
            Arc::new(NoopSmsProvider::new()),
        )
    }

    /// Send a notification (queues job for async delivery)
    pub async fn send(
        &self,
        request: NotificationRequest,
    ) -> Result<NotificationResponse, ApiError> {
        let (subject, body, _html) = self
            .template_engine
            .read()
            .render(&request.template_key, &request.template_vars)
            .unwrap_or_else(|_| {
                (
                    format!("Notification: {}", request.template_key),
                    format!(
                        "Template '{}' not found. Variables: {}",
                        request.template_key, request.template_vars
                    ),
                    None,
                )
            });

        // Check preferences before queuing
        if let Some(user_id) = request.user_id {
            let enabled = self
                .preference_repo
                .is_enabled(
                    request.tenant_id,
                    user_id,
                    request.channel,
                    &request.template_key,
                )
                .await?;
            if !enabled {
                let notification = Notification {
                    id: 0,
                    tenant_id: request.tenant_id,
                    user_id: request.user_id,
                    channel: request.channel,
                    priority: request.priority,
                    status: NotificationStatus::Cancelled,
                    notification_type: request.template_key.clone(),
                    subject: subject.clone(),
                    body: body.clone(),
                    recipient: request.recipient.clone(),
                    template_key: Some(request.template_key.clone()),
                    template_vars: Some(request.template_vars.clone()),
                    provider_message_id: None,
                    created_at: Utc::now(),
                    sent_at: None,
                    read_at: None,
                    last_error: Some("Disabled by user preference".to_string()),
                    attempts: 0,
                    job_id: None,
                    deleted_at: None,
                    deleted_by: None,
                };
                let saved = self.notification_repo.create(notification).await?;
                return Ok(saved.into());
            }
        }

        let notification = Notification {
            id: 0,
            tenant_id: request.tenant_id,
            user_id: request.user_id,
            channel: request.channel,
            priority: request.priority,
            status: NotificationStatus::Queued,
            notification_type: request.template_key.clone(),
            subject: subject.clone(),
            body: body.clone(),
            recipient: request.recipient.clone(),
            template_key: Some(request.template_key.clone()),
            template_vars: Some(request.template_vars.clone()),
            provider_message_id: None,
            created_at: Utc::now(),
            sent_at: None,
            read_at: None,
            last_error: None,
            attempts: 0,
            job_id: None,
            deleted_at: None,
            deleted_by: None,
        };

        let saved = self.notification_repo.create(notification).await?;

        // Create in-app notification if channel is InApp
        if request.channel == NotificationChannel::InApp {
            if let Some(user_id) = request.user_id {
                let in_app = InAppNotification {
                    id: 0,
                    tenant_id: request.tenant_id,
                    user_id,
                    title: subject.clone(),
                    message: body.clone(),
                    notification_type: request.template_key.clone(),
                    read: false,
                    created_at: Utc::now(),
                    read_at: None,
                    link: None,
                    related_notification_id: Some(saved.id),
                    deleted_at: None,
                    deleted_by: None,
                };
                self.in_app_repo.create(in_app).await?;
            }
        }

        // Schedule background job for delivery
        let job = CreateJob::new(
            JobType::SendNotification {
                notification_id: saved.id,
                tenant_id: request.tenant_id,
            },
            request.tenant_id,
        )
        .with_priority(match request.priority {
            NotificationPriority::Urgent => JobPriority::Critical,
            NotificationPriority::High => JobPriority::High,
            NotificationPriority::Normal => JobPriority::Normal,
            NotificationPriority::Low => JobPriority::Low,
        });

        let scheduled = self.job_scheduler.schedule(job).await.map_err(|e| {
            ApiError::Internal(format!("Failed to schedule notification job: {}", e))
        })?;

        // Update notification with job_id
        self.notification_repo
            .update_status(
                saved.id,
                request.tenant_id,
                NotificationStatus::Queued,
                None,
                None,
            )
            .await?;

        let mut response: NotificationResponse = saved.into();
        response.status = NotificationStatus::Queued.to_string();
        response.job_id = Some(scheduled.id);
        Ok(response)
    }

    /// Get a notification by ID
    pub async fn get_notification(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<NotificationResponse>, ApiError> {
        let n = self.notification_repo.find_by_id(id, tenant_id).await?;
        Ok(n.map(Into::into))
    }

    /// Get notification history for a user
    pub async fn get_history(
        &self,
        tenant_id: i64,
        user_id: Option<i64>,
        channel: Option<NotificationChannel>,
        limit: i64,
        offset: i64,
    ) -> Result<PaginatedResult<NotificationResponse>, ApiError> {
        if limit <= 0 {
            return Err(ApiError::BadRequest(
                "Limit must be greater than 0".to_string(),
            ));
        }
        let result = self
            .notification_repo
            .find_by_user(tenant_id, user_id, channel, limit, offset)
            .await?;
        Ok(result.map(Into::into))
    }

    /// Get in-app notifications for a user
    pub async fn get_in_app_notifications(
        &self,
        tenant_id: i64,
        user_id: i64,
        unread_only: bool,
    ) -> Result<Vec<InAppNotificationResponse>, ApiError> {
        let notifications = self
            .in_app_repo
            .find_by_user(tenant_id, user_id, unread_only)
            .await?;
        Ok(notifications.into_iter().map(Into::into).collect())
    }

    /// Mark an in-app notification as read
    pub async fn mark_as_read(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.in_app_repo.mark_as_read(id, tenant_id).await?;
        self.notification_repo
            .update_status(id, tenant_id, NotificationStatus::Read, None, None)
            .await?;
        Ok(())
    }

    /// Mark all notifications as read for a user
    pub async fn mark_all_as_read(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError> {
        self.in_app_repo.mark_all_as_read(tenant_id, user_id).await
    }

    /// Get unread notification count for a user
    pub async fn unread_count(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError> {
        self.in_app_repo.unread_count(tenant_id, user_id).await
    }

    /// Get user preferences
    pub async fn get_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
    ) -> Result<Vec<NotificationPreferenceResponse>, ApiError> {
        let prefs = self
            .preference_repo
            .get_preferences(tenant_id, user_id)
            .await?;
        Ok(prefs.into_iter().map(Into::into).collect())
    }

    /// Update user preferences
    pub async fn update_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
        prefs: Vec<UpdatePreference>,
    ) -> Result<Vec<NotificationPreferenceResponse>, ApiError> {
        let updated = self
            .preference_repo
            .update_preferences(tenant_id, user_id, prefs)
            .await?;
        Ok(updated.into_iter().map(Into::into).collect())
    }

    /// Retry a failed notification
    pub async fn retry(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let notification = self
            .notification_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Notification {} not found", id)))?;

        if notification.status != NotificationStatus::Failed {
            return Err(ApiError::BadRequest(
                "Can only retry failed notifications".to_string(),
            ));
        }

        self.notification_repo
            .update_status(id, tenant_id, NotificationStatus::Queued, None, None)
            .await?;

        let job = CreateJob::new(
            JobType::SendNotification {
                notification_id: id,
                tenant_id,
            },
            tenant_id,
        );

        self.job_scheduler
            .schedule(job)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to schedule retry job: {}", e)))?;

        Ok(())
    }

    /// Execute notification delivery (called by job worker)
    pub async fn execute_delivery(
        &self,
        notification_id: i64,
        tenant_id: i64,
    ) -> Result<(), ApiError> {
        let notification = self
            .notification_repo
            .find_by_id(notification_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Notification {} not found", notification_id))
            })?;

        self.notification_repo
            .increment_attempt(notification_id, tenant_id)
            .await?;

        let result = match notification.channel {
            NotificationChannel::Email => {
                self.email_provider
                    .send_email(
                        &notification.recipient,
                        &notification.subject,
                        &notification.body,
                        None,
                    )
                    .await
            }
            NotificationChannel::Sms => {
                self.sms_provider
                    .send_sms(&notification.recipient, &notification.body)
                    .await
            }
            NotificationChannel::InApp => {
                // In-app is already created during send()
                Ok("in-app".to_string())
            }
        };

        match result {
            Ok(provider_id) => {
                self.notification_repo
                    .update_status(
                        notification_id,
                        tenant_id,
                        NotificationStatus::Sent,
                        Some(provider_id),
                        None,
                    )
                    .await?;
                Ok(())
            }
            Err(e) => {
                self.notification_repo
                    .update_status(
                        notification_id,
                        tenant_id,
                        NotificationStatus::Failed,
                        None,
                        Some(e.to_string()),
                    )
                    .await?;
                Err(e)
            }
        }
    }

    /// Register a custom email template
    pub fn register_template(
        &mut self,
        key: &str,
        subject: &str,
        body: &str,
        html: Option<&str>,
    ) -> Result<(), ApiError> {
        self.template_engine
            .write()
            .register(key, subject, body, html)
    }

    /// Soft delete a notification
    pub async fn soft_delete(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.notification_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    /// Restore a soft-deleted notification
    pub async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.notification_repo.restore(id, tenant_id).await
    }

    /// List deleted notifications for a tenant
    pub async fn find_deleted(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<NotificationResponse>, ApiError> {
        let notifications = self.notification_repo.find_deleted(tenant_id).await?;
        Ok(notifications.into_iter().map(Into::into).collect())
    }

    /// Permanently destroy a soft-deleted notification
    pub async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.notification_repo.destroy(id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::InMemoryJobScheduler;
    use crate::domain::notification::repository::{
        InMemoryInAppNotificationRepository, InMemoryNotificationPreferenceRepository,
        InMemoryNotificationRepository,
    };

    fn make_service() -> NotificationService {
        let repo = Arc::new(InMemoryNotificationRepository::new()) as BoxNotificationRepository;
        let in_app =
            Arc::new(InMemoryInAppNotificationRepository::new()) as BoxInAppNotificationRepository;
        let prefs = Arc::new(InMemoryNotificationPreferenceRepository::new())
            as BoxNotificationPreferenceRepository;
        let jobs = Arc::new(InMemoryJobScheduler::new()) as BoxJobScheduler;
        NotificationService::with_noop_providers(repo, in_app, prefs, jobs)
    }

    #[tokio::test]
    async fn test_send_email_notification() {
        let svc = make_service();
        let request = NotificationRequest {
            tenant_id: 1,
            user_id: Some(1),
            channel: NotificationChannel::Email,
            priority: NotificationPriority::Normal,
            template_key: "invoice_created".to_string(),
            template_vars: serde_json::json!({
                "customer_name": "Acme Corp",
                "invoice_number": "INV-001",
                "amount": "1000.00",
                "currency": "TRY",
                "due_date": "2024-02-01"
            }),
            recipient: "test@example.com".to_string(),
        };

        let notification = svc.send(request).await.unwrap();
        assert_eq!(notification.tenant_id, 1);
        assert!(notification.subject.contains("INV-001"));
    }

    #[tokio::test]
    async fn test_send_in_app_notification() {
        let svc = make_service();
        let request = NotificationRequest {
            tenant_id: 1,
            user_id: Some(1),
            channel: NotificationChannel::InApp,
            priority: NotificationPriority::High,
            template_key: "payment_received".to_string(),
            template_vars: serde_json::json!({
                "customer_name": "Beta Inc",
                "payment_id": "PAY-001",
                "amount": "5000.00",
                "currency": "TRY",
                "payment_date": "2024-01-15"
            }),
            recipient: "user1@example.com".to_string(),
        };

        let _ = svc.send(request).await.unwrap();
        let notifications = svc.get_in_app_notifications(1, 1, false).await.unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(!notifications[0].read);
    }

    #[tokio::test]
    async fn test_mark_as_read() {
        let svc = make_service();
        let request = NotificationRequest {
            tenant_id: 1,
            user_id: Some(1),
            channel: NotificationChannel::InApp,
            priority: NotificationPriority::Normal,
            template_key: "employee_hired".to_string(),
            template_vars: serde_json::json!({
                "employee_name": "John Doe",
                "department": "Engineering",
                "start_date": "2024-01-01"
            }),
            recipient: "hr@example.com".to_string(),
        };

        let n = svc.send(request).await.unwrap();
        let count_before = svc.unread_count(1, 1).await.unwrap();
        assert_eq!(count_before, 1);

        svc.mark_as_read(n.id, 1).await.unwrap();

        let count_after = svc.unread_count(1, 1).await.unwrap();
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_preference_disable() {
        let svc = make_service();
        let request = NotificationRequest {
            tenant_id: 1,
            user_id: Some(1),
            channel: NotificationChannel::Email,
            priority: NotificationPriority::Normal,
            template_key: "invoice_created".to_string(),
            template_vars: serde_json::json!({"customer_name": "Test"}),
            recipient: "test@example.com".to_string(),
        };

        // Disable email notifications for invoice_created
        svc.update_preferences(
            1,
            1,
            vec![UpdatePreference {
                channel: "email".to_string(),
                notification_type: "invoice_created".to_string(),
                enabled: false,
            }],
        )
        .await
        .unwrap();

        let notification = svc.send(request).await.unwrap();
        assert_eq!(notification.status, "cancelled");
    }

    #[tokio::test]
    async fn test_retry_failed_notification() {
        let svc = make_service();
        let request = NotificationRequest {
            tenant_id: 1,
            user_id: None,
            channel: NotificationChannel::Email,
            priority: NotificationPriority::Normal,
            template_key: "invoice_created".to_string(),
            template_vars: serde_json::json!({"customer_name": "Test"}),
            recipient: "test@example.com".to_string(),
        };

        let notification = svc.send(request).await.unwrap();
        let id = notification.id;

        // Simulate failure by updating status directly
        svc.notification_repo
            .update_status(
                id,
                1,
                NotificationStatus::Failed,
                None,
                Some("SMTP error".to_string()),
            )
            .await
            .unwrap();

        svc.retry(id, 1).await.unwrap();

        let after_retry = svc.get_notification(id, 1).await.unwrap().unwrap();
        assert_eq!(after_retry.status, "queued");
    }

    #[tokio::test]
    async fn test_execute_delivery() {
        let svc = make_service();
        let request = NotificationRequest {
            tenant_id: 1,
            user_id: None,
            channel: NotificationChannel::Email,
            priority: NotificationPriority::Normal,
            template_key: "payment_received".to_string(),
            template_vars: serde_json::json!({"customer_name": "Test"}),
            recipient: "test@example.com".to_string(),
        };

        let notification = svc.send(request).await.unwrap();
        let id = notification.id;

        svc.execute_delivery(id, 1).await.unwrap();

        let after = svc.get_notification(id, 1).await.unwrap().unwrap();
        assert_eq!(after.status, "sent");
        assert!(after.sent_at.is_some());
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let svc = make_service();
        for tenant_id in [1, 2] {
            let request = NotificationRequest {
                tenant_id,
                user_id: Some(tenant_id),
                channel: NotificationChannel::InApp,
                priority: NotificationPriority::Normal,
                template_key: "password_reset".to_string(),
                template_vars: serde_json::json!({
                    "reset_link": format!("https://example.com/reset/{}", tenant_id),
                    "expiry_minutes": "30"
                }),
                recipient: format!("user{}@example.com", tenant_id),
            };
            svc.send(request).await.unwrap();
        }

        let tenant1 = svc.get_in_app_notifications(1, 1, false).await.unwrap();
        let tenant2 = svc.get_in_app_notifications(2, 2, false).await.unwrap();
        assert_eq!(tenant1.len(), 1);
        assert_eq!(tenant2.len(), 1);
    }

    #[tokio::test]
    async fn test_get_history() {
        let svc = make_service();
        for i in 0..5 {
            let request = NotificationRequest {
                tenant_id: 1,
                user_id: Some(1),
                channel: NotificationChannel::Email,
                priority: NotificationPriority::Normal,
                template_key: "invoice_created".to_string(),
                template_vars: serde_json::json!({"customer_name": format!("Customer {}", i)}),
                recipient: format!("customer{}@example.com", i),
            };
            svc.send(request).await.unwrap();
        }

        let history = svc.get_history(1, Some(1), None, 10, 0).await.unwrap();
        assert_eq!(history.items.len(), 5);
        assert_eq!(history.total, 5);
    }
}
