//! Notification repository traits and in-memory implementations

use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;

use crate::common::soft_delete::SoftDeletable;
use crate::common::PaginatedResult;
use crate::domain::notification::model::{
    InAppNotification, Notification, NotificationChannel, NotificationPreference,
    NotificationStatus, UpdatePreference,
};
use crate::error::ApiError;

/// Repository trait for notification records
#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn create(&self, notification: Notification) -> Result<Notification, ApiError>;

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Notification>, ApiError>;

    async fn find_by_user(
        &self,
        tenant_id: i64,
        user_id: Option<i64>,
        channel: Option<NotificationChannel>,
        limit: i64,
        offset: i64,
    ) -> Result<PaginatedResult<Notification>, ApiError>;

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: NotificationStatus,
        provider_message_id: Option<String>,
        last_error: Option<String>,
    ) -> Result<(), ApiError>;

    async fn increment_attempt(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Notification>, ApiError>;

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for in-app notifications
#[async_trait]
pub trait InAppNotificationRepository: Send + Sync {
    async fn create(&self, notification: InAppNotification) -> Result<InAppNotification, ApiError>;

    async fn find_by_user(
        &self,
        tenant_id: i64,
        user_id: i64,
        unread_only: bool,
    ) -> Result<Vec<InAppNotification>, ApiError>;

    async fn mark_as_read(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    async fn mark_all_as_read(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError>;

    async fn unread_count(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError>;

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<InAppNotification>, ApiError>;

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for notification preferences
#[async_trait]
pub trait NotificationPreferenceRepository: Send + Sync {
    async fn get_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
    ) -> Result<Vec<NotificationPreference>, ApiError>;

    async fn is_enabled(
        &self,
        tenant_id: i64,
        user_id: i64,
        channel: NotificationChannel,
        notification_type: &str,
    ) -> Result<bool, ApiError>;

    async fn update_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
        prefs: Vec<UpdatePreference>,
    ) -> Result<Vec<NotificationPreference>, ApiError>;
}

pub type BoxNotificationRepository = Arc<dyn NotificationRepository>;
pub type BoxInAppNotificationRepository = Arc<dyn InAppNotificationRepository>;
pub type BoxNotificationPreferenceRepository = Arc<dyn NotificationPreferenceRepository>;

// --- InMemory implementations ---

struct NotificationInner {
    notifications: Vec<Notification>,
    next_id: i64,
}

pub struct InMemoryNotificationRepository {
    inner: RwLock<NotificationInner>,
}

impl InMemoryNotificationRepository {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(NotificationInner {
                notifications: Vec::new(),
                next_id: 1,
            }),
        }
    }

    #[allow(dead_code)]
    fn allocate_id(&self) -> i64 {
        let mut inner = self.inner.write();
        let id = inner.next_id;
        inner.next_id += 1;
        id
    }
}

impl Default for InMemoryNotificationRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NotificationRepository for InMemoryNotificationRepository {
    async fn create(&self, mut notification: Notification) -> Result<Notification, ApiError> {
        let mut inner = self.inner.write();
        notification.id = inner.next_id;
        inner.next_id += 1;
        inner.notifications.push(notification.clone());
        Ok(notification)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Notification>, ApiError> {
        let inner = self.inner.read();
        Ok(inner
            .notifications
            .iter()
            .find(|n| n.id == id && n.tenant_id == tenant_id && !n.is_deleted())
            .cloned())
    }

    async fn find_by_user(
        &self,
        tenant_id: i64,
        user_id: Option<i64>,
        channel: Option<NotificationChannel>,
        limit: i64,
        offset: i64,
    ) -> Result<PaginatedResult<Notification>, ApiError> {
        let inner = self.inner.read();
        let mut filtered: Vec<_> = inner
            .notifications
            .iter()
            .filter(|n| n.tenant_id == tenant_id)
            .filter(|n| !n.is_deleted())
            .filter(|n| user_id.is_none() || n.user_id == user_id)
            .filter(|n| channel.is_none() || n.channel == channel.unwrap())
            .cloned()
            .collect();

        filtered.sort_by_key(|b| std::cmp::Reverse(b.created_at));

        let total = filtered.len() as u64;
        let items = filtered
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect();

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
        let mut inner = self.inner.write();
        let n = inner
            .notifications
            .iter_mut()
            .find(|n| n.id == id && n.tenant_id == tenant_id && !n.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Notification {} not found", id)))?;
        n.status = status;
        if let Some(mid) = provider_message_id {
            n.provider_message_id = Some(mid);
        }
        if let Some(err) = last_error {
            n.last_error = Some(err);
        }
        if status == NotificationStatus::Sent || status == NotificationStatus::Delivered {
            n.sent_at = Some(chrono::Utc::now());
        }
        Ok(())
    }

    async fn increment_attempt(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let n = inner
            .notifications
            .iter_mut()
            .find(|n| n.id == id && n.tenant_id == tenant_id && !n.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Notification {} not found", id)))?;
        n.attempts += 1;
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let n = inner
            .notifications
            .iter_mut()
            .find(|n| n.id == id && n.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Notification {} not found", id)))?;

        if n.is_deleted() {
            return Err(ApiError::Conflict(format!(
                "Notification {} is already deleted",
                id
            )));
        }

        n.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let n = inner
            .notifications
            .iter_mut()
            .find(|n| n.id == id && n.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Notification {} not found", id)))?;

        if !n.is_deleted() {
            return Err(ApiError::BadRequest(format!(
                "Notification {} is not deleted",
                id
            )));
        }

        n.restore();
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Notification>, ApiError> {
        let inner = self.inner.read();
        let mut results: Vec<_> = inner
            .notifications
            .iter()
            .filter(|n| n.tenant_id == tenant_id && n.is_deleted())
            .cloned()
            .collect();
        results.sort_by_key(|n| std::cmp::Reverse(n.deleted_at.unwrap()));
        Ok(results)
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let len_before = inner.notifications.len();
        inner
            .notifications
            .retain(|n| !(n.id == id && n.tenant_id == tenant_id && n.is_deleted()));

        if inner.notifications.len() == len_before {
            return Err(ApiError::NotFound(format!(
                "Deleted notification {} not found",
                id
            )));
        }
        Ok(())
    }
}

struct InAppInner {
    notifications: Vec<InAppNotification>,
    next_id: i64,
}

pub struct InMemoryInAppNotificationRepository {
    inner: RwLock<InAppInner>,
}

impl InMemoryInAppNotificationRepository {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(InAppInner {
                notifications: Vec::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryInAppNotificationRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InAppNotificationRepository for InMemoryInAppNotificationRepository {
    async fn create(
        &self,
        mut notification: InAppNotification,
    ) -> Result<InAppNotification, ApiError> {
        let mut inner = self.inner.write();
        notification.id = inner.next_id;
        inner.next_id += 1;
        inner.notifications.push(notification.clone());
        Ok(notification)
    }

    async fn find_by_user(
        &self,
        tenant_id: i64,
        user_id: i64,
        unread_only: bool,
    ) -> Result<Vec<InAppNotification>, ApiError> {
        let inner = self.inner.read();
        let mut filtered: Vec<_> = inner
            .notifications
            .iter()
            .filter(|n| n.tenant_id == tenant_id && n.user_id == user_id && !n.is_deleted())
            .filter(|n| !unread_only || !n.read)
            .cloned()
            .collect();
        filtered.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        Ok(filtered)
    }

    async fn mark_as_read(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let n = inner
            .notifications
            .iter_mut()
            .find(|n| n.id == id && n.tenant_id == tenant_id && !n.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("In-app notification {} not found", id)))?;
        n.read = true;
        n.read_at = Some(chrono::Utc::now());
        Ok(())
    }

    async fn mark_all_as_read(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError> {
        let mut inner = self.inner.write();
        let mut count = 0u64;
        for n in inner.notifications.iter_mut() {
            if n.tenant_id == tenant_id && n.user_id == user_id && !n.read && !n.is_deleted() {
                n.read = true;
                n.read_at = Some(chrono::Utc::now());
                count += 1;
            }
        }
        Ok(count)
    }

    async fn unread_count(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError> {
        let inner = self.inner.read();
        Ok(inner
            .notifications
            .iter()
            .filter(|n| {
                n.tenant_id == tenant_id && n.user_id == user_id && !n.read && !n.is_deleted()
            })
            .count() as u64)
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let n = inner
            .notifications
            .iter_mut()
            .find(|n| n.id == id && n.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("In-app notification {} not found", id)))?;

        if n.is_deleted() {
            return Err(ApiError::Conflict(format!(
                "In-app notification {} is already deleted",
                id
            )));
        }

        n.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let n = inner
            .notifications
            .iter_mut()
            .find(|n| n.id == id && n.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("In-app notification {} not found", id)))?;

        if !n.is_deleted() {
            return Err(ApiError::BadRequest(format!(
                "In-app notification {} is not deleted",
                id
            )));
        }

        n.restore();
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<InAppNotification>, ApiError> {
        let inner = self.inner.read();
        let mut results: Vec<_> = inner
            .notifications
            .iter()
            .filter(|n| n.tenant_id == tenant_id && n.is_deleted())
            .cloned()
            .collect();
        results.sort_by_key(|n| std::cmp::Reverse(n.deleted_at.unwrap()));
        Ok(results)
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.write();
        let len_before = inner.notifications.len();
        inner
            .notifications
            .retain(|n| !(n.id == id && n.tenant_id == tenant_id && n.is_deleted()));

        if inner.notifications.len() == len_before {
            return Err(ApiError::NotFound(format!(
                "Deleted in-app notification {} not found",
                id
            )));
        }
        Ok(())
    }
}

struct PreferenceInner {
    preferences: Vec<NotificationPreference>,
    next_id: i64,
}

pub struct InMemoryNotificationPreferenceRepository {
    inner: RwLock<PreferenceInner>,
}

impl InMemoryNotificationPreferenceRepository {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(PreferenceInner {
                preferences: Vec::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryNotificationPreferenceRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NotificationPreferenceRepository for InMemoryNotificationPreferenceRepository {
    async fn get_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
    ) -> Result<Vec<NotificationPreference>, ApiError> {
        let inner = self.inner.read();
        Ok(inner
            .preferences
            .iter()
            .filter(|p| p.tenant_id == tenant_id && p.user_id == user_id && !p.is_deleted())
            .cloned()
            .collect())
    }

    async fn is_enabled(
        &self,
        tenant_id: i64,
        user_id: i64,
        channel: NotificationChannel,
        notification_type: &str,
    ) -> Result<bool, ApiError> {
        let inner = self.inner.read();
        let pref = inner.preferences.iter().find(|p| {
            p.tenant_id == tenant_id
                && p.user_id == user_id
                && p.channel == channel
                && p.notification_type == notification_type
                && !p.is_deleted()
        });
        Ok(pref.map(|p| p.enabled).unwrap_or(true))
    }

    async fn update_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
        prefs: Vec<UpdatePreference>,
    ) -> Result<Vec<NotificationPreference>, ApiError> {
        let mut inner = self.inner.write();
        let mut results = Vec::new();

        for pref in prefs {
            let channel = pref
                .channel
                .parse()
                .map_err(|e: String| ApiError::Validation(format!("Invalid channel: {}", e)))?;

            if let Some(existing) = inner.preferences.iter_mut().find(|p| {
                p.tenant_id == tenant_id
                    && p.user_id == user_id
                    && p.channel == channel
                    && p.notification_type == pref.notification_type
                    && !p.is_deleted()
            }) {
                existing.enabled = pref.enabled;
                existing.updated_at = chrono::Utc::now();
                results.push(existing.clone());
            } else {
                let new_pref = NotificationPreference {
                    id: inner.next_id,
                    tenant_id,
                    user_id,
                    channel,
                    notification_type: pref.notification_type,
                    enabled: pref.enabled,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    deleted_at: None,
                    deleted_by: None,
                };
                inner.next_id += 1;
                inner.preferences.push(new_pref.clone());
                results.push(new_pref);
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::notification::model::NotificationPriority;

    fn make_notification(tenant_id: i64, user_id: Option<i64>) -> Notification {
        Notification {
            id: 0,
            tenant_id,
            user_id,
            channel: NotificationChannel::Email,
            priority: NotificationPriority::Normal,
            status: NotificationStatus::Queued,
            notification_type: "test".to_string(),
            subject: "Subject".to_string(),
            body: "Body".to_string(),
            recipient: "test@example.com".to_string(),
            template_key: None,
            template_vars: None,
            provider_message_id: None,
            created_at: chrono::Utc::now(),
            sent_at: None,
            read_at: None,
            last_error: None,
            attempts: 0,
            job_id: None,
            deleted_at: None,
            deleted_by: None,
        }
    }

    #[tokio::test]
    async fn test_notification_create_and_find() {
        let repo = InMemoryNotificationRepository::new();
        let n = repo.create(make_notification(1, Some(1))).await.unwrap();
        assert_eq!(n.id, 1);

        let found = repo.find_by_id(1, 1).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_notification_update_status() {
        let repo = InMemoryNotificationRepository::new();
        let n = repo.create(make_notification(1, None)).await.unwrap();

        repo.update_status(
            n.id,
            1,
            NotificationStatus::Sent,
            Some("msg-id".to_string()),
            None,
        )
        .await
        .unwrap();

        let found = repo.find_by_id(n.id, 1).await.unwrap().unwrap();
        assert_eq!(found.status, NotificationStatus::Sent);
        assert_eq!(found.provider_message_id, Some("msg-id".to_string()));
        assert!(found.sent_at.is_some());
    }

    #[tokio::test]
    async fn test_in_app_create_and_find() {
        let repo = InMemoryInAppNotificationRepository::new();
        let n = InAppNotification {
            id: 0,
            tenant_id: 1,
            user_id: 1,
            title: "Title".to_string(),
            message: "Message".to_string(),
            notification_type: "test".to_string(),
            read: false,
            created_at: chrono::Utc::now(),
            read_at: None,
            link: None,
            related_notification_id: None,
            deleted_at: None,
            deleted_by: None,
        };
        let created = repo.create(n).await.unwrap();
        assert_eq!(created.id, 1);

        let found = repo.find_by_user(1, 1, false).await.unwrap();
        assert_eq!(found.len(), 1);
    }

    #[tokio::test]
    async fn test_in_app_mark_as_read() {
        let repo = InMemoryInAppNotificationRepository::new();
        let n = InAppNotification {
            id: 0,
            tenant_id: 1,
            user_id: 1,
            title: "Title".to_string(),
            message: "Message".to_string(),
            notification_type: "test".to_string(),
            read: false,
            created_at: chrono::Utc::now(),
            read_at: None,
            link: None,
            related_notification_id: None,
            deleted_at: None,
            deleted_by: None,
        };
        repo.create(n).await.unwrap();

        let count_before = repo.unread_count(1, 1).await.unwrap();
        assert_eq!(count_before, 1);

        repo.mark_as_read(1, 1).await.unwrap();

        let count_after = repo.unread_count(1, 1).await.unwrap();
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_preference_update() {
        let repo = InMemoryNotificationPreferenceRepository::new();
        let prefs = vec![UpdatePreference {
            channel: "email".to_string(),
            notification_type: "invoice_created".to_string(),
            enabled: false,
        }];

        let updated = repo.update_preferences(1, 1, prefs).await.unwrap();
        assert_eq!(updated.len(), 1);
        assert!(!updated[0].enabled);

        let enabled = repo
            .is_enabled(1, 1, NotificationChannel::Email, "invoice_created")
            .await
            .unwrap();
        assert!(!enabled);
    }

    #[tokio::test]
    async fn test_preference_default_enabled() {
        let repo = InMemoryNotificationPreferenceRepository::new();
        let enabled = repo
            .is_enabled(1, 1, NotificationChannel::Email, "unknown")
            .await
            .unwrap();
        assert!(enabled);
    }
}
