//! PostgreSQL assets repository implementation

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::assets::model::{
    Asset, AssetCategory, AssetStatus, CreateAsset, CreateMaintenanceRecord, DepreciationMethod,
    MaintenanceRecord, UpdateAsset,
};
use crate::domain::assets::repository::{
    AssetCategoryRepository, AssetsRepository, BoxAssetCategoryRepository, BoxAssetsRepository,
};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

/// Parse AssetStatus from database string representation
fn parse_asset_status(s: &str) -> AssetStatus {
    match s {
        "Active" => AssetStatus::Active,
        "InUse" => AssetStatus::InUse,
        "UnderMaintenance" => AssetStatus::UnderMaintenance,
        "Disposed" => AssetStatus::Disposed,
        "WrittenOff" => AssetStatus::WrittenOff,
        _ => {
            tracing::warn!(
                "Invalid asset status '{}' in database, defaulting to Active",
                s
            );
            AssetStatus::default()
        }
    }
}

/// Parse DepreciationMethod from database string representation
fn parse_depreciation_method(s: &str) -> DepreciationMethod {
    match s {
        "StraightLine" => DepreciationMethod::StraightLine,
        "DecliningBalance" => DepreciationMethod::DecliningBalance,
        "UnitsOfProduction" => DepreciationMethod::UnitsOfProduction,
        "None" => DepreciationMethod::None,
        _ => {
            tracing::warn!(
                "Invalid depreciation method '{}' in database, defaulting to StraightLine",
                s
            );
            DepreciationMethod::default()
        }
    }
}

// ============================================================================
// AssetCategory Row and Repository
// ============================================================================

