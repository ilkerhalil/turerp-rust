//! PostgreSQL stock repository implementation

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::stock::model::{
    CreateStockMovement, CreateWarehouse, MovementType, StockLevel, StockMovement, Warehouse,
};
use crate::domain::stock::repository::{
    BoxStockLevelRepository, BoxStockMovementRepository, BoxWarehouseRepository,
    StockLevelRepository, StockMovementRepository, WarehouseRepository,
};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

// ============================================================================
// Warehouse Row and Repository
// ============================================================================

/// Database row representation for Warehouse
#[derive(Debug, FromRow)]
struct WarehouseRow {
    id: i64,
    tenant_id: i64,
    code: String,
    name: String,
    address: Option<String>,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    total_count: Option<i64>,
}

impl From<WarehouseRow> for Warehouse {
    fn from(row: WarehouseRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            code: row.code,
            name: row.name,
            address: row.address,
            is_active: row.is_active,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL warehouse repository
pub struct PostgresWarehouseRepository {
    pool: Arc<PgPool>,
}

impl PostgresWarehouseRepository {
    /// Create a new PostgreSQL warehouse repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxWarehouseRepository {
        Arc::new(self) as BoxWarehouseRepository
    }
}

#[async_trait]
impl WarehouseRepository for PostgresWarehouseRepository {
    async fn create(&self, warehouse: CreateWarehouse) -> Result<Warehouse, ApiError> {
        let row: WarehouseRow = sqlx::query_as(
            r#"
            INSERT INTO warehouses (tenant_id, code, name, address, is_active, created_at)
            VALUES ($1, $2, $3, $4, true, NOW())
            RETURNING id, tenant_id, code, name, address, is_active, created_at
            "#,
        )
        .bind(warehouse.tenant_id)
        .bind(&warehouse.code)
        .bind(&warehouse.name)
        .bind(&warehouse.address)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Warehouse"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Warehouse>, ApiError> {
        let result: Option<WarehouseRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, address, is_active, created_at
            FROM warehouses
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find warehouse by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Warehouse>, ApiError> {
        let rows: Vec<WarehouseRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, address, is_active, created_at
            FROM warehouses
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Warehouse"))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Warehouse>, ApiError> {
        let offset = (page.saturating_sub(1)) * per_page;
        let rows: Vec<WarehouseRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, address, is_active, created_at, COUNT(*) OVER() as total_count
            FROM warehouses
            WHERE tenant_id = $1
            ORDER BY id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Warehouse"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<Warehouse> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(
        &self,
        id: i64,
        code: Option<String>,
        name: Option<String>,
        address: Option<String>,
        is_active: Option<bool>,
    ) -> Result<Warehouse, ApiError> {
        let row: WarehouseRow = sqlx::query_as(
            r#"
            UPDATE warehouses
            SET
                code = COALESCE($1, code),
                name = COALESCE($2, name),
                address = COALESCE($3, address),
                is_active = COALESCE($4, is_active)
            WHERE id = $5
            RETURNING id, tenant_id, code, name, address, is_active, created_at
            "#,
        )
        .bind(&code)
        .bind(&name)
        .bind(&address)
        .bind(is_active)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Warehouse"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM warehouses
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete warehouse: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Warehouse not found".to_string()));
        }

        Ok(())
    }
}

// ============================================================================
// StockLevel Row and Repository
// ============================================================================

/// Database row representation for StockLevel
#[derive(Debug, FromRow)]
struct StockLevelRow {
    id: i64,
    warehouse_id: i64,
    product_id: i64,
    quantity: Decimal,
    reserved_quantity: Decimal,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<StockLevelRow> for StockLevel {
    fn from(row: StockLevelRow) -> Self {
        Self {
            id: row.id,
            warehouse_id: row.warehouse_id,
            product_id: row.product_id,
            quantity: row.quantity,
            reserved_quantity: row.reserved_quantity,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL stock level repository
pub struct PostgresStockLevelRepository {
    pool: Arc<PgPool>,
}

impl PostgresStockLevelRepository {
    /// Create a new PostgreSQL stock level repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxStockLevelRepository {
        Arc::new(self) as BoxStockLevelRepository
    }
}

#[async_trait]
impl StockLevelRepository for PostgresStockLevelRepository {
    async fn find_by_warehouse_product(
        &self,
        warehouse_id: i64,
        product_id: i64,
    ) -> Result<Option<StockLevel>, ApiError> {
        let result: Option<StockLevelRow> = sqlx::query_as(
            r#"
            SELECT id, warehouse_id, product_id, quantity, reserved_quantity, updated_at
            FROM stock_levels
            WHERE warehouse_id = $1 AND product_id = $2
            "#,
        )
        .bind(warehouse_id)
        .bind(product_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find stock level: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<StockLevel>, ApiError> {
        let rows: Vec<StockLevelRow> = sqlx::query_as(
            r#"
            SELECT id, warehouse_id, product_id, quantity, reserved_quantity, updated_at
            FROM stock_levels
            WHERE product_id = $1
            ORDER BY warehouse_id
            "#,
        )
        .bind(product_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find stock levels by product: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_warehouse(&self, warehouse_id: i64) -> Result<Vec<StockLevel>, ApiError> {
        let rows: Vec<StockLevelRow> = sqlx::query_as(
            r#"
            SELECT id, warehouse_id, product_id, quantity, reserved_quantity, updated_at
            FROM stock_levels
            WHERE warehouse_id = $1
            ORDER BY product_id
            "#,
        )
        .bind(warehouse_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find stock levels by warehouse: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<StockLevel, ApiError> {
        // UPSERT: insert if not exists, update quantity if exists
        let row: StockLevelRow = sqlx::query_as(
            r#"
            INSERT INTO stock_levels (warehouse_id, product_id, quantity, reserved_quantity, updated_at)
            VALUES ($1, $2, $3, 0, NOW())
            ON CONFLICT (warehouse_id, product_id)
            DO UPDATE SET quantity = $3, updated_at = NOW()
            RETURNING id, warehouse_id, product_id, quantity, reserved_quantity, updated_at
            "#,
        )
        .bind(warehouse_id)
        .bind(product_id)
        .bind(quantity)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to update stock quantity: {}", e)))?;

        Ok(row.into())
    }

    async fn reserve_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<StockLevel, ApiError> {
        // UPSERT: create row with zero quantities if not exists, then increment reserved
        let row: StockLevelRow = sqlx::query_as(
            r#"
            INSERT INTO stock_levels (warehouse_id, product_id, quantity, reserved_quantity, updated_at)
            VALUES ($1, $2, 0, $3, NOW())
            ON CONFLICT (warehouse_id, product_id)
            DO UPDATE SET reserved_quantity = reserved_quantity + $3, updated_at = NOW()
            RETURNING id, warehouse_id, product_id, quantity, reserved_quantity, updated_at
            "#,
        )
        .bind(warehouse_id)
        .bind(product_id)
        .bind(quantity)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to reserve stock quantity: {}", e)))?;

        // Validate available stock after reservation
        let available = row.quantity - row.reserved_quantity;
        if available < Decimal::ZERO {
            // Roll back: subtract the reserved quantity we just added
            sqlx::query(
                r#"
                UPDATE stock_levels
                SET reserved_quantity = reserved_quantity - $1, updated_at = NOW()
                WHERE warehouse_id = $2 AND product_id = $3
                "#,
            )
            .bind(quantity)
            .bind(warehouse_id)
            .bind(product_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| {
                ApiError::Database(format!("Failed to rollback stock reservation: {}", e))
            })?;

            return Err(ApiError::BadRequest(format!(
                "Insufficient stock. Available: {}, requested: {}",
                available + quantity,
                quantity
            )));
        }

        Ok(row.into())
    }

    async fn release_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<StockLevel, ApiError> {
        // UPSERT: create row if not exists, then decrement reserved (floor at 0)
        let row: StockLevelRow = sqlx::query_as(
            r#"
            INSERT INTO stock_levels (warehouse_id, product_id, quantity, reserved_quantity, updated_at)
            VALUES ($1, $2, 0, 0, NOW())
            ON CONFLICT (warehouse_id, product_id)
            DO UPDATE SET reserved_quantity = GREATEST(reserved_quantity - $3, 0), updated_at = NOW()
            RETURNING id, warehouse_id, product_id, quantity, reserved_quantity, updated_at
            "#,
        )
        .bind(warehouse_id)
        .bind(product_id)
        .bind(quantity)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to release stock quantity: {}", e)))?;

        Ok(row.into())
    }
}

// ============================================================================
// StockMovement Row and Repository
// ============================================================================

/// Database row representation for StockMovement
#[derive(Debug, FromRow)]
struct StockMovementRow {
    id: i64,
    warehouse_id: i64,
    product_id: i64,
    movement_type: String,
    quantity: Decimal,
    reference_type: Option<String>,
    reference_id: Option<i64>,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    created_by: i64,
}

impl From<StockMovementRow> for StockMovement {
    fn from(row: StockMovementRow) -> Self {
        let movement_type = row.movement_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid movement_type '{}' in database: {}, defaulting to Adjustment",
                row.movement_type,
                e
            );
            MovementType::Adjustment
        });

        Self {
            id: row.id,
            warehouse_id: row.warehouse_id,
            product_id: row.product_id,
            movement_type,
            quantity: row.quantity,
            reference_type: row.reference_type,
            reference_id: row.reference_id,
            notes: row.notes,
            created_at: row.created_at,
            created_by: row.created_by,
        }
    }
}

/// PostgreSQL stock movement repository
pub struct PostgresStockMovementRepository {
    pool: Arc<PgPool>,
}

impl PostgresStockMovementRepository {
    /// Create a new PostgreSQL stock movement repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxStockMovementRepository {
        Arc::new(self) as BoxStockMovementRepository
    }
}

#[async_trait]
impl StockMovementRepository for PostgresStockMovementRepository {
    async fn create(&self, movement: CreateStockMovement) -> Result<StockMovement, ApiError> {
        let movement_type_str = movement.movement_type.to_string();

        let row: StockMovementRow = sqlx::query_as(
            r#"
            INSERT INTO stock_movements (warehouse_id, product_id, movement_type, quantity,
                                          reference_type, reference_id, notes, created_at, created_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), $8)
            RETURNING id, warehouse_id, product_id, movement_type, quantity,
                      reference_type, reference_id, notes, created_at, created_by
            "#,
        )
        .bind(movement.warehouse_id)
        .bind(movement.product_id)
        .bind(&movement_type_str)
        .bind(movement.quantity)
        .bind(&movement.reference_type)
        .bind(movement.reference_id)
        .bind(&movement.notes)
        .bind(movement.created_by)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "StockMovement"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<StockMovement>, ApiError> {
        let result: Option<StockMovementRow> = sqlx::query_as(
            r#"
            SELECT id, warehouse_id, product_id, movement_type, quantity,
                   reference_type, reference_id, notes, created_at, created_by
            FROM stock_movements
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find stock movement by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<StockMovement>, ApiError> {
        let rows: Vec<StockMovementRow> = sqlx::query_as(
            r#"
            SELECT id, warehouse_id, product_id, movement_type, quantity,
                   reference_type, reference_id, notes, created_at, created_by
            FROM stock_movements
            WHERE product_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(product_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find stock movements by product: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_warehouse(&self, warehouse_id: i64) -> Result<Vec<StockMovement>, ApiError> {
        let rows: Vec<StockMovementRow> = sqlx::query_as(
            r#"
            SELECT id, warehouse_id, product_id, movement_type, quantity,
                   reference_type, reference_id, notes, created_at, created_by
            FROM stock_movements
            WHERE warehouse_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(warehouse_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find stock movements by warehouse: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_reference(
        &self,
        reference_type: &str,
        reference_id: i64,
    ) -> Result<Vec<StockMovement>, ApiError> {
        let rows: Vec<StockMovementRow> = sqlx::query_as(
            r#"
            SELECT id, warehouse_id, product_id, movement_type, quantity,
                   reference_type, reference_id, notes, created_at, created_by
            FROM stock_movements
            WHERE reference_type = $1 AND reference_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(reference_type)
        .bind(reference_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find stock movements by reference: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}
