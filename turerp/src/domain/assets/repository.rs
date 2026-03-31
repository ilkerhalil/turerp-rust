//! Assets repository trait and implementations

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use super::model::{
    Asset, AssetCategory, AssetStatus, CreateAsset, CreateMaintenanceRecord, MaintenanceRecord,
    UpdateAsset,
};
use crate::common::pagination::PaginatedResult;
use crate::error::ApiError;

/// Repository trait for Assets operations
#[async_trait]
pub trait AssetsRepository: Send + Sync {
    /// Create a new asset
    async fn create(&self, asset: CreateAsset) -> Result<Asset, ApiError>;

    /// Find asset by ID
    async fn find_by_id(&self, id: i64) -> Result<Option<Asset>, ApiError>;

    /// Find all assets for a tenant
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Asset>, ApiError>;

    /// Find assets by tenant with pagination
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Asset>, ApiError>;

    /// Find assets by status
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: AssetStatus,
    ) -> Result<Vec<Asset>, ApiError>;

    /// Find assets by category
    async fn find_by_category(
        &self,
        tenant_id: i64,
        category_id: i64,
    ) -> Result<Vec<Asset>, ApiError>;

    /// Update an asset
    async fn update(&self, id: i64, update: UpdateAsset) -> Result<Asset, ApiError>;

    /// Update asset status
    async fn update_status(&self, id: i64, status: AssetStatus) -> Result<Asset, ApiError>;

    /// Record depreciation for an asset
    async fn record_depreciation(&self, id: i64, amount: f64) -> Result<Asset, ApiError>;

    /// Delete an asset
    async fn delete(&self, id: i64) -> Result<(), ApiError>;

    /// Create maintenance record
    async fn create_maintenance_record(
        &self,
        record: CreateMaintenanceRecord,
    ) -> Result<MaintenanceRecord, ApiError>;

    /// Get maintenance records for an asset
    async fn get_maintenance_records(
        &self,
        asset_id: i64,
    ) -> Result<Vec<MaintenanceRecord>, ApiError>;
}

/// Repository trait for Asset Category operations
#[async_trait]
pub trait AssetCategoryRepository: Send + Sync {
    /// Create a new category
    async fn create(
        &self,
        category: super::model::AssetCategory,
    ) -> Result<AssetCategory, ApiError>;

    /// Find category by ID
    async fn find_by_id(&self, id: i64) -> Result<Option<AssetCategory>, ApiError>;

    /// Find all categories for a tenant
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<AssetCategory>, ApiError>;

    /// Delete a category
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Type aliases
pub type BoxAssetsRepository = Arc<dyn AssetsRepository>;
pub type BoxAssetCategoryRepository = Arc<dyn AssetCategoryRepository>;

fn generate_asset_code(count: i64) -> String {
    format!("AST-{:06}", count)
}

/// In-memory assets repository
pub struct InMemoryAssetsRepository {
    assets: Mutex<std::collections::HashMap<i64, Asset>>,
    maintenance_records: Mutex<std::collections::HashMap<i64, Vec<MaintenanceRecord>>>,
    next_id: Mutex<i64>,
    next_maintenance_id: Mutex<i64>,
}

impl InMemoryAssetsRepository {
    pub fn new() -> Self {
        Self {
            assets: Mutex::new(std::collections::HashMap::new()),
            maintenance_records: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
            next_maintenance_id: Mutex::new(1),
        }
    }
}

impl Default for InMemoryAssetsRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AssetsRepository for InMemoryAssetsRepository {
    async fn create(&self, create: CreateAsset) -> Result<Asset, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;

        let asset_code = generate_asset_code(id);
        let now = chrono::Utc::now();
        let depreciation_method = create.depreciation_method.unwrap_or_default();

