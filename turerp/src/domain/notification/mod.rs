//! Notification domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod provider;
pub mod push_repository;
pub mod push_service;
pub mod push_token;
pub mod repository;
pub mod service;
pub mod template;

pub use model::{
    EmailTemplate, InAppNotification, InAppNotificationResponse, Notification, NotificationChannel,
    NotificationPreference, NotificationPreferenceResponse, NotificationPriority,
    NotificationRequest, NotificationResponse, NotificationStatus, UpdatePreference,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::{
    PostgresInAppNotificationRepository, PostgresNotificationPreferenceRepository,
    PostgresNotificationRepository,
};
pub use provider::{
    EmailProvider, NoopEmailProvider, NoopSmsProvider, SmsProvider, SmtpEmailProvider,
};
pub use push_repository::{
    BoxPushTokenRepository, InMemoryPushTokenRepository, PushTokenRepository,
};
pub use push_service::PushNotificationService;
pub use push_token::{DeviceType, PushMessage, PushToken, RegisterPushToken};
pub use repository::{
    BoxInAppNotificationRepository, BoxNotificationPreferenceRepository, BoxNotificationRepository,
    InAppNotificationRepository, InMemoryInAppNotificationRepository,
    InMemoryNotificationPreferenceRepository, InMemoryNotificationRepository,
    NotificationPreferenceRepository, NotificationRepository,
};
pub use service::NotificationService;
pub use template::TemplateEngine;
