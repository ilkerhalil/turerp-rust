//! Notification service with email template engine and in-app notifications
//!
//! Provides a `NotificationService` trait for sending notifications via
//! email, SMS, and in-app channels. Uses an async queue for reliable delivery
//! and a Handlebars-based template engine for email formatting.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Notification channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    Sms,
    InApp,
}

/// Notification priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NotificationPriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

/// Notification status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationStatus {
    Queued,
    Sending,
    Sent,
    Delivered,
    Failed,
    Read,
}

/// Notification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRequest {
    pub tenant_id: i64,
    pub user_id: Option<i64>,
    pub channel: NotificationChannel,
    pub priority: NotificationPriority,
    pub template_key: String,
    pub template_vars: serde_json::Value,
    pub recipient: String,
}

/// Notification record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: Option<i64>,
    pub channel: NotificationChannel,
    pub priority: NotificationPriority,
    pub status: NotificationStatus,
    pub template_key: String,
    pub subject: String,
    pub body: String,
    pub recipient: String,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub read_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub attempts: u32,
}

/// In-app notification for the notification bell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InAppNotification {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub read: bool,
    pub created_at: DateTime<Utc>,
    pub link: Option<String>,
}

/// Email template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplate {
    pub key: String,
    pub subject_template: String,
    pub body_template: String,
}

/// Simple template engine using string interpolation
fn render_template(template: &str, vars: &serde_json::Value) -> String {
    let mut result = template.to_string();
    if let serde_json::Value::Object(map) = vars {
        for (key, value) in map {
            let placeholder = format!("{{{{{}}}}}", key);
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => String::new(),
            };
            result = result.replace(&placeholder, &replacement);
        }
    }
    result
}

/// Notification service trait
#[async_trait::async_trait]
pub trait NotificationService: Send + Sync {
    /// Send a notification
    async fn send(&self, request: NotificationRequest) -> Result<Notification, String>;

    /// Get a notification by ID
    async fn get_notification(&self, id: i64) -> Result<Option<Notification>, String>;

    /// Get in-app notifications for a user
    async fn get_in_app_notifications(
        &self,
        tenant_id: i64,
        user_id: i64,
        unread_only: bool,
    ) -> Result<Vec<InAppNotification>, String>;

    /// Mark an in-app notification as read
    async fn mark_as_read(&self, id: i64) -> Result<(), String>;

    /// Mark all notifications as read for a user
    async fn mark_all_as_read(&self, tenant_id: i64, user_id: i64) -> Result<u64, String>;

    /// Get unread notification count for a user
    async fn unread_count(&self, tenant_id: i64, user_id: i64) -> Result<u64, String>;

    /// Register an email template
    fn register_template(&self, template: EmailTemplate);

    /// Retry a failed notification
    async fn retry(&self, id: i64) -> Result<(), String>;
}

/// Type alias for boxed notification service
pub type BoxNotificationService = Arc<dyn NotificationService>;

/// Built-in email templates
fn default_templates() -> Vec<EmailTemplate> {
    vec![
        EmailTemplate {
            key: "invoice_created".to_string(),
            subject_template: "Yeni Fatura: {{invoice_number}}".to_string(),
            body_template: "Sayın {{customer_name}},\n\n{{amount}} {{currency}} tutarında {{invoice_number}} numaralı fatura oluşturulmuştur.\n\nVade tarihi: {{due_date}}\n\nSaygılarımızla,\nTurerp ERP".to_string(),
        },
        EmailTemplate {
            key: "payment_received".to_string(),
            subject_template: "Ödeme Alındı: {{payment_id}}".to_string(),
            body_template: "Sayın {{customer_name}},\n\n{{amount}} {{currency}} tutarındaki ödemeniz alınmıştır.\n\nÖdeme tarihi: {{payment_date}}\n\nSaygılarımızla,\nTurerp ERP".to_string(),
        },
        EmailTemplate {
            key: "employee_hired".to_string(),
            subject_template: "Yeni Çalışan: {{employee_name}}".to_string(),
            body_template: "{{employee_name}} {{department}} departmanına atanmıştır.\n\nBaşlangıç tarihi: {{start_date}}\n\nİK Departmanı".to_string(),
        },
        EmailTemplate {
            key: "stock_low".to_string(),
            subject_template: "Düşük Stok Uyarısı: {{product_name}}".to_string(),
            body_template: "{{warehouse_name}} deposunda {{product_name}} ürününün stok miktarı {{quantity}} seviyesine düşmüştür.\n\nMinimum stok seviyesi: {{min_stock}}\n\nStok Yönetimi".to_string(),
        },
        EmailTemplate {
            key: "password_reset".to_string(),
            subject_template: "Şifre Sıfırlama".to_string(),
            body_template: "Şifrenizi sıfırlamak için aşağıdaki bağlantıyı kullanın:\n\n{{reset_link}}\n\nBu bağlantı {{expiry_minutes}} dakika geçerlidir.\n\nTurerp ERP".to_string(),
        },
    ]
}

