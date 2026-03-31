//! Assets service for business logic

use rust_decimal::Decimal;
use std::sync::Arc;

use super::model::{
    Asset, AssetCategory, AssetStatus, CreateAsset, CreateMaintenanceRecord, MaintenanceRecord,
    UpdateAsset,
};
use super::repository::{AssetCategoryRepository, AssetsRepository};
use crate::common::pagination::PaginatedResult;
use crate::error::ApiError;

/// Assets service
#[derive(Clone)]
pub struct AssetsService {
    asset_repo: Arc<dyn AssetsRepository>,
    category_repo: Option<Arc<dyn AssetCategoryRepository>>,
}

impl AssetsService {
    /// Create a new assets service
    pub fn new(asset_repo: Arc<dyn AssetsRepository>) -> Self {
        Self {
            asset_repo,
            category_repo: None,
        }
    }

    /// Create assets service with category support
    pub fn with_categories(
        asset_repo: Arc<dyn AssetsRepository>,
        category_repo: Arc<dyn AssetCategoryRepository>,
    ) -> Self {
        Self {
            asset_repo,
            category_repo: Some(category_repo),
        }
    }

    /// Create a new asset
    pub async fn create_asset(&self, create: CreateAsset) -> Result<Asset, ApiError> {
        self.asset_repo.create(create).await
    }

    /// Get an asset by ID
    pub async fn get_asset(&self, id: i64) -> Result<Asset, ApiError> {
        self.asset_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Asset {} not found", id)))
    }

    /// Get all assets for a tenant
    pub async fn get_assets_by_tenant(&self, tenant_id: i64) -> Result<Vec<Asset>, ApiError> {
        self.asset_repo.find_by_tenant(tenant_id).await
    }

