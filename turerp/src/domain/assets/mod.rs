//! Fixed Assets Module
//!
//! This module provides fixed asset management with depreciation tracking,
//! maintenance records, and asset lifecycle management.

pub mod model;
pub mod repository;
pub mod service;

pub use model::{
    Asset, AssetCategory, AssetStatus, CreateAsset, CreateMaintenanceRecord, DepreciationMethod,
    MaintenanceRecord, UpdateAsset,
};
pub use repository::{AssetsRepository, BoxAssetsRepository, InMemoryAssetsRepository};
pub use service::AssetsService;
