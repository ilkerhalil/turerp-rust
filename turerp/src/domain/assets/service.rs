//! Assets service for business logic

use rust_decimal::Decimal;
use std::sync::Arc;

use super::model::{
    Asset, AssetCategory, AssetStatus, CreateAsset, CreateMaintenanceRecord, MaintenanceRecord,
    UpdateAsset,
};
use super::repository::{AssetsRepository, BoxAssetCategoryRepository};
use crate::common::pagination::PaginatedResult;
use crate::domain::company::service::ensure_company_owned;
use crate::domain::company::BoxCompanyRepository;
use crate::domain::user::repository::BoxUserRepository;
use crate::domain::user::service::ensure_user_owned;
use crate::error::ApiError;

/// Assets service
#[derive(Clone)]
pub struct AssetsService {
    asset_repo: Arc<dyn AssetsRepository>,
    category_repo: BoxAssetCategoryRepository,
    company_repo: BoxCompanyRepository,
    user_repo: BoxUserRepository,
}

impl AssetsService {
    /// Create a new assets service
    pub fn new(
        asset_repo: Arc<dyn AssetsRepository>,
        category_repo: BoxAssetCategoryRepository,
        company_repo: BoxCompanyRepository,
        user_repo: BoxUserRepository,
    ) -> Self {
        Self {
            asset_repo,
            category_repo,
            company_repo,
            user_repo,
        }
    }

