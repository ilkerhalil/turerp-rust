//! Notification service with email template engine and in-app notifications
//!
//! Provides a `NotificationService` trait for sending notifications via
//! email, SMS, and in-app channels. Uses an async queue for reliable delivery
//! and a Handlebars-based template engine for email formatting.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::common::PaginatedResult;
use crate::error::ApiError;

/// Notification channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    Sms,
    InApp,
}

impl std::fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Email => write!(f, "email"),
            Self::Sms => write!(f, "sms"),
            Self::InApp => write!(f, "inapp"),
        }
    }
}

impl std::str::FromStr for NotificationChannel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "email" => Ok(Self::Email),
            "sms" => Ok(Self::Sms),
            "inapp" => Ok(Self::InApp),
            _ => Err(format!("Invalid notification channel: {}", s)),
        }
    }
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

impl std::fmt::Display for NotificationPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for NotificationPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "normal" => Ok(Self::Normal),
            "high" => Ok(Self::High),
            "urgent" => Ok(Self::Urgent),
            _ => Err(format!("Invalid notification priority: {}", s)),
        }
    }
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
    Cancelled,
}

impl std::fmt::Display for NotificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Sending => write!(f, "sending"),
            Self::Sent => write!(f, "sent"),
            Self::Delivered => write!(f, "delivered"),
            Self::Failed => write!(f, "failed"),
            Self::Read => write!(f, "read"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for NotificationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "queued" => Ok(Self::Queued),
            "sending" => Ok(Self::Sending),
            "sent" => Ok(Self::Sent),
            "delivered" => Ok(Self::Delivered),
            "failed" => Ok(Self::Failed),
            "read" => Ok(Self::Read),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("Invalid notification status: {}", s)),
        }
    }
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

