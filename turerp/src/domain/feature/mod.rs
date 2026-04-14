//! Feature Flags Module
//!
//! This module provides feature flag functionality for gradual rollout,
//! A/B testing, and tenant-specific feature toggles.

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    CreateFeatureFlag, FeatureFlag, FeatureFlagResponse, FeatureFlagStatus, UpdateFeatureFlag,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresFeatureFlagRepository;
pub use repository::{FeatureFlagRepository, InMemoryFeatureFlagRepository};
pub use service::FeatureFlagService;