    /// Create a new asset
    #[tracing::instrument(skip(self))]
    pub async fn create_asset(&self, create: CreateAsset) -> Result<Asset, ApiError> {
        // Parent-ownership precheck: body company_id must belong to the caller's
        // tenant (legacy `1` sentinel skipped for backward compat).
        ensure_company_owned(&self.company_repo, create.company_id, create.tenant_id).await?;
        // Parent-ownership precheck: body responsible_person_id (when set) must
        // belong to the caller's tenant (REFERENCES users(id)). `None` is a
        // legitimate "unassigned" value and is NOT rejected.
        if let Some(id) = create.responsible_person_id {
            ensure_user_owned(&self.user_repo, id, create.tenant_id).await?;
        }
        // Parent-ownership precheck: body category_id (when set) must belong to
        // the caller's tenant (REFERENCES asset_categories(id)). `None` is a
        // legitimate "uncategorized" value and is NOT rejected (orphan-FK
        // IDOR, issue #302).
        if let Some(id) = create.category_id {
            self.category_repo
                .find_by_id(id, create.tenant_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("Asset category {} not found", id)))?;
        }
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
        // Parent-ownership precheck (update path): the repo UPDATE persists
        // responsible_person_id via COALESCE, so a foreign / fabricated user id
        // could be re-stamped onto an asset. `None` leaves the stored value
        // untouched and is NOT rejected.
        if let Some(id) = update.responsible_person_id {
            ensure_user_owned(&self.user_repo, id, tenant_id).await?;
        }
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
        self.category_repo.create(category).await
    }

    /// Get all categories for a tenant
    #[tracing::instrument(skip(self))]
    pub async fn get_categories(&self, tenant_id: i64) -> Result<Vec<AssetCategory>, ApiError> {
        self.category_repo.find_by_tenant(tenant_id).await
    }

    /// Delete a category
    #[tracing::instrument(skip(self))]
    pub async fn delete_category(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.category_repo.delete(id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::assets::model::{AssetCategory, DepreciationMethod};
    use crate::domain::assets::repository::{
        BoxAssetCategoryRepository, InMemoryAssetCategoryRepository, InMemoryAssetsRepository,
    };
    use crate::domain::company::repository::InMemoryCompanyRepository;
    use crate::domain::company::service::LEGACY_COMPANY_ID;
    use crate::domain::company::CreateCompany;
    use crate::domain::user::model::{CreateUser, Role};
    use crate::domain::user::repository::{BoxUserRepository, InMemoryUserRepository};

    async fn create_service() -> AssetsService {
        let asset_repo = Arc::new(InMemoryAssetsRepository::new());
        let category_repo =
            Arc::new(InMemoryAssetCategoryRepository::new()) as BoxAssetCategoryRepository;
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
        let user_repo = Arc::new(InMemoryUserRepository::new()) as BoxUserRepository;
        // Seed a user per tenant: auto-id 1 for tenant-1 and auto-id 2 for
        // tenant-2 (the foreign referent the `responsible_person_id` reject
        // tests target).
        for tenant in [1, 2] {
            user_repo
                .create(
                    CreateUser {
                        username: format!("t{}asset", tenant),
                        email: format!("t{}asset@example.com", tenant),
                        full_name: format!("Tenant {} asset user", tenant),
                        password: "password123456".to_string(),
                        tenant_id: tenant,
                        role: Some(Role::User),
                    },
                    "hash".to_string(),
                )
                .await
                .expect("seed user");
        }
        AssetsService::new(asset_repo, category_repo, company_repo, user_repo)
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

    /// Returns the tenant-2 user id (a foreign user) for the
    /// `responsible_person_id` reject tests.
    async fn foreign_user_id(service: &AssetsService) -> i64 {
        service
            .user_repo
            .find_all(2)
            .await
            .expect("list tenant-2 users")
            .into_iter()
            .map(|u| u.id)
            .next()
            .expect("tenant-2 user seeded")
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

    /// Build a minimal `CreateAsset` for the user-precheck reject tests, varying
    /// only `company_id` (LEGACY `1` sentinel → skipped) and `responsible_person_id`.
    fn make_create(tenant_id: i64, company_id: i64, person: Option<i64>) -> CreateAsset {
        CreateAsset {
            tenant_id,
            company_id,
            name: "Reject Asset".to_string(),
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
            responsible_person_id: person,
            notes: None,
        }
    }

    #[actix_web::test]
    async fn test_create_asset_rejects_foreign_responsible_person() {
        let service = create_service().await;
        let foreign = foreign_user_id(&service).await;
        // company_id=1 is the LEGACY sentinel (skipped), isolating the user gate.
        let result = service.create_asset(make_create(1, 1, Some(foreign))).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    /// Rejects an asset stamped onto a foreign-tenant category (orphan-FK
    /// IDOR, issue #302). Own-tenant category succeeds; a foreign-tenant
    /// category id and a nonexistent id both 404; `None` (uncategorized) is
    /// accepted. `company_id=1` (LEGACY sentinel) + `responsible_person_id`
    /// None isolate the category gate.
    #[actix_web::test]
    async fn test_create_asset_rejects_foreign_category() {
        let service = create_service().await;
        let own_cat = service
            .create_category(AssetCategory {
                id: 0,
                tenant_id: 1,
                name: "Own Cat".to_string(),
                description: None,
                default_useful_life_years: 5,
                default_depreciation_method: DepreciationMethod::StraightLine,
                created_at: chrono::Utc::now(),
            })
            .await
            .unwrap();
        let foreign_cat = service
            .create_category(AssetCategory {
                id: 0,
                tenant_id: 2,
                name: "Foreign Cat".to_string(),
                description: None,
                default_useful_life_years: 5,
                default_depreciation_method: DepreciationMethod::StraightLine,
                created_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        // Own-tenant category → ok.
        let mut req = make_create(1, 1, None);
        req.category_id = Some(own_cat.id);
        assert!(
            service.create_asset(req).await.is_ok(),
            "own-tenant category must succeed"
        );

        // Foreign-tenant category → NotFound.
        let mut req = make_create(1, 1, None);
        req.category_id = Some(foreign_cat.id);
        let result = service.create_asset(req).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "tenant-1 must NOT create an asset for a tenant-2 category, got {:?}",
            result
        );

        // Nonexistent category → NotFound.
        let mut req = make_create(1, 1, None);
        req.category_id = Some(999);
        let result = service.create_asset(req).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "nonexistent category must be NotFound, got {:?}",
            result
        );

        // None (uncategorized) → ok (legitimate, never rejected).
        let req = make_create(1, 1, None);
        assert!(
            service.create_asset(req).await.is_ok(),
            "uncategorized asset (category_id None) must succeed"
        );
    }

    #[actix_web::test]
    async fn test_update_asset_rejects_foreign_responsible_person() {
        let service = create_service().await;
        // Create a tenant-1 asset with no responsible person (precheck skipped).
        let asset = service.create_asset(make_create(1, 1, None)).await.unwrap();
        let foreign = foreign_user_id(&service).await;
        let result = service
            .update_asset(
                asset.id,
                1,
                UpdateAsset {
                    name: None,
                    description: None,
                    serial_number: None,
                    location: None,
                    status: None,
                    location_id: None,
                    responsible_person_id: Some(foreign),
                    notes: None,
                    company_id: None,
                },
            )
            .await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }
}