        let asset = Asset {
            id,
            tenant_id: create.tenant_id,
            asset_code,
            name: create.name,
            category_id: create.category_id,
            description: create.description,
            serial_number: create.serial_number,
            location: create.location,
            status: AssetStatus::Active,
            acquisition_date: create.acquisition_date,
            acquisition_cost: create.acquisition_cost,
            salvage_value: create.salvage_value,
            useful_life_years: create.useful_life_years,
            depreciation_method,
            accumulated_depreciation: 0.0,
            book_value: create.acquisition_cost,
            warranty_expiry: create.warranty_expiry,
            insurance_number: create.insurance_number,
            insurance_expiry: create.insurance_expiry,
            responsible_person_id: create.responsible_person_id,
            notes: create.notes,
            created_at: now,
            updated_at: now,
        };

        self.assets.lock().insert(id, asset.clone());
        Ok(asset)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Asset>, ApiError> {
        Ok(self.assets.lock().get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Asset>, ApiError> {
        let assets = self.assets.lock();
        Ok(assets
            .values()
            .filter(|a| a.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Asset>, ApiError> {
        let assets = self.assets.lock();
        let total = assets.values().filter(|a| a.tenant_id == tenant_id).count() as u64;

        let items: Vec<Asset> = assets
            .values()
            .filter(|a| a.tenant_id == tenant_id)
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .cloned()
            .collect();

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: AssetStatus,
    ) -> Result<Vec<Asset>, ApiError> {
        let assets = self.assets.lock();
        Ok(assets
            .values()
            .filter(|a| a.tenant_id == tenant_id && a.status == status)
            .cloned()
            .collect())
    }

    async fn find_by_category(
        &self,
        tenant_id: i64,
        category_id: i64,
    ) -> Result<Vec<Asset>, ApiError> {
        let assets = self.assets.lock();
        Ok(assets
            .values()
            .filter(|a| a.tenant_id == tenant_id && a.category_id == Some(category_id))
            .cloned()
            .collect())
    }

    async fn update(&self, id: i64, update: UpdateAsset) -> Result<Asset, ApiError> {
        let mut assets = self.assets.lock();
        let asset = assets
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Asset {} not found", id)))?;

        if let Some(name) = update.name {
            asset.name = name;
        }
        if let Some(description) = update.description {
            asset.description = Some(description);
        }
        if let Some(serial_number) = update.serial_number {
            asset.serial_number = Some(serial_number);
        }
        if let Some(location) = update.location {
            asset.location = Some(location);
        }
        if let Some(status) = update.status {
            asset.status = status;
        }
        if let Some(responsible_person_id) = update.responsible_person_id {
            asset.responsible_person_id = Some(responsible_person_id);
        }
        if let Some(notes) = update.notes {
            asset.notes = Some(notes);
        }

        asset.updated_at = chrono::Utc::now();
        Ok(asset.clone())
    }

    async fn update_status(&self, id: i64, status: AssetStatus) -> Result<Asset, ApiError> {
        let mut assets = self.assets.lock();
        let asset = assets
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Asset {} not found", id)))?;

        asset.status = status;
        asset.updated_at = chrono::Utc::now();
        Ok(asset.clone())
    }

    async fn record_depreciation(&self, id: i64, amount: f64) -> Result<Asset, ApiError> {
        let mut assets = self.assets.lock();
        let asset = assets
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Asset {} not found", id)))?;

        asset.accumulated_depreciation += amount;
        asset.book_value = asset.calculate_book_value();
        asset.updated_at = chrono::Utc::now();

        // Check if fully depreciated
        if asset.book_value <= asset.salvage_value {
            asset.status = AssetStatus::WrittenOff;
        }

        Ok(asset.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.assets.lock().remove(&id);
        self.maintenance_records.lock().remove(&id);
        Ok(())
    }

    async fn create_maintenance_record(
        &self,
        create: CreateMaintenanceRecord,
    ) -> Result<MaintenanceRecord, ApiError> {
        // Validate the record
        if create.maintenance_type.is_empty() || create.maintenance_type.len() > 100 {
            return Err(ApiError::Validation(
                "Maintenance type must be 1-100 characters".to_string(),
            ));
        }
        if create.description.is_empty() || create.description.len() > 1000 {
            return Err(ApiError::Validation(
                "Description must be 1-1000 characters".to_string(),
            ));
        }
        if create.cost < 0.0 {
            return Err(ApiError::Validation(
                "Cost must be non-negative".to_string(),
            ));
        }

        // Verify asset exists
        {
            let assets = self.assets.lock();
            if !assets.contains_key(&create.asset_id) {
                return Err(ApiError::NotFound(format!(
                    "Asset {} not found",
                    create.asset_id
                )));
            }
        }

        let mut next_id = self.next_maintenance_id.lock();
        let id = *next_id;
        *next_id += 1;

        let record = MaintenanceRecord {
            id,
            asset_id: create.asset_id,
            maintenance_date: create.maintenance_date,
            maintenance_type: create.maintenance_type,
            description: create.description,
            cost: create.cost,
            performed_by: create.performed_by,
            next_maintenance_date: create.next_maintenance_date,
            created_at: chrono::Utc::now(),
        };

        self.maintenance_records
            .lock()
            .entry(create.asset_id)
            .or_default()
            .push(record.clone());

        Ok(record)
    }

    async fn get_maintenance_records(
        &self,
        asset_id: i64,
    ) -> Result<Vec<MaintenanceRecord>, ApiError> {
        let records = self.maintenance_records.lock();
        Ok(records.get(&asset_id).cloned().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::assets::model::DepreciationMethod;

    #[actix_web::test]
    async fn test_create_asset() {
        let repo = InMemoryAssetsRepository::new();

        let asset = repo
            .create(CreateAsset {
                tenant_id: 1,
                name: "Test Computer".to_string(),
                category_id: None,
                description: None,
                serial_number: Some("SN12345".to_string()),
                location: Some("Office".to_string()),
                acquisition_date: chrono::Utc::now(),
                acquisition_cost: 5000.0,
                salvage_value: 500.0,
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

        assert_eq!(asset.name, "Test Computer");
        assert_eq!(asset.acquisition_cost, 5000.0);
        assert!(asset.asset_code.starts_with("AST-"));
    }

    #[actix_web::test]
    async fn test_record_depreciation() {
        let repo = InMemoryAssetsRepository::new();

        let asset = repo
            .create(CreateAsset {
                tenant_id: 1,
                name: "Test Asset".to_string(),
                category_id: None,
                description: None,
                serial_number: None,
                location: None,
                acquisition_date: chrono::Utc::now(),
                acquisition_cost: 10000.0,
                salvage_value: 1000.0,
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

        let updated = repo.record_depreciation(asset.id, 1800.0).await.unwrap();
        assert_eq!(updated.accumulated_depreciation, 1800.0);
        assert_eq!(updated.book_value, 8200.0);
    }

    #[actix_web::test]
    async fn test_create_maintenance_record() {
        let repo = InMemoryAssetsRepository::new();

        let asset = repo
            .create(CreateAsset {
                tenant_id: 1,
                name: "Test Machine".to_string(),
                category_id: None,
                description: None,
                serial_number: None,
                location: None,
                acquisition_date: chrono::Utc::now(),
                acquisition_cost: 10000.0,
                salvage_value: 1000.0,
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

        let record = repo
            .create_maintenance_record(CreateMaintenanceRecord {
                asset_id: asset.id,
                maintenance_date: chrono::Utc::now(),
                maintenance_type: "Preventive".to_string(),
                description: "Annual maintenance".to_string(),
                cost: 500.0,
                performed_by: Some("John Doe".to_string()),
                next_maintenance_date: Some(chrono::Utc::now() + chrono::Duration::days(365)),
            })
            .await
            .unwrap();

        assert_eq!(record.asset_id, asset.id);
        assert_eq!(record.maintenance_type, "Preventive");

        let records = repo.get_maintenance_records(asset.id).await.unwrap();
        assert_eq!(records.len(), 1);
    }
}
