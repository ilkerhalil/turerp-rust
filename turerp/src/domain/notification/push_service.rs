//! Push notification service with mock FCM integration

use std::sync::Arc;

use chrono::Utc;

use crate::domain::notification::push_repository::{
    BoxPushTokenRepository, InMemoryPushTokenRepository,
};
use crate::domain::notification::push_token::{
    DeviceType, PushMessage, PushToken, RegisterPushToken,
};
use crate::error::ApiError;

/// Service for managing push tokens and sending push notifications
pub struct PushNotificationService {
    repo: BoxPushTokenRepository,
}

impl PushNotificationService {
    pub fn new(repo: BoxPushTokenRepository) -> Self {
        Self { repo }
    }

    /// Create a service with an in-memory repository for testing
    pub fn in_memory() -> Self {
        Self::new(Arc::new(InMemoryPushTokenRepository::new()))
    }

    /// Register a new push token for a user
    pub async fn register_token(
        &self,
        tenant_id: i64,
        token: RegisterPushToken,
    ) -> Result<PushToken, ApiError> {
        let device_type = token
            .device_type
            .parse::<DeviceType>()
            .map_err(|e| ApiError::Validation(format!("Invalid device type: {}", e)))?;

        // Check if token already exists for this tenant
        if let Some(existing) = self.repo.find_by_token(tenant_id, &token.token).await? {
            if existing.user_id == token.user_id && existing.device_type == device_type {
                // Reactivate if same user and device
                let mut updated = existing;
                updated.is_active = true;
                updated.updated_at = Utc::now();
                return self.repo.update(updated).await;
            }
        }

        // Deactivate any existing token for the same user + device_type
        let existing_tokens = self.repo.find_by_user(tenant_id, token.user_id).await?;
        for t in existing_tokens {
            if t.device_type == device_type && t.is_active {
                self.repo.deactivate(t.id, tenant_id).await?;
            }
        }

        let push_token = PushToken {
            id: 0,
            tenant_id,
            user_id: token.user_id,
            device_type,
            token: token.token,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };

        self.repo.create(push_token).await
    }

    /// Unregister (deactivate) a push token for a user by device type
    pub async fn unregister_token(
        &self,
        tenant_id: i64,
        user_id: i64,
        device_type: DeviceType,
    ) -> Result<(), ApiError> {
        let tokens = self.repo.find_by_user(tenant_id, user_id).await?;
        for token in tokens {
            if token.device_type == device_type {
                self.repo.deactivate(token.id, tenant_id).await?;
            }
        }
        Ok(())
    }

    /// Send a push notification to a specific user
    pub async fn send_push(
        &self,
        tenant_id: i64,
        user_id: i64,
        message: PushMessage,
    ) -> Result<(), ApiError> {
        let tokens = self.repo.find_by_user(tenant_id, user_id).await?;
        if tokens.is_empty() {
            return Err(ApiError::NotFound(format!(
                "No active push tokens found for user {}",
                user_id
            )));
        }

        for token in tokens {
            // Mock FCM send - log and return Ok
            tracing::info!(
                "[MOCK FCM] Sending push to user {} (device: {}, token: {}): title='{}' body='{}'",
                user_id,
                token.device_type,
                token.token,
                message.title,
                message.body
            );
        }

        Ok(())
    }

    /// Broadcast a push notification to all active users in a tenant
    pub async fn send_broadcast(
        &self,
        tenant_id: i64,
        message: PushMessage,
    ) -> Result<Vec<i64>, ApiError> {
        let tokens = self.repo.find_all_for_tenant(tenant_id).await?;
        let mut sent_to = Vec::with_capacity(tokens.len());
        let mut seen_users = std::collections::HashSet::new();

        for token in tokens {
            if token.is_active && seen_users.insert(token.user_id) {
                tracing::info!(
                    "[MOCK FCM] Broadcasting to user {} (device: {}): title='{}' body='{}'",
                    token.user_id,
                    token.device_type,
                    message.title,
                    message.body
                );
                sent_to.push(token.user_id);
            }
        }

        Ok(sent_to)
    }

    /// Get all active push tokens for a user
    pub async fn get_user_tokens(
        &self,
        tenant_id: i64,
        user_id: i64,
    ) -> Result<Vec<PushToken>, ApiError> {
        self.repo.find_by_user(tenant_id, user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service() -> PushNotificationService {
        PushNotificationService::in_memory()
    }

    #[tokio::test]
    async fn test_register_token() {
        let svc = make_service();
        let token = RegisterPushToken {
            user_id: 1,
            device_type: "ios".to_string(),
            token: "apns_token_123".to_string(),
        };

        let result = svc.register_token(1, token).await.unwrap();
        assert_eq!(result.user_id, 1);
        assert_eq!(result.device_type, DeviceType::Ios);
        assert!(result.is_active);
    }

    #[tokio::test]
    async fn test_unregister_token() {
        let svc = make_service();
        let token = RegisterPushToken {
            user_id: 1,
            device_type: "android".to_string(),
            token: "fcm_token_456".to_string(),
        };
        svc.register_token(1, token).await.unwrap();

        svc.unregister_token(1, 1, DeviceType::Android)
            .await
            .unwrap();

        let tokens = svc.get_user_tokens(1, 1).await.unwrap();
        assert!(tokens.is_empty());
    }

    #[tokio::test]
    async fn test_send_push() {
        let svc = make_service();
        let token = RegisterPushToken {
            user_id: 1,
            device_type: "ios".to_string(),
            token: "apns_token_789".to_string(),
        };
        svc.register_token(1, token).await.unwrap();

        let message = PushMessage {
            title: "Test".to_string(),
            body: "Hello".to_string(),
            data: None,
            badge: Some(1),
            sound: Some("default".to_string()),
        };

        svc.send_push(1, 1, message).await.unwrap();
    }

    #[tokio::test]
    async fn test_send_push_no_tokens() {
        let svc = make_service();
        let message = PushMessage {
            title: "Test".to_string(),
            body: "Hello".to_string(),
            data: None,
            badge: None,
            sound: None,
        };

        let result = svc.send_push(1, 99, message).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_broadcast() {
        let svc = make_service();
        svc.register_token(
            1,
            RegisterPushToken {
                user_id: 1,
                device_type: "ios".to_string(),
                token: "t1".to_string(),
            },
        )
        .await
        .unwrap();
        svc.register_token(
            1,
            RegisterPushToken {
                user_id: 2,
                device_type: "android".to_string(),
                token: "t2".to_string(),
            },
        )
        .await
        .unwrap();

        let message = PushMessage {
            title: "Broadcast".to_string(),
            body: "To all".to_string(),
            data: None,
            badge: None,
            sound: None,
        };

        let sent_to = svc.send_broadcast(1, message).await.unwrap();
        assert_eq!(sent_to.len(), 2);
        assert!(sent_to.contains(&1));
        assert!(sent_to.contains(&2));
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let svc = make_service();
        svc.register_token(
            1,
            RegisterPushToken {
                user_id: 1,
                device_type: "web".to_string(),
                token: "web_t".to_string(),
            },
        )
        .await
        .unwrap();

        let tenant1 = svc.get_user_tokens(1, 1).await.unwrap();
        let tenant2 = svc.get_user_tokens(2, 1).await.unwrap();
        assert_eq!(tenant1.len(), 1);
        assert!(tenant2.is_empty());
    }
}
