//! Fixed Assets Module
//!
//! This module provides fixed asset management with depreciation tracking,
//! maintenance records, and asset lifecycle management.

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    Asset, AssetCategory, AssetStatus, CreateAsset, CreateMaintenanceRecord, DepreciationMethod,
    MaintenanceRecord, UpdateAsset,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::{PostgresAssetCategoryRepository, PostgresAssetsRepository};
pub use repository::{
    AssetCategoryRepository, AssetsRepository, BoxAssetCategoryRepository, BoxAssetsRepository,
    InMemoryAssetsRepository,
};
pub use service::AssetsService;
