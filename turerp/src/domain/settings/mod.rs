//! Settings / Configuration Management domain module
//!
//! Provides tenant-scoped, typed configuration management with categories,
//! default values, and validation.

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    CreateSetting, Setting, SettingDataType, SettingGroup, SettingResponse, UpdateSetting,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresSettingsRepository;
pub use repository::{BoxSettingsRepository, InMemorySettingsRepository, SettingsRepository};
pub use service::SettingsService;