/// Database row representation for AssetCategory
#[derive(Debug, FromRow)]
struct AssetCategoryRow {
    id: i64,
    tenant_id: i64,
    name: String,
    description: Option<String>,
    default_useful_life_years: i32,
    default_depreciation_method: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<AssetCategoryRow> for AssetCategory {
    fn from(row: AssetCategoryRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            description: row.description,
            default_useful_life_years: row.default_useful_life_years,
            default_depreciation_method: parse_depreciation_method(
                &row.default_depreciation_method,
            ),
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL asset category repository
pub struct PostgresAssetCategoryRepository {
    pool: Arc<PgPool>,
}

impl PostgresAssetCategoryRepository {
    /// Create a new PostgreSQL asset category repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxAssetCategoryRepository {
        Arc::new(self) as BoxAssetCategoryRepository
    }
}

#[async_trait]
impl AssetCategoryRepository for PostgresAssetCategoryRepository {
    async fn create(&self, category: AssetCategory) -> Result<AssetCategory, ApiError> {
        let depreciation_method = category.default_depreciation_method.to_string();

        let row: AssetCategoryRow = sqlx::query_as(
            r#"
            INSERT INTO asset_categories (tenant_id, name, description, default_useful_life_years, default_depreciation_method, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            RETURNING id, tenant_id, name, description, default_useful_life_years, default_depreciation_method, created_at
            "#,
        )
        .bind(category.tenant_id)
        .bind(&category.name)
        .bind(&category.description)
        .bind(category.default_useful_life_years)
        .bind(&depreciation_method)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "AssetCategory"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<AssetCategory>, ApiError> {
        let result: Option<AssetCategoryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, default_useful_life_years, default_depreciation_method, created_at
            FROM asset_categories
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find asset category by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<AssetCategory>, ApiError> {
        let rows: Vec<AssetCategoryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, default_useful_life_years, default_depreciation_method, created_at
            FROM asset_categories
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find asset categories by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM asset_categories
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete asset category: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("AssetCategory not found".to_string()));
        }

        Ok(())
    }
}

// ============================================================================
// Asset Row and Repository
// ============================================================================

/// Database row representation for Asset
#[derive(Debug, FromRow)]
struct AssetRow {
    id: i64,
    tenant_id: i64,
    asset_code: String,
    name: String,
    category_id: Option<i64>,
    description: Option<String>,
    serial_number: Option<String>,
    location: Option<String>,
    status: String,
    acquisition_date: chrono::DateTime<chrono::Utc>,
    acquisition_cost: Decimal,
    salvage_value: Decimal,
    useful_life_years: i32,
    depreciation_method: String,
    accumulated_depreciation: Decimal,
    book_value: Decimal,
    warranty_expiry: Option<chrono::DateTime<chrono::Utc>>,
    insurance_number: Option<String>,
    insurance_expiry: Option<chrono::DateTime<chrono::Utc>>,
    responsible_person_id: Option<i64>,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<AssetRow> for Asset {
    fn from(row: AssetRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            asset_code: row.asset_code,
            name: row.name,
            category_id: row.category_id,
            description: row.description,
            serial_number: row.serial_number,
            location: row.location,
            status: parse_asset_status(&row.status),
            acquisition_date: row.acquisition_date,
            acquisition_cost: row.acquisition_cost,
            salvage_value: row.salvage_value,
            useful_life_years: row.useful_life_years,
            depreciation_method: parse_depreciation_method(&row.depreciation_method),
            accumulated_depreciation: row.accumulated_depreciation,
            book_value: row.book_value,
            warranty_expiry: row.warranty_expiry,
            insurance_number: row.insurance_number,
            insurance_expiry: row.insurance_expiry,
            responsible_person_id: row.responsible_person_id,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at.unwrap_or(row.created_at),
        }
    }
}

/// Database row representation for paginated asset queries with total count
#[derive(Debug, FromRow)]
struct AssetRowWithTotal {
    id: i64,
    tenant_id: i64,
    asset_code: String,
    name: String,
    category_id: Option<i64>,
    description: Option<String>,
    serial_number: Option<String>,
    location: Option<String>,
    status: String,
    acquisition_date: chrono::DateTime<chrono::Utc>,
    acquisition_cost: Decimal,
    salvage_value: Decimal,
    useful_life_years: i32,
    depreciation_method: String,
    accumulated_depreciation: Decimal,
    book_value: Decimal,
    warranty_expiry: Option<chrono::DateTime<chrono::Utc>>,
    insurance_number: Option<String>,
    insurance_expiry: Option<chrono::DateTime<chrono::Utc>>,
    responsible_person_id: Option<i64>,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    total: i64,
}

impl From<AssetRowWithTotal> for (Asset, i64) {
    fn from(row: AssetRowWithTotal) -> (Asset, i64) {
        let asset = Asset {
            id: row.id,
            tenant_id: row.tenant_id,
            asset_code: row.asset_code,
            name: row.name,
            category_id: row.category_id,
            description: row.description,
            serial_number: row.serial_number,
            location: row.location,
            status: parse_asset_status(&row.status),
            acquisition_date: row.acquisition_date,
            acquisition_cost: row.acquisition_cost,
            salvage_value: row.salvage_value,
            useful_life_years: row.useful_life_years,
            depreciation_method: parse_depreciation_method(&row.depreciation_method),
            accumulated_depreciation: row.accumulated_depreciation,
            book_value: row.book_value,
            warranty_expiry: row.warranty_expiry,
            insurance_number: row.insurance_number,
            insurance_expiry: row.insurance_expiry,
            responsible_person_id: row.responsible_person_id,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at.unwrap_or(row.created_at),
        };
        (asset, row.total)
    }
}

/// PostgreSQL assets repository
pub struct PostgresAssetsRepository {
    pool: Arc<PgPool>,
}

impl PostgresAssetsRepository {
    /// Create a new PostgreSQL assets repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxAssetsRepository {
        Arc::new(self) as BoxAssetsRepository
    }
}

#[async_trait]
impl AssetsRepository for PostgresAssetsRepository {
    async fn create(&self, create: CreateAsset) -> Result<Asset, ApiError> {
        let status_str = AssetStatus::default().to_string();
        let depreciation_method = create.depreciation_method.unwrap_or_default().to_string();
        let book_value = create.acquisition_cost - create.salvage_value;

        // Generate asset code using timestamp-based approach
        let asset_code = format!("AST-{}", chrono::Utc::now().timestamp_millis());

        let row: AssetRow = sqlx::query_as(
            r#"
            INSERT INTO assets (tenant_id, asset_code, name, category_id, description,
                                serial_number, location, status, acquisition_date,
                                acquisition_cost, salvage_value, useful_life_years,
                                depreciation_method, accumulated_depreciation, book_value,
                                warranty_expiry, insurance_number, insurance_expiry,
                                responsible_person_id, notes, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                    $16, $17, $18, $19, $20, NOW(), NOW())
            RETURNING id, tenant_id, asset_code, name, category_id, description,
                      serial_number, location, status, acquisition_date,
                      acquisition_cost, salvage_value, useful_life_years,
                      depreciation_method, accumulated_depreciation, book_value,
                      warranty_expiry, insurance_number, insurance_expiry,
                      responsible_person_id, notes, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&asset_code)
        .bind(&create.name)
        .bind(create.category_id)
        .bind(&create.description)
        .bind(&create.serial_number)
        .bind(&create.location)
        .bind(&status_str)
        .bind(create.acquisition_date)
        .bind(create.acquisition_cost)
        .bind(create.salvage_value)
        .bind(create.useful_life_years)
        .bind(&depreciation_method)
        .bind(Decimal::ZERO)
        .bind(book_value)
        .bind(create.warranty_expiry)
        .bind(&create.insurance_number)
        .bind(create.insurance_expiry)
        .bind(create.responsible_person_id)
        .bind(&create.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Asset"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Asset>, ApiError> {
        let result: Option<AssetRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, asset_code, name, category_id, description,
                   serial_number, location, status, acquisition_date,
                   acquisition_cost, salvage_value, useful_life_years,
                   depreciation_method, accumulated_depreciation, book_value,
                   warranty_expiry, insurance_number, insurance_expiry,
                   responsible_person_id, notes, created_at, updated_at
            FROM assets
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find asset by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Asset>, ApiError> {
        let rows: Vec<AssetRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, asset_code, name, category_id, description,
                   serial_number, location, status, acquisition_date,
                   acquisition_cost, salvage_value, useful_life_years,
                   depreciation_method, accumulated_depreciation, book_value,
                   warranty_expiry, insurance_number, insurance_expiry,
                   responsible_person_id, notes, created_at, updated_at
            FROM assets
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find assets by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Asset>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;

        let rows: Vec<AssetRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, asset_code, name, category_id, description,
                   serial_number, location, status, acquisition_date,
                   acquisition_cost, salvage_value, useful_life_years,
                   depreciation_method, accumulated_depreciation, book_value,
                   warranty_expiry, insurance_number, insurance_expiry,
                   responsible_person_id, notes, created_at, updated_at,
                   COUNT(*) OVER() as total
            FROM assets
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find assets by tenant paginated: {}", e))
        })?;

        let total = rows.first().map(|r| r.total as u64).unwrap_or(0);
        let items: Vec<Asset> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(asset, _)| asset)
            .collect();

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: AssetStatus,
    ) -> Result<Vec<Asset>, ApiError> {
        let status_str = status.to_string();

        let rows: Vec<AssetRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, asset_code, name, category_id, description,
                   serial_number, location, status, acquisition_date,
                   acquisition_cost, salvage_value, useful_life_years,
                   depreciation_method, accumulated_depreciation, book_value,
                   warranty_expiry, insurance_number, insurance_expiry,
                   responsible_person_id, notes, created_at, updated_at
            FROM assets
            WHERE tenant_id = $1 AND status = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find assets by status: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_category(
        &self,
        tenant_id: i64,
        category_id: i64,
    ) -> Result<Vec<Asset>, ApiError> {
        let rows: Vec<AssetRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, asset_code, name, category_id, description,
                   serial_number, location, status, acquisition_date,
                   acquisition_cost, salvage_value, useful_life_years,
                   depreciation_method, accumulated_depreciation, book_value,
                   warranty_expiry, insurance_number, insurance_expiry,
                   responsible_person_id, notes, created_at, updated_at
            FROM assets
            WHERE tenant_id = $1 AND category_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(category_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find assets by category: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update(&self, id: i64, update: UpdateAsset) -> Result<Asset, ApiError> {
        let status_str = update.status.map(|s| s.to_string());

        let row: AssetRow = sqlx::query_as(
            r#"
            UPDATE assets
            SET
                name = COALESCE($1, name),
                description = COALESCE($2, description),
                serial_number = COALESCE($3, serial_number),
                location = COALESCE($4, location),
                status = COALESCE($5, status),
                responsible_person_id = COALESCE($6, responsible_person_id),
                notes = COALESCE($7, notes),
                updated_at = NOW()
            WHERE id = $8
            RETURNING id, tenant_id, asset_code, name, category_id, description,
                      serial_number, location, status, acquisition_date,
                      acquisition_cost, salvage_value, useful_life_years,
                      depreciation_method, accumulated_depreciation, book_value,
                      warranty_expiry, insurance_number, insurance_expiry,
                      responsible_person_id, notes, created_at, updated_at
            "#,
        )
        .bind(&update.name)
        .bind(&update.description)
        .bind(&update.serial_number)
        .bind(&update.location)
        .bind(&status_str)
        .bind(update.responsible_person_id)
        .bind(&update.notes)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Asset"))?;

        Ok(row.into())
    }

    async fn update_status(&self, id: i64, status: AssetStatus) -> Result<Asset, ApiError> {
        let status_str = status.to_string();

        let row: AssetRow = sqlx::query_as(
            r#"
            UPDATE assets
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, tenant_id, asset_code, name, category_id, description,
                      serial_number, location, status, acquisition_date,
                      acquisition_cost, salvage_value, useful_life_years,
                      depreciation_method, accumulated_depreciation, book_value,
                      warranty_expiry, insurance_number, insurance_expiry,
                      responsible_person_id, notes, created_at, updated_at
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Asset"))?;

        Ok(row.into())
    }

    async fn record_depreciation(&self, id: i64, amount: Decimal) -> Result<Asset, ApiError> {
        let row: AssetRow = sqlx::query_as(
            r#"
            UPDATE assets
            SET accumulated_depreciation = accumulated_depreciation + $1,
                book_value = book_value - $1,
                updated_at = NOW()
            WHERE id = $2
            RETURNING id, tenant_id, asset_code, name, category_id, description,
                      serial_number, location, status, acquisition_date,
                      acquisition_cost, salvage_value, useful_life_years,
                      depreciation_method, accumulated_depreciation, book_value,
                      warranty_expiry, insurance_number, insurance_expiry,
                      responsible_person_id, notes, created_at, updated_at
            "#,
        )
        .bind(amount)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Asset"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM assets
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete asset: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Asset not found".to_string()));
        }

        Ok(())
    }

    async fn create_maintenance_record(
        &self,
        create: CreateMaintenanceRecord,
    ) -> Result<MaintenanceRecord, ApiError> {
        let row: MaintenanceRecordRow = sqlx::query_as(
            r#"
            INSERT INTO maintenance_records (asset_id, maintenance_date, maintenance_type,
                                               description, cost, performed_by,
                                               next_maintenance_date, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING id, asset_id, maintenance_date, maintenance_type,
                      description, cost, performed_by, next_maintenance_date, created_at
            "#,
        )
        .bind(create.asset_id)
        .bind(create.maintenance_date)
        .bind(&create.maintenance_type)
        .bind(&create.description)
        .bind(create.cost)
        .bind(&create.performed_by)
        .bind(create.next_maintenance_date)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "MaintenanceRecord"))?;

        Ok(row.into())
    }

    async fn get_maintenance_records(
        &self,
        asset_id: i64,
    ) -> Result<Vec<MaintenanceRecord>, ApiError> {
        let rows: Vec<MaintenanceRecordRow> = sqlx::query_as(
            r#"
            SELECT id, asset_id, maintenance_date, maintenance_type,
                   description, cost, performed_by, next_maintenance_date, created_at
            FROM maintenance_records
            WHERE asset_id = $1
            ORDER BY maintenance_date DESC
            "#,
        )
        .bind(asset_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get maintenance records: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

// ============================================================================
// MaintenanceRecord Row
// ============================================================================

/// Database row representation for MaintenanceRecord
#[derive(Debug, FromRow)]
struct MaintenanceRecordRow {
    id: i64,
    asset_id: i64,
    maintenance_date: chrono::DateTime<chrono::Utc>,
    maintenance_type: String,
    description: String,
    cost: Decimal,
    performed_by: Option<String>,
    next_maintenance_date: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<MaintenanceRecordRow> for MaintenanceRecord {
    fn from(row: MaintenanceRecordRow) -> Self {
        Self {
            id: row.id,
            asset_id: row.asset_id,
            maintenance_date: row.maintenance_date,
            maintenance_type: row.maintenance_type,
            description: row.description,
            cost: row.cost,
            performed_by: row.performed_by,
            next_maintenance_date: row.next_maintenance_date,
            created_at: row.created_at,
        }
    }
}
