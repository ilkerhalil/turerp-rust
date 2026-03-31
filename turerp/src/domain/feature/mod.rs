//! Feature Flags Module
//!
//! This module provides feature flag functionality for gradual rollout,
//! A/B testing, and tenant-specific feature toggles.

pub mod model;
pub mod repository;
pub mod service;

pub use model::{
    CreateFeatureFlag, FeatureFlag, FeatureFlagResponse, FeatureFlagStatus, UpdateFeatureFlag,
};
pub use repository::{FeatureFlagRepository, InMemoryFeatureFlagRepository};
pub use service::FeatureFlagService;