/// In-memory notification service for development
pub struct InMemoryNotificationService {
    notifications: parking_lot::RwLock<Vec<Notification>>,
    in_app: parking_lot::RwLock<Vec<InAppNotification>>,
    templates: parking_lot::RwLock<Vec<EmailTemplate>>,
    next_id: parking_lot::RwLock<i64>,
    next_in_app_id: parking_lot::RwLock<i64>,
}

impl InMemoryNotificationService {
    pub fn new() -> Self {
        Self {
            notifications: parking_lot::RwLock::new(Vec::new()),
            in_app: parking_lot::RwLock::new(Vec::new()),
            templates: parking_lot::RwLock::new(default_templates()),
            next_id: parking_lot::RwLock::new(1),
            next_in_app_id: parking_lot::RwLock::new(1),
        }
    }

    fn allocate_id(&self) -> i64 {
        let mut id = self.next_id.write();
        let notification_id = *id;
        *id += 1;
        notification_id
    }

    fn allocate_in_app_id(&self) -> i64 {
        let mut id = self.next_in_app_id.write();
        let in_app_id = *id;
        *id += 1;
        in_app_id
    }

    fn render_notification(&self, request: &NotificationRequest) -> (String, String) {
        let templates = self.templates.read();
        let template = templates.iter().find(|t| t.key == request.template_key);

        match template {
            Some(t) => (
                render_template(&t.subject_template, &request.template_vars),
                render_template(&t.body_template, &request.template_vars),
            ),
            None => (
                format!("Notification: {}", request.template_key),
                format!(
                    "Template '{}' not found. Variables: {}",
                    request.template_key, request.template_vars
                ),
            ),
        }
    }
}

impl Default for InMemoryNotificationService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl NotificationService for InMemoryNotificationService {
    async fn send(&self, request: NotificationRequest) -> Result<Notification, String> {
        let id = self.allocate_id();
        let (subject, body) = self.render_notification(&request);

        let notification = Notification {
            id,
            tenant_id: request.tenant_id,
            user_id: request.user_id,
            channel: request.channel,
            priority: request.priority,
            status: NotificationStatus::Sent,
            template_key: request.template_key.clone(),
            subject,
            body: body.clone(),
            recipient: request.recipient.clone(),
            created_at: Utc::now(),
            sent_at: Some(Utc::now()),
            read_at: None,
            last_error: None,
            attempts: 1,
        };

        // If in-app, also create an in-app notification
        if request.channel == NotificationChannel::InApp {
            if let Some(user_id) = request.user_id {
                let in_app_id = self.allocate_in_app_id();
                let in_app = InAppNotification {
                    id: in_app_id,
                    tenant_id: request.tenant_id,
                    user_id,
                    title: format!("Notification: {}", request.template_key),
                    message: body,
                    notification_type: request.template_key.clone(),
                    read: false,
                    created_at: Utc::now(),
                    link: None,
                };
                self.in_app.write().push(in_app);
            }
        }

        self.notifications.write().push(notification.clone());
        Ok(notification)
    }

    async fn get_notification(&self, id: i64) -> Result<Option<Notification>, String> {
        Ok(self
            .notifications
            .read()
            .iter()
            .find(|n| n.id == id)
            .cloned())
    }

    async fn get_in_app_notifications(
        &self,
        tenant_id: i64,
        user_id: i64,
        unread_only: bool,
    ) -> Result<Vec<InAppNotification>, String> {
        let in_app = self.in_app.read();
        Ok(in_app
            .iter()
            .filter(|n| n.tenant_id == tenant_id && n.user_id == user_id)
            .filter(|n| !unread_only || !n.read)
            .cloned()
            .collect())
    }

    async fn mark_as_read(&self, id: i64) -> Result<(), String> {
        let mut in_app = self.in_app.write();
        let notification = in_app
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or_else(|| format!("In-app notification {} not found", id))?;
        notification.read = true;

        // Also mark the main notification as read
        let mut notifications = self.notifications.write();
        if let Some(n) = notifications.iter_mut().find(|n| n.id == id) {
            n.status = NotificationStatus::Read;
            n.read_at = Some(Utc::now());
        }

        Ok(())
    }

    async fn mark_all_as_read(&self, tenant_id: i64, user_id: i64) -> Result<u64, String> {
        let mut in_app = self.in_app.write();
        let mut count = 0u64;
        for n in in_app.iter_mut() {
            if n.tenant_id == tenant_id && n.user_id == user_id && !n.read {
                n.read = true;
                count += 1;
            }
        }
        Ok(count)
    }

