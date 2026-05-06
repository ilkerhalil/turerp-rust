//! Notification domain module

pub mod model;
pub mod postgres_repository;
pub mod provider;
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
pub use repository::{
    BoxInAppNotificationRepository, BoxNotificationPreferenceRepository, BoxNotificationRepository,
    InAppNotificationRepository, InMemoryInAppNotificationRepository,
    InMemoryNotificationPreferenceRepository, InMemoryNotificationRepository,
    NotificationPreferenceRepository, NotificationRepository,
};
pub use service::NotificationService;
pub use template::TemplateEngine;
