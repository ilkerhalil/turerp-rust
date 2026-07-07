//! Assets service for business logic

use rust_decimal::Decimal;
use std::sync::Arc;

use super::model::{
    Asset, AssetCategory, AssetStatus, CreateAsset, CreateMaintenanceRecord, MaintenanceRecord,
    UpdateAsset,
};
use super::repository::{AssetCategoryRepository, AssetsRepository};
use crate::common::pagination::PaginatedResult;
use crate::domain::company::service::ensure_company_owned;
use crate::domain::company::BoxCompanyRepository;
use crate::error::ApiError;

/// Assets service
#[derive(Clone)]
pub struct AssetsService {
    asset_repo: Arc<dyn AssetsRepository>,
    category_repo: Option<Arc<dyn AssetCategoryRepository>>,
    company_repo: BoxCompanyRepository,
}

impl AssetsService {
    /// Create a new assets service
    pub fn new(asset_repo: Arc<dyn AssetsRepository>, company_repo: BoxCompanyRepository) -> Self {
        Self {
            asset_repo,
            category_repo: None,
            company_repo,
        }
    }

    /// Create a new asset
    #[tracing::instrument(skip(self))]
    pub async fn create_asset(&self, create: CreateAsset) -> Result<Asset, ApiError> {
        // Parent-ownership precheck: body company_id must belong to the caller's
        // tenant (legacy `1` sentinel skipped for backward compat).
        ensure_company_owned(&self.company_repo, create.company_id, create.tenant_id).await?;
        self.asset_repo.create(create).await
    }

    /// Get an asset by ID
    #[tracing::instrument(skip(self))]
    pub async fn get_asset(&self, id: i64, tenant_id: i64) -> Result<Asset, ApiError> {
        self.asset_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Asset {} not found", id)))
    }

    /// Get all assets for a tenant
    #[tracing::instrument(skip(self))]
    pub async fn get_assets_by_tenant(&self, tenant_id: i64) -> Result<Vec<Asset>, ApiError> {
        self.asset_repo.find_by_tenant(tenant_id).await
    }

    /// Get assets by tenant with pagination
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
    pub async fn get_assets_by_status(
        &self,
        tenant_id: i64,
        status: AssetStatus,
    ) -> Result<Vec<Asset>, ApiError> {
        self.asset_repo.find_by_status(tenant_id, status).await
    }

    /// Get assets by category
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
    pub async fn update_asset(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateAsset,
    ) -> Result<Asset, ApiError> {
        self.asset_repo.update(id, tenant_id, update).await
    }