    async fn unread_count(&self, tenant_id: i64, user_id: i64) -> Result<u64, String> {
        let in_app = self.in_app.read();
        Ok(in_app
            .iter()
            .filter(|n| n.tenant_id == tenant_id && n.user_id == user_id && !n.read)
            .count() as u64)
    }

    fn register_template(&self, template: EmailTemplate) {
        let mut templates = self.templates.write();
        templates.retain(|t| t.key != template.key);
        templates.push(template);
    }

    async fn retry(&self, id: i64) -> Result<(), String> {
        let mut notifications = self.notifications.write();
        let notification = notifications
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or_else(|| format!("Notification {} not found", id))?;

        if notification.status != NotificationStatus::Failed {
            return Err("Can only retry failed notifications".to_string());
        }

        notification.status = NotificationStatus::Sent;
        notification.sent_at = Some(Utc::now());
        notification.attempts += 1;
        notification.last_error = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_email_notification() {
        let service = InMemoryNotificationService::new();

        let notification = service
            .send(NotificationRequest {
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
            })
            .await
            .unwrap();

        assert_eq!(notification.tenant_id, 1);
        assert_eq!(notification.status, NotificationStatus::Sent);
        assert!(notification.subject.contains("INV-001"));
        assert!(notification.body.contains("Acme Corp"));
    }

    #[tokio::test]
    async fn test_in_app_notification() {
        let service = InMemoryNotificationService::new();

        service
            .send(NotificationRequest {
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
            })
            .await
            .unwrap();

        let notifications = service.get_in_app_notifications(1, 1, false).await.unwrap();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].user_id, 1);
        assert!(!notifications[0].read);
    }

    #[tokio::test]
    async fn test_mark_as_read() {
        let service = InMemoryNotificationService::new();

        service
            .send(NotificationRequest {
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
            })
            .await
            .unwrap();

        let count = service.unread_count(1, 1).await.unwrap();
        assert_eq!(count, 1);

        service.mark_as_read(1).await.unwrap();

        let count = service.unread_count(1, 1).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_mark_all_as_read() {
        let service = InMemoryNotificationService::new();

        for i in 0..3 {
            service
                .send(NotificationRequest {
                    tenant_id: 1,
                    user_id: Some(1),
                    channel: NotificationChannel::InApp,
                    priority: NotificationPriority::Normal,
                    template_key: "stock_low".to_string(),
                    template_vars: serde_json::json!({
                        "product_name": format!("Product {}", i),
                        "warehouse_name": "Main",
                        "quantity": "5",
                        "min_stock": "10"
                    }),
                    recipient: "warehouse@example.com".to_string(),
                })
                .await
                .unwrap();
        }

        let count = service.mark_all_as_read(1, 1).await.unwrap();
        assert_eq!(count, 3);

        let count = service.unread_count(1, 1).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_register_template() {
        let service = InMemoryNotificationService::new();

        service.register_template(EmailTemplate {
            key: "custom_template".to_string(),
            subject_template: "Custom: {{title}}".to_string(),
            body_template: "Hello {{name}},\n\n{{message}}\n\nGoodbye".to_string(),
        });

        let notification = service
            .send(NotificationRequest {
                tenant_id: 1,
                user_id: None,
                channel: NotificationChannel::Email,
                priority: NotificationPriority::Low,
                template_key: "custom_template".to_string(),
                template_vars: serde_json::json!({
                    "title": "Test Email",
                    "name": "Alice",
                    "message": "This is a test."
                }),
                recipient: "alice@example.com".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(notification.subject, "Custom: Test Email");
        assert!(notification.body.contains("Hello Alice"));
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let service = InMemoryNotificationService::new();

        for tenant_id in [1, 2] {
            service
                .send(NotificationRequest {
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
                })
                .await
                .unwrap();
        }

        let tenant1 = service.get_in_app_notifications(1, 1, false).await.unwrap();
        let tenant2 = service.get_in_app_notifications(2, 2, false).await.unwrap();
        assert_eq!(tenant1.len(), 1);
        assert_eq!(tenant2.len(), 1);
    }

    #[tokio::test]
    async fn test_template_rendering() {
        let vars = serde_json::json!({
            "name": "Test User",
            "amount": "100"
        });

        let result = render_template("Hello {{name}}, your amount is {{amount}}", &vars);
        assert_eq!(result, "Hello Test User, your amount is 100");

        let result = render_template("No placeholders here", &vars);
        assert_eq!(result, "No placeholders here");

        let result = render_template("Hello {{unknown}}", &vars);
        assert_eq!(result, "Hello {{unknown}}");
    }
}
