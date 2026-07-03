//! Forecasting repository trait and in-memory implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::domain::stock::model::StockLevel;
use crate::error::ApiError;

/// Product info for forecasting
#[derive(Debug, Clone)]
pub struct ForecastProduct {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
}

/// Historical sale record for forecasting
#[derive(Debug, Clone)]
pub struct HistoricalSale {
    pub product_id: i64,
    pub warehouse_id: i64,
    pub quantity: Decimal,
    pub sale_date: DateTime<Utc>,
}

/// Repository trait for forecasting data access
#[async_trait]
pub trait ForecastingRepository: Send + Sync {
    /// Get products for a tenant
    async fn get_products(&self, tenant_id: i64) -> Result<Vec<ForecastProduct>, ApiError>;

    /// Get historical sale movements for a product (or all products if product_id is None)
    async fn get_historical_sales(
        &self,
        tenant_id: i64,
        product_id: Option<i64>,
        warehouse_id: Option<i64>,
        days_back: i32,
    ) -> Result<Vec<HistoricalSale>, ApiError>;

    /// Get current stock levels for a tenant
    async fn get_stock_levels(
        &self,
        tenant_id: i64,
        warehouse_id: Option<i64>,
    ) -> Result<Vec<StockLevel>, ApiError>;
}

/// Type alias for boxed forecasting repository
pub type BoxForecastingRepository = Arc<dyn ForecastingRepository>;

// ============================================================================
// In-Memory Implementation
// ============================================================================

struct InMemoryForecastingInner {
    products: std::collections::HashMap<i64, ForecastProduct>,
    sales: Vec<HistoricalSale>,
    stock_levels: std::collections::HashMap<(i64, i64), StockLevel>,
    next_id: i64,
}

/// In-memory forecasting repository
pub struct InMemoryForecastingRepository {
    inner: Mutex<InMemoryForecastingInner>,
}

impl InMemoryForecastingRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryForecastingInner {
                products: std::collections::HashMap::new(),
                sales: Vec::new(),
                stock_levels: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }

    /// Seed a product for testing
    pub fn seed_product(&self, id: i64, tenant_id: i64, name: &str) {
        let mut inner = self.inner.lock();
        inner.products.insert(
            id,
            ForecastProduct {
                id,
                tenant_id,
                name: name.to_string(),
            },
        );
    }

    /// Seed a historical sale for testing
    pub fn seed_sale(
        &self,
        product_id: i64,
        warehouse_id: i64,
        quantity: Decimal,
        sale_date: DateTime<Utc>,
    ) {
        let mut inner = self.inner.lock();
        inner.sales.push(HistoricalSale {
            product_id,
            warehouse_id,
            quantity,
            sale_date,
        });
    }

    /// Seed a stock level for testing
    pub fn seed_stock_level(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
        reserved_quantity: Decimal,
    ) {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        inner.stock_levels.insert(
            (warehouse_id, product_id),
            StockLevel {
                id,
                tenant_id: 1,
                warehouse_id,
                product_id,
                quantity,
                reserved_quantity,
                updated_at: Utc::now(),
                deleted_at: None,
                deleted_by: None,
            },
        );
    }
}

impl Default for InMemoryForecastingRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ForecastingRepository for InMemoryForecastingRepository {
    async fn get_products(&self, tenant_id: i64) -> Result<Vec<ForecastProduct>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .products
            .values()
            .filter(|p| p.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn get_historical_sales(
        &self,
        _tenant_id: i64,
        product_id: Option<i64>,
        warehouse_id: Option<i64>,
        days_back: i32,
    ) -> Result<Vec<HistoricalSale>, ApiError> {
        let cutoff = Utc::now() - chrono::Duration::days(days_back as i64);
        let inner = self.inner.lock();
        Ok(inner
            .sales
            .iter()
            .filter(|s| {
                s.sale_date >= cutoff
                    && product_id.is_none_or(|pid| s.product_id == pid)
                    && warehouse_id.is_none_or(|wid| s.warehouse_id == wid)
            })
            .cloned()
            .collect())
    }

    async fn get_stock_levels(
        &self,
        _tenant_id: i64,
        warehouse_id: Option<i64>,
    ) -> Result<Vec<StockLevel>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .stock_levels
            .values()
            .filter(|l| {
                l.deleted_at.is_none() && warehouse_id.is_none_or(|wid| l.warehouse_id == wid)
            })
            .cloned()
            .collect())
    }
}