    /// Update asset status
    #[tracing::instrument(skip(self))]
    pub async fn update_asset_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: AssetStatus,
    ) -> Result<Asset, ApiError> {
        self.asset_repo.update_status(id, tenant_id, status).await
    }

    /// Calculate and record depreciation for an asset
    #[tracing::instrument(skip(self))]
    pub async fn calculate_depreciation(&self, id: i64, tenant_id: i64) -> Result<Asset, ApiError> {
        let asset = self.get_asset(id, tenant_id).await?;
        let annual_depreciation = asset.calculate_annual_depreciation();
        self.asset_repo
            .record_depreciation(id, tenant_id, annual_depreciation)
            .await
    }

    /// Record manual depreciation for an asset
    #[tracing::instrument(skip(self))]
    pub async fn record_depreciation(
        &self,
        id: i64,
        tenant_id: i64,
        amount: Decimal,
    ) -> Result<Asset, ApiError> {
        if amount < Decimal::ZERO {
            return Err(ApiError::Validation(
                "Depreciation amount must be non-negative".to_string(),
            ));
        }
        self.asset_repo
            .record_depreciation(id, tenant_id, amount)
            .await
    }

    /// Dispose an asset
    #[tracing::instrument(skip(self))]
    pub async fn dispose_asset(&self, id: i64, tenant_id: i64) -> Result<Asset, ApiError> {
        self.update_asset_status(id, tenant_id, AssetStatus::Disposed)
            .await
    }

    /// Write off an asset
    #[tracing::instrument(skip(self))]
    pub async fn write_off_asset(&self, id: i64, tenant_id: i64) -> Result<Asset, ApiError> {
        self.update_asset_status(id, tenant_id, AssetStatus::WrittenOff)
            .await
    }

    /// Put asset under maintenance
    #[tracing::instrument(skip(self))]
    pub async fn start_maintenance(&self, id: i64, tenant_id: i64) -> Result<Asset, ApiError> {
        self.update_asset_status(id, tenant_id, AssetStatus::UnderMaintenance)
            .await
    }

    /// End maintenance and return to active/in-use
    #[tracing::instrument(skip(self))]
    pub async fn end_maintenance(
        &self,
        id: i64,
        tenant_id: i64,
        new_status: AssetStatus,
    ) -> Result<Asset, ApiError> {
        if !matches!(new_status, AssetStatus::Active | AssetStatus::InUse) {
            return Err(ApiError::Validation(
                "Status after maintenance must be Active or InUse".to_string(),
            ));
        }
        self.update_asset_status(id, tenant_id, new_status).await
    }

    /// Delete an asset
    #[tracing::instrument(skip(self))]
    pub async fn delete_asset(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.asset_repo.delete(id, tenant_id).await
    }

    /// Soft delete an asset
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_asset(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.asset_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Restore a soft-deleted asset
    #[tracing::instrument(skip(self))]
    pub async fn restore_asset(&self, id: i64, tenant_id: i64) -> Result<Asset, ApiError> {
        self.asset_repo.restore(id, tenant_id).await
    }

    /// List soft-deleted assets (admin only)
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_assets(&self, tenant_id: i64) -> Result<Vec<Asset>, ApiError> {
        self.asset_repo.find_deleted(tenant_id).await
    }

    /// Hard delete (destroy) an asset (admin only)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_asset(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.asset_repo.destroy(id, tenant_id).await
    }

    /// Create a maintenance record (tenant_id from the auth principal).
    /// Parent-ownership precheck: the body `asset_id` is an untrusted FK, so
    /// verify the asset belongs to the caller's tenant before writing —
    /// `get_asset` returns NotFound for a foreign asset, closing the
    /// cross-tenant create leak.
    #[tracing::instrument(skip(self, record), fields(tenant_id = tenant_id))]
    pub async fn create_maintenance_record(
        &self,
        record: CreateMaintenanceRecord,
        tenant_id: i64,
    ) -> Result<MaintenanceRecord, ApiError> {
        self.get_asset(record.asset_id, tenant_id).await?;
        self.asset_repo
            .create_maintenance_record(record, tenant_id)
            .await
    }

    /// Get maintenance records for an asset (tenant-scoped)
    #[tracing::instrument(skip(self), fields(tenant_id = tenant_id))]
    pub async fn get_maintenance_records(
        &self,
        asset_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<MaintenanceRecord>, ApiError> {
        self.asset_repo
            .get_maintenance_records(asset_id, tenant_id)
            .await
    }

    /// Create a category
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
    pub async fn get_categories(&self, tenant_id: i64) -> Result<Vec<AssetCategory>, ApiError> {
        let repo = self
            .category_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Category repository not configured".to_string()))?;
        repo.find_by_tenant(tenant_id).await
    }

    /// Delete a category
    #[tracing::instrument(skip(self))]
    pub async fn delete_category(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let repo = self
            .category_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Category repository not configured".to_string()))?;
        repo.delete(id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::assets::model::DepreciationMethod;
    use crate::domain::assets::repository::InMemoryAssetsRepository;
    use crate::domain::company::repository::InMemoryCompanyRepository;
    use crate::domain::company::service::LEGACY_COMPANY_ID;
    use crate::domain::company::CreateCompany;

    async fn create_service() -> AssetsService {
        let asset_repo = Arc::new(InMemoryAssetsRepository::new());
        let company_repo = Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository;
        // Seed tenant-1 (id=1 = LEGACY sentinel, skipped by the precheck) then
        // tenant-2 (id=2 non-sentinel) so reject tests hit the real find_by_id.
        for tenant in [1i64, 2] {
            company_repo
                .create(CreateCompany {
                    code: format!("CO{}", tenant),
                    name: format!("Tenant {} Co", tenant),
                    tax_number: None,
                    address: None,
                    city: None,
                    country: None,
                    currency: "TRY".to_string(),
                    tenant_id: tenant,
                })
                .await
                .unwrap();
        }
        AssetsService::new(asset_repo, company_repo)
    }

    /// Resolve tenant-2's company id (guaranteed non-sentinel by seeding order).
    async fn foreign_company_id(service: &AssetsService) -> i64 {
        let id = service
            .company_repo
            .find_by_tenant(2)
            .await
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .id;
        assert_ne!(id, LEGACY_COMPANY_ID);
        id
    }

    #[actix_web::test]
    async fn test_create_and_get_asset() {
        let service = create_service().await;

        let asset = service
            .create_asset(CreateAsset {
                tenant_id: 1,
                company_id: 1,
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

        let retrieved = service.get_asset(asset.id, 1).await.unwrap();
        assert_eq!(retrieved.name, "Test Computer");
    }

    #[actix_web::test]
    async fn test_depreciation_calculation() {
        let service = create_service().await;

        let asset = service
            .create_asset(CreateAsset {
                tenant_id: 1,
                company_id: 1,
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

        let updated = service.calculate_depreciation(asset.id, 1).await.unwrap();
        // Annual depreciation: (10000 - 1000) / 5 = 1800
        assert_eq!(updated.accumulated_depreciation, Decimal::from(1800));
        assert_eq!(updated.book_value, Decimal::from(8200));
    }

    #[actix_web::test]
    async fn test_asset_status_changes() {
        let service = create_service().await;

        let asset = service
            .create_asset(CreateAsset {
                tenant_id: 1,
                company_id: 1,
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

        let under_maintenance = service.start_maintenance(asset.id, 1).await.unwrap();
        assert_eq!(under_maintenance.status, AssetStatus::UnderMaintenance);

        let back_active = service
            .end_maintenance(asset.id, 1, AssetStatus::Active)
            .await
            .unwrap();
        assert_eq!(back_active.status, AssetStatus::Active);
    }

    #[actix_web::test]
    async fn test_maintenance_record() {
        let service = create_service().await;

        let asset = service
            .create_asset(CreateAsset {
                tenant_id: 1,
                company_id: 1,
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
            .create_maintenance_record(
                CreateMaintenanceRecord {
                    asset_id: asset.id,
                    maintenance_date: chrono::Utc::now(),
                    maintenance_type: "Preventive".to_string(),
                    description: "Annual service".to_string(),
                    cost: Decimal::from(500),
                    performed_by: Some("John".to_string()),
                    next_maintenance_date: Some(chrono::Utc::now() + chrono::Duration::days(365)),
                },
                1,
            )
            .await
            .unwrap();

        assert_eq!(record.asset_id, asset.id);

        let records = service.get_maintenance_records(asset.id, 1).await.unwrap();
        assert_eq!(records.len(), 1);
    }

    #[actix_web::test]
    async fn test_create_asset_rejects_foreign_company() {
        let service = create_service().await;
        let foreign = foreign_company_id(&service).await;

        let result = service
            .create_asset(CreateAsset {
                tenant_id: 1,
                company_id: foreign,
                name: "Foreign Co Asset".to_string(),
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
            .await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }
}