/// Notification preference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreference {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub channel: NotificationChannel,
    pub notification_type: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for updating a preference
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdatePreference {
    pub channel: String,
    pub notification_type: String,
    pub enabled: bool,
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
    async fn send(&self, request: NotificationRequest) -> Result<Notification, ApiError>;

    /// Get a notification by ID
    async fn get_notification(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<Notification>, ApiError>;

    /// Get notification history for a user
    async fn get_history(
        &self,
        tenant_id: i64,
        user_id: Option<i64>,
        channel: Option<NotificationChannel>,
        limit: i64,
        offset: i64,
    ) -> Result<PaginatedResult<Notification>, ApiError>;

    /// Get in-app notifications for a user
    async fn get_in_app_notifications(
        &self,
        tenant_id: i64,
        user_id: i64,
        unread_only: bool,
    ) -> Result<Vec<InAppNotification>, ApiError>;

    /// Mark an in-app notification as read
    async fn mark_as_read(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Mark all notifications as read for a user
    async fn mark_all_as_read(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError>;

    /// Get unread notification count for a user
    async fn unread_count(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError>;

    /// Get user preferences
    async fn get_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
    ) -> Result<Vec<NotificationPreference>, ApiError>;

    /// Update user preferences
    async fn update_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
        prefs: Vec<UpdatePreference>,
    ) -> Result<Vec<NotificationPreference>, ApiError>;

    /// Register an email template
    fn register_template(&self, template: EmailTemplate);

    /// Retry a failed notification
    async fn retry(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
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
    preferences: parking_lot::RwLock<Vec<NotificationPreference>>,
    next_id: parking_lot::RwLock<i64>,
    next_in_app_id: parking_lot::RwLock<i64>,
    next_pref_id: parking_lot::RwLock<i64>,
}

impl InMemoryNotificationService {
    pub fn new() -> Self {
        Self {
            notifications: parking_lot::RwLock::new(Vec::new()),
            in_app: parking_lot::RwLock::new(Vec::new()),
            templates: parking_lot::RwLock::new(default_templates()),
            preferences: parking_lot::RwLock::new(Vec::new()),
            next_id: parking_lot::RwLock::new(1),
            next_in_app_id: parking_lot::RwLock::new(1),
            next_pref_id: parking_lot::RwLock::new(1),
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

    fn allocate_pref_id(&self) -> i64 {
        let mut id = self.next_pref_id.write();
        let pref_id = *id;
        *id += 1;
        pref_id
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
    async fn send(&self, request: NotificationRequest) -> Result<Notification, ApiError> {
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

    async fn get_notification(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<Notification>, ApiError> {
        let inner = self.notifications.read();
        Ok(inner
            .iter()
            .find(|n| n.id == id && n.tenant_id == tenant_id)
            .cloned())
    }

    async fn get_history(
        &self,
        tenant_id: i64,
        user_id: Option<i64>,
        channel: Option<NotificationChannel>,
        limit: i64,
        offset: i64,
    ) -> Result<PaginatedResult<Notification>, ApiError> {
        let inner = self.notifications.read();
        let mut filtered: Vec<_> = inner
            .iter()
            .filter(|n| n.tenant_id == tenant_id)
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

    async fn get_in_app_notifications(
        &self,
        tenant_id: i64,
        user_id: i64,
        unread_only: bool,
    ) -> Result<Vec<InAppNotification>, ApiError> {
        let in_app = self.in_app.read();
        Ok(in_app
            .iter()
            .filter(|n| n.tenant_id == tenant_id && n.user_id == user_id)
            .filter(|n| !unread_only || !n.read)
            .cloned()
            .collect())
    }

    async fn mark_as_read(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut in_app = self.in_app.write();
        let notification = in_app
            .iter_mut()
            .find(|n| n.id == id && n.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("In-app notification {} not found", id)))?;
        notification.read = true;

        let mut notifications = self.notifications.write();
        if let Some(n) = notifications.iter_mut().find(|n| n.id == id) {
            n.status = NotificationStatus::Read;
            n.read_at = Some(Utc::now());
        }

        Ok(())
    }

    async fn mark_all_as_read(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError> {
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

    async fn unread_count(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError> {
        let in_app = self.in_app.read();
        Ok(in_app
            .iter()
            .filter(|n| n.tenant_id == tenant_id && n.user_id == user_id && !n.read)
            .count() as u64)
    }

    async fn get_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
    ) -> Result<Vec<NotificationPreference>, ApiError> {
        let prefs = self.preferences.read();
        Ok(prefs
            .iter()
            .filter(|p| p.tenant_id == tenant_id && p.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn update_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
        prefs: Vec<UpdatePreference>,
    ) -> Result<Vec<NotificationPreference>, ApiError> {
        let mut preferences = self.preferences.write();
        let mut results = Vec::new();

        for pref in prefs {
            let channel = pref
                .channel
                .parse()
                .map_err(|e: String| ApiError::Validation(format!("Invalid channel: {}", e)))?;

            if let Some(existing) = preferences.iter_mut().find(|p| {
                p.tenant_id == tenant_id
                    && p.user_id == user_id
                    && p.channel == channel
                    && p.notification_type == pref.notification_type
            }) {
                existing.enabled = pref.enabled;
                existing.updated_at = Utc::now();
                results.push(existing.clone());
            } else {
                let new_pref = NotificationPreference {
                    id: self.allocate_pref_id(),
                    tenant_id,
                    user_id,
                    channel,
                    notification_type: pref.notification_type,
                    enabled: pref.enabled,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                preferences.push(new_pref.clone());
                results.push(new_pref);
            }
        }
        Ok(results)
    }

    fn register_template(&self, template: EmailTemplate) {
        let mut templates = self.templates.write();
        templates.retain(|t| t.key != template.key);
        templates.push(template);
    }

    async fn retry(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut notifications = self.notifications.write();
        let notification = notifications
            .iter_mut()
            .find(|n| n.id == id && n.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Notification {} not found", id)))?;

        if notification.status != NotificationStatus::Failed {
            return Err(ApiError::BadRequest(
                "Can only retry failed notifications".to_string(),
            ));
        }

        notification.status = NotificationStatus::Sent;
        notification.sent_at = Some(Utc::now());
        notification.attempts += 1;
        notification.last_error = None;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Conversions between common and domain notification types
// ---------------------------------------------------------------------------

impl From<NotificationChannel> for crate::domain::notification::model::NotificationChannel {
    fn from(c: NotificationChannel) -> Self {
        match c {
            NotificationChannel::Email => Self::Email,
            NotificationChannel::Sms => Self::Sms,
            NotificationChannel::InApp => Self::InApp,
        }
    }
}

impl From<NotificationPriority> for crate::domain::notification::model::NotificationPriority {
    fn from(p: NotificationPriority) -> Self {
        match p {
            NotificationPriority::Low => Self::Low,
            NotificationPriority::Normal => Self::Normal,
            NotificationPriority::High => Self::High,
            NotificationPriority::Urgent => Self::Urgent,
        }
    }
}

impl From<crate::domain::notification::model::NotificationChannel> for NotificationChannel {
    fn from(c: crate::domain::notification::model::NotificationChannel) -> Self {
        match c {
            crate::domain::notification::model::NotificationChannel::Email => Self::Email,
            crate::domain::notification::model::NotificationChannel::Sms => Self::Sms,
            crate::domain::notification::model::NotificationChannel::InApp => Self::InApp,
        }
    }
}

impl From<crate::domain::notification::model::NotificationPriority> for NotificationPriority {
    fn from(p: crate::domain::notification::model::NotificationPriority) -> Self {
        match p {
            crate::domain::notification::model::NotificationPriority::Low => Self::Low,
            crate::domain::notification::model::NotificationPriority::Normal => Self::Normal,
            crate::domain::notification::model::NotificationPriority::High => Self::High,
            crate::domain::notification::model::NotificationPriority::Urgent => Self::Urgent,
        }
    }
}

// ---------------------------------------------------------------------------
// Adapter: bridge domain notification service to common trait
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl NotificationService for crate::domain::notification::service::NotificationService {
    async fn send(&self, request: NotificationRequest) -> Result<Notification, ApiError> {
        use crate::domain::notification::model::NotificationRequest as DomainRequest;

        let domain_request = DomainRequest {
            tenant_id: request.tenant_id,
            user_id: request.user_id,
            channel: request.channel.into(),
            priority: request.priority.into(),
            template_key: request.template_key,
            template_vars: request.template_vars,
            recipient: request.recipient,
        };

        let response =
            crate::domain::notification::service::NotificationService::send(self, domain_request)
                .await?;

        Ok(Notification {
            id: response.id,
            tenant_id: response.tenant_id,
            user_id: response.user_id,
            channel: request.channel,
            priority: request.priority,
            status: response
                .status
                .parse()
                .unwrap_or(NotificationStatus::Queued),
            template_key: response.template_key.unwrap_or_default(),
            subject: response.subject,
            body: response.body,
            recipient: response.recipient,
            created_at: response.created_at,
            sent_at: response.sent_at,
            read_at: None,
            last_error: None,
            attempts: response.attempts,
        })
    }

    async fn get_notification(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<Notification>, ApiError> {
        let response = crate::domain::notification::service::NotificationService::get_notification(
            self, id, tenant_id,
        )
        .await?;
        Ok(response.map(|r| Notification {
            id: r.id,
            tenant_id: r.tenant_id,
            user_id: r.user_id,
            channel: r.channel.parse().unwrap_or(NotificationChannel::Email),
            priority: r.priority.parse().unwrap_or_default(),
            status: r.status.parse().unwrap_or(NotificationStatus::Queued),
            template_key: r.template_key.unwrap_or_default(),
            subject: r.subject,
            body: r.body,
            recipient: r.recipient,
            created_at: r.created_at,
            sent_at: r.sent_at,
            read_at: None,
            last_error: None,
            attempts: r.attempts,
        }))
    }

    async fn get_history(
        &self,
        tenant_id: i64,
        user_id: Option<i64>,
        channel: Option<NotificationChannel>,
        limit: i64,
        offset: i64,
    ) -> Result<PaginatedResult<Notification>, ApiError> {
        let result = crate::domain::notification::service::NotificationService::get_history(
            self,
            tenant_id,
            user_id,
            channel.map(|c| c.into()),
            limit,
            offset,
        )
        .await?;

        Ok(result.map(|r| Notification {
            id: r.id,
            tenant_id: r.tenant_id,
            user_id: r.user_id,
            channel: r.channel.parse().unwrap_or(NotificationChannel::Email),
            priority: r.priority.parse().unwrap_or_default(),
            status: r.status.parse().unwrap_or(NotificationStatus::Queued),
            template_key: r.template_key.unwrap_or_default(),
            subject: r.subject,
            body: r.body,
            recipient: r.recipient,
            created_at: r.created_at,
            sent_at: r.sent_at,
            read_at: None,
            last_error: None,
            attempts: r.attempts,
        }))
    }

    async fn get_in_app_notifications(
        &self,
        tenant_id: i64,
        user_id: i64,
        unread_only: bool,
    ) -> Result<Vec<InAppNotification>, ApiError> {
        let responses =
            crate::domain::notification::service::NotificationService::get_in_app_notifications(
                self,
                tenant_id,
                user_id,
                unread_only,
            )
            .await?;

        Ok(responses
            .into_iter()
            .map(|r| InAppNotification {
                id: r.id,
                tenant_id: r.tenant_id,
                user_id: r.user_id,
                title: r.title,
                message: r.message,
                notification_type: r.notification_type,
                read: r.read,
                created_at: r.created_at.parse().unwrap_or(Utc::now()),
                link: r.link,
            })
            .collect())
    }

    async fn mark_as_read(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        crate::domain::notification::service::NotificationService::mark_as_read(self, id, tenant_id)
            .await
    }

    async fn mark_all_as_read(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError> {
        crate::domain::notification::service::NotificationService::mark_all_as_read(
            self, tenant_id, user_id,
        )
        .await
    }

    async fn unread_count(&self, tenant_id: i64, user_id: i64) -> Result<u64, ApiError> {
        crate::domain::notification::service::NotificationService::unread_count(
            self, tenant_id, user_id,
        )
        .await
    }

    async fn get_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
    ) -> Result<Vec<NotificationPreference>, ApiError> {
        let responses = crate::domain::notification::service::NotificationService::get_preferences(
            self, tenant_id, user_id,
        )
        .await?;

        Ok(responses
            .into_iter()
            .map(|r| NotificationPreference {
                id: r.id,
                tenant_id,
                user_id,
                channel: r.channel.parse().unwrap_or(NotificationChannel::Email),
                notification_type: r.notification_type,
                enabled: r.enabled,
                created_at: r.updated_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    async fn update_preferences(
        &self,
        tenant_id: i64,
        user_id: i64,
        prefs: Vec<UpdatePreference>,
    ) -> Result<Vec<NotificationPreference>, ApiError> {
        let domain_prefs: Vec<crate::domain::notification::model::UpdatePreference> = prefs
            .into_iter()
            .map(|p| crate::domain::notification::model::UpdatePreference {
                channel: p.channel,
                notification_type: p.notification_type,
                enabled: p.enabled,
            })
            .collect();

        let responses =
            crate::domain::notification::service::NotificationService::update_preferences(
                self,
                tenant_id,
                user_id,
                domain_prefs,
            )
            .await?;

        Ok(responses
            .into_iter()
            .map(|r| NotificationPreference {
                id: r.id,
                tenant_id,
                user_id,
                channel: r.channel.parse().unwrap_or(NotificationChannel::Email),
                notification_type: r.notification_type,
                enabled: r.enabled,
                created_at: r.updated_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    fn register_template(&self, template: EmailTemplate) {
        let _ = template;
    }

    async fn retry(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        crate::domain::notification::service::NotificationService::retry(self, id, tenant_id).await
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

        service.mark_as_read(1, 1).await.unwrap();

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

    #[tokio::test]
    async fn test_send_sms_notification() {
        let service = InMemoryNotificationService::new();

        let notification = service
            .send(NotificationRequest {
                tenant_id: 1,
                user_id: Some(1),
                channel: NotificationChannel::Sms,
                priority: NotificationPriority::High,
                template_key: "stock_low".to_string(),
                template_vars: serde_json::json!({
                    "product_name": "Widget A",
                    "warehouse_name": "Main",
                    "quantity": "3",
                    "min_stock": "10"
                }),
                recipient: "+905551234567".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(notification.tenant_id, 1);
        assert_eq!(notification.channel, NotificationChannel::Sms);
        assert_eq!(notification.status, NotificationStatus::Sent);
        assert_eq!(notification.recipient, "+905551234567");
        assert!(notification.body.contains("Widget A"));
    }

    #[tokio::test]
    async fn test_get_notification_by_id() {
        let service = InMemoryNotificationService::new();

        let sent = service
            .send(NotificationRequest {
                tenant_id: 1,
                user_id: Some(1),
                channel: NotificationChannel::Email,
                priority: NotificationPriority::Normal,
                template_key: "invoice_created".to_string(),
                template_vars: serde_json::json!({
                    "customer_name": "Acme Corp",
                    "invoice_number": "INV-002",
                    "amount": "2000.00",
                    "currency": "TRY",
                    "due_date": "2024-03-01"
                }),
                recipient: "test@example.com".to_string(),
            })
            .await
            .unwrap();

        let found = service.get_notification(sent.id, 1).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id, sent.id);
        assert_eq!(found.template_key, "invoice_created");

        let not_found = service.get_notification(9999, 1).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_retry_failed_notification() {
        let service = InMemoryNotificationService::new();

        let notification = service
            .send(NotificationRequest {
                tenant_id: 1,
                user_id: Some(1),
                channel: NotificationChannel::Email,
                priority: NotificationPriority::Normal,
                template_key: "payment_received".to_string(),
                template_vars: serde_json::json!({
                    "customer_name": "Beta Inc",
                    "payment_id": "PAY-001",
                    "amount": "5000.00",
                    "currency": "TRY",
                    "payment_date": "2024-01-15"
                }),
                recipient: "finance@example.com".to_string(),
            })
            .await
            .unwrap();

        let result = service.retry(notification.id, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unread_only_filter() {
        let service = InMemoryNotificationService::new();

        for i in 0..2 {
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

        let all = service.get_in_app_notifications(1, 1, false).await.unwrap();
        assert_eq!(all.len(), 2);

        let unread = service.get_in_app_notifications(1, 1, true).await.unwrap();
        assert_eq!(unread.len(), 2);

        service.mark_as_read(1, 1).await.unwrap();

        let unread_after = service.get_in_app_notifications(1, 1, true).await.unwrap();
        assert_eq!(unread_after.len(), 1);

        let all_after = service.get_in_app_notifications(1, 1, false).await.unwrap();
        assert_eq!(all_after.len(), 2);
    }

    #[tokio::test]
    async fn test_missing_template_fallback() {
        let service = InMemoryNotificationService::new();

        let notification = service
            .send(NotificationRequest {
                tenant_id: 1,
                user_id: Some(1),
                channel: NotificationChannel::Email,
                priority: NotificationPriority::Normal,
                template_key: "nonexistent_template".to_string(),
                template_vars: serde_json::json!({"foo": "bar"}),
                recipient: "test@example.com".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(notification.status, NotificationStatus::Sent);
        assert!(notification.subject.contains("nonexistent_template"));
        assert!(notification.body.contains("not found"));
    }

    #[tokio::test]
    async fn test_get_history() {
        let service = InMemoryNotificationService::new();

        for i in 0..5 {
            service
                .send(NotificationRequest {
                    tenant_id: 1,
                    user_id: Some(1),
                    channel: NotificationChannel::Email,
                    priority: NotificationPriority::Normal,
                    template_key: "invoice_created".to_string(),
                    template_vars: serde_json::json!({
                        "customer_name": format!("Customer {}", i),
                        "invoice_number": format!("INV-{}", i),
                        "amount": "1000.00",
                        "currency": "TRY",
                        "due_date": "2024-02-01"
                    }),
                    recipient: format!("customer{}@example.com", i),
                })
                .await
                .unwrap();
        }

        let history = service.get_history(1, Some(1), None, 10, 0).await.unwrap();
        assert_eq!(history.items.len(), 5);
        assert_eq!(history.total, 5);
    }

    #[tokio::test]
    async fn test_preferences() {
        let service = InMemoryNotificationService::new();

        let prefs = vec![UpdatePreference {
            channel: "email".to_string(),
            notification_type: "invoice_created".to_string(),
            enabled: false,
        }];

        let updated = service.update_preferences(1, 1, prefs).await.unwrap();
        assert_eq!(updated.len(), 1);
        assert!(!updated[0].enabled);

        let fetched = service.get_preferences(1, 1).await.unwrap();
        assert_eq!(fetched.len(), 1);
        assert!(!fetched[0].enabled);
    }
}
