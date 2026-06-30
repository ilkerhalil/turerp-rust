//! PostgreSQL forecasting repository implementation

use async_trait::async_trait;
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::forecasting::repository::{
    ForecastProduct, ForecastingRepository, HistoricalSale,
};
use crate::domain::stock::model::StockLevel;
use crate::error::ApiError;

/// Database row for product info
#[derive(Debug, FromRow)]
struct ProductRow {
    id: i64,
    tenant_id: i64,
    name: String,
}

impl From<ProductRow> for ForecastProduct {
    fn from(row: ProductRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
        }
    }
}

/// Database row for historical sales from stock movements
#[derive(Debug, FromRow)]
struct HistoricalSaleRow {
    product_id: i64,
    warehouse_id: i64,
    quantity: Decimal,
    sale_date: chrono::DateTime<chrono::Utc>,
}

impl From<HistoricalSaleRow> for HistoricalSale {
    fn from(row: HistoricalSaleRow) -> Self {
        Self {
            product_id: row.product_id,
            warehouse_id: row.warehouse_id,
            quantity: row.quantity,
            sale_date: row.sale_date,
        }
    }
}

/// Database row for stock levels
#[derive(Debug, FromRow)]
struct StockLevelRow {
    id: i64,
    tenant_id: i64,
    warehouse_id: i64,
    product_id: i64,
    quantity: Decimal,
    reserved_quantity: Decimal,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<StockLevelRow> for StockLevel {
    fn from(row: StockLevelRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            warehouse_id: row.warehouse_id,
            product_id: row.product_id,
            quantity: row.quantity,
            reserved_quantity: row.reserved_quantity,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL forecasting repository
pub struct PostgresForecastingRepository {
    pool: Arc<PgPool>,
}

impl PostgresForecastingRepository {
    /// Create a new PostgreSQL forecasting repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> Arc<dyn ForecastingRepository> {
        Arc::new(self) as Arc<dyn ForecastingRepository>
    }
}

#[async_trait]
impl ForecastingRepository for PostgresForecastingRepository {
    async fn get_products(&self, tenant_id: i64) -> Result<Vec<ForecastProduct>, ApiError> {
        let rows: Vec<ProductRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name
            FROM products
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY name
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Product"))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_historical_sales(
        &self,
        tenant_id: i64,
        product_id: Option<i64>,
        warehouse_id: Option<i64>,
        days_back: i32,
    ) -> Result<Vec<HistoricalSale>, ApiError> {
        let cutoff = Utc::now() - chrono::Duration::days(days_back as i64);

        let rows: Vec<HistoricalSaleRow> = sqlx::query_as(
            r#"
            SELECT sm.product_id, sm.warehouse_id, sm.quantity, sm.created_at as sale_date
            FROM stock_movements sm
            JOIN warehouses w ON w.id = sm.warehouse_id
            WHERE w.tenant_id = $1
              AND sm.movement_type = 'Sale'
              AND sm.created_at >= $2
              AND sm.deleted_at IS NULL
              AND ($3::bigint IS NULL OR sm.product_id = $3)
              AND ($4::bigint IS NULL OR sm.warehouse_id = $4)
            ORDER BY sm.created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(cutoff)
        .bind(product_id)
        .bind(warehouse_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to fetch historical sales: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_stock_levels(
        &self,
        tenant_id: i64,
        warehouse_id: Option<i64>,
    ) -> Result<Vec<StockLevel>, ApiError> {
        let rows: Vec<StockLevelRow> = sqlx::query_as(
            r#"
            SELECT sl.id, sl.tenant_id, sl.warehouse_id, sl.product_id, sl.quantity,
                   sl.reserved_quantity, sl.updated_at, sl.deleted_at, sl.deleted_by
            FROM stock_levels sl
            JOIN warehouses w ON w.id = sl.warehouse_id
            WHERE w.tenant_id = $1
              AND sl.deleted_at IS NULL
              AND ($2::bigint IS NULL OR sl.warehouse_id = $2)
            ORDER BY sl.product_id
            "#,
        )
        .bind(tenant_id)
        .bind(warehouse_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to fetch stock levels: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_stock_level(
        &self,
        warehouse_id: i64,
        product_id: i64,
    ) -> Result<Option<StockLevel>, ApiError> {
        let result: Option<StockLevelRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, warehouse_id, product_id, quantity, reserved_quantity,
                   updated_at, deleted_at, deleted_by
            FROM stock_levels
            WHERE warehouse_id = $1 AND product_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(warehouse_id)
        .bind(product_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to fetch stock level: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }
}