    /// Get assets by tenant with pagination
    pub async fn get_assets_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Asset>, ApiError> {
        self.asset_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    /// Get assets by status
    pub async fn get_assets_by_status(
        &self,
        tenant_id: i64,
        status: AssetStatus,
    ) -> Result<Vec<Asset>, ApiError> {
        self.asset_repo.find_by_status(tenant_id, status).await
    }

    /// Get assets by category
    pub async fn get_assets_by_category(
        &self,
        tenant_id: i64,
        category_id: i64,
    ) -> Result<Vec<Asset>, ApiError> {
        self.asset_repo
            .find_by_category(tenant_id, category_id)
            .await
    }

    /// Update an asset
    pub async fn update_asset(&self, id: i64, update: UpdateAsset) -> Result<Asset, ApiError> {
        self.asset_repo.update(id, update).await
    }

    /// Update asset status
    pub async fn update_asset_status(
        &self,
        id: i64,
        status: AssetStatus,
    ) -> Result<Asset, ApiError> {
        self.asset_repo.update_status(id, status).await
    }

    /// Calculate and record depreciation for an asset
    pub async fn calculate_depreciation(&self, id: i64) -> Result<Asset, ApiError> {
        let asset = self.get_asset(id).await?;
        let annual_depreciation = asset.calculate_annual_depreciation();
        self.asset_repo
            .record_depreciation(id, annual_depreciation)
            .await
    }

    /// Record manual depreciation for an asset
    pub async fn record_depreciation(&self, id: i64, amount: Decimal) -> Result<Asset, ApiError> {
        if amount < Decimal::ZERO {
            return Err(ApiError::Validation(
                "Depreciation amount must be non-negative".to_string(),
            ));
        }
        self.asset_repo.record_depreciation(id, amount).await
    }

    /// Dispose an asset
    pub async fn dispose_asset(&self, id: i64) -> Result<Asset, ApiError> {
        self.update_asset_status(id, AssetStatus::Disposed).await
    }

    /// Write off an asset
    pub async fn write_off_asset(&self, id: i64) -> Result<Asset, ApiError> {
        self.update_asset_status(id, AssetStatus::WrittenOff).await
    }

    /// Put asset under maintenance
    pub async fn start_maintenance(&self, id: i64) -> Result<Asset, ApiError> {
        self.update_asset_status(id, AssetStatus::UnderMaintenance)
            .await
    }

    /// End maintenance and return to active/in-use
    pub async fn end_maintenance(
        &self,
        id: i64,
        new_status: AssetStatus,
    ) -> Result<Asset, ApiError> {
        if !matches!(new_status, AssetStatus::Active | AssetStatus::InUse) {
            return Err(ApiError::Validation(
                "Status after maintenance must be Active or InUse".to_string(),
            ));
        }
        self.update_asset_status(id, new_status).await
    }

    /// Delete an asset
    pub async fn delete_asset(&self, id: i64) -> Result<(), ApiError> {
        self.asset_repo.delete(id).await
    }

    /// Create a maintenance record
    pub async fn create_maintenance_record(
        &self,
        record: CreateMaintenanceRecord,
    ) -> Result<MaintenanceRecord, ApiError> {
        self.asset_repo.create_maintenance_record(record).await
    }

    /// Get maintenance records for an asset
    pub async fn get_maintenance_records(
        &self,
        asset_id: i64,
    ) -> Result<Vec<MaintenanceRecord>, ApiError> {
        self.asset_repo.get_maintenance_records(asset_id).await
    }

    /// Create a category
    pub async fn create_category(
        &self,
        category: super::model::AssetCategory,
    ) -> Result<AssetCategory, ApiError> {
        let repo = self
            .category_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Category repository not configured".to_string()))?;
        repo.create(category).await
    }

    /// Get all categories for a tenant
    pub async fn get_categories(&self, tenant_id: i64) -> Result<Vec<AssetCategory>, ApiError> {
        let repo = self
            .category_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Category repository not configured".to_string()))?;
        repo.find_by_tenant(tenant_id).await
    }

    /// Delete a category
    pub async fn delete_category(&self, id: i64) -> Result<(), ApiError> {
        let repo = self
            .category_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Category repository not configured".to_string()))?;
        repo.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::assets::model::DepreciationMethod;
    use crate::domain::assets::repository::InMemoryAssetsRepository;

    #[actix_web::test]
    async fn test_create_and_get_asset() {
        let repo = Arc::new(InMemoryAssetsRepository::new());
        let service = AssetsService::new(repo);

        let asset = service
            .create_asset(CreateAsset {
                tenant_id: 1,
                name: "Test Computer".to_string(),
                category_id: None,
                description: None,
                serial_number: Some("SN123".to_string()),
                location: Some("Office".to_string()),
                acquisition_date: chrono::Utc::now(),
                acquisition_cost: Decimal::from(5000),
                salvage_value: Decimal::from(500),
                useful_life_years: 5,
                depreciation_method: Some(DepreciationMethod::StraightLine),
                warranty_expiry: None,
                insurance_number: None,
                insurance_expiry: None,
                responsible_person_id: None,
                notes: None,
            })
            .await
            .unwrap();

        let retrieved = service.get_asset(asset.id).await.unwrap();
        assert_eq!(retrieved.name, "Test Computer");
    }

    #[actix_web::test]
    async fn test_depreciation_calculation() {
        let repo = Arc::new(InMemoryAssetsRepository::new());
        let service = AssetsService::new(repo);

        let asset = service
            .create_asset(CreateAsset {
                tenant_id: 1,
                name: "Test Machine".to_string(),
                category_id: None,
                description: None,
                serial_number: None,
                location: None,
                acquisition_date: chrono::Utc::now(),
                acquisition_cost: Decimal::from(10000),
                salvage_value: Decimal::from(1000),
                useful_life_years: 5,
                depreciation_method: Some(DepreciationMethod::StraightLine),
                warranty_expiry: None,
                insurance_number: None,
                insurance_expiry: None,
                responsible_person_id: None,
                notes: None,
            })
            .await
            .unwrap();

        let updated = service.calculate_depreciation(asset.id).await.unwrap();
        // Annual depreciation: (10000 - 1000) / 5 = 1800
        assert_eq!(updated.accumulated_depreciation, Decimal::from(1800));
        assert_eq!(updated.book_value, Decimal::from(8200));
    }

    #[actix_web::test]
    async fn test_asset_status_changes() {
        let repo = Arc::new(InMemoryAssetsRepository::new());
        let service = AssetsService::new(repo);

        let asset = service
            .create_asset(CreateAsset {
                tenant_id: 1,
                name: "Test Asset".to_string(),
                category_id: None,
                description: None,
                serial_number: None,
                location: None,
                acquisition_date: chrono::Utc::now(),
                acquisition_cost: Decimal::from(1000),
                salvage_value: Decimal::from(100),
                useful_life_years: 5,
                depreciation_method: Some(DepreciationMethod::StraightLine),
                warranty_expiry: None,
                insurance_number: None,
                insurance_expiry: None,
                responsible_person_id: None,
                notes: None,
            })
            .await
            .unwrap();

        assert_eq!(asset.status, AssetStatus::Active);

        let under_maintenance = service.start_maintenance(asset.id).await.unwrap();
        assert_eq!(under_maintenance.status, AssetStatus::UnderMaintenance);

        let back_active = service
            .end_maintenance(asset.id, AssetStatus::Active)
            .await
            .unwrap();
        assert_eq!(back_active.status, AssetStatus::Active);
    }

    #[actix_web::test]
    async fn test_maintenance_record() {
        let repo = Arc::new(InMemoryAssetsRepository::new());
        let service = AssetsService::new(repo);

        let asset = service
            .create_asset(CreateAsset {
                tenant_id: 1,
                name: "Test Machine".to_string(),
                category_id: None,
                description: None,
                serial_number: None,
                location: None,
                acquisition_date: chrono::Utc::now(),
                acquisition_cost: Decimal::from(10000),
                salvage_value: Decimal::from(1000),
                useful_life_years: 10,
                depreciation_method: Some(DepreciationMethod::StraightLine),
                warranty_expiry: None,
                insurance_number: None,
                insurance_expiry: None,
                responsible_person_id: None,
                notes: None,
            })
            .await
            .unwrap();

        let record = service
            .create_maintenance_record(CreateMaintenanceRecord {
                asset_id: asset.id,
                maintenance_date: chrono::Utc::now(),
                maintenance_type: "Preventive".to_string(),
                description: "Annual service".to_string(),
                cost: Decimal::from(500),
                performed_by: Some("John".to_string()),
                next_maintenance_date: Some(chrono::Utc::now() + chrono::Duration::days(365)),
            })
            .await
            .unwrap();

        assert_eq!(record.asset_id, asset.id);

        let records = service.get_maintenance_records(asset.id).await.unwrap();
        assert_eq!(records.len(), 1);
    }
}
