//! Stock repository

use async_trait::async_trait;
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::domain::stock::model::{
    CreateStockMovement, CreateWarehouse, StockLevel, StockMovement, Warehouse,
};
use crate::error::ApiError;

/// Repository trait for Warehouse operations
#[async_trait]
pub trait WarehouseRepository: Send + Sync {
    async fn create(&self, warehouse: CreateWarehouse) -> Result<Warehouse, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Warehouse>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Warehouse>, ApiError>;
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Warehouse>, ApiError>;
    async fn update(
        &self,
        id: i64,
        code: Option<String>,
        name: Option<String>,
        address: Option<String>,
        is_active: Option<bool>,
    ) -> Result<Warehouse, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for StockLevel operations
#[async_trait]
pub trait StockLevelRepository: Send + Sync {
    async fn find_by_warehouse_product(
        &self,
        warehouse_id: i64,
        product_id: i64,
    ) -> Result<Option<StockLevel>, ApiError>;
    async fn find_by_product(&self, product_id: i64) -> Result<Vec<StockLevel>, ApiError>;
    async fn find_by_warehouse(&self, warehouse_id: i64) -> Result<Vec<StockLevel>, ApiError>;
    async fn update_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<StockLevel, ApiError>;
    async fn reserve_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<StockLevel, ApiError>;
    async fn release_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<StockLevel, ApiError>;
}

/// Repository trait for StockMovement operations
#[async_trait]
pub trait StockMovementRepository: Send + Sync {
    async fn create(&self, movement: CreateStockMovement) -> Result<StockMovement, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<StockMovement>, ApiError>;
    async fn find_by_product(&self, product_id: i64) -> Result<Vec<StockMovement>, ApiError>;
    async fn find_by_warehouse(&self, warehouse_id: i64) -> Result<Vec<StockMovement>, ApiError>;
    async fn find_by_reference(
        &self,
        reference_type: &str,
        reference_id: i64,
    ) -> Result<Vec<StockMovement>, ApiError>;
}

/// Type aliases
pub type BoxWarehouseRepository = Arc<dyn WarehouseRepository>;
pub type BoxStockLevelRepository = Arc<dyn StockLevelRepository>;
pub type BoxStockMovementRepository = Arc<dyn StockMovementRepository>;

/// Inner state for InMemoryWarehouseRepository
struct InMemoryWarehouseInner {
    warehouses: std::collections::HashMap<i64, Warehouse>,
    next_id: i64,
}

/// In-memory warehouse repository
pub struct InMemoryWarehouseRepository {
    inner: Mutex<InMemoryWarehouseInner>,
}

impl InMemoryWarehouseRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryWarehouseInner {
                warehouses: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryWarehouseRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WarehouseRepository for InMemoryWarehouseRepository {
    async fn create(&self, create: CreateWarehouse) -> Result<Warehouse, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let warehouse = Warehouse {
            id,
            tenant_id: create.tenant_id,
            code: create.code,
            name: create.name,
            address: create.address,
            is_active: true,
            created_at: chrono::Utc::now(),
        };

        inner.warehouses.insert(id, warehouse.clone());
        Ok(warehouse)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Warehouse>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.warehouses.get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Warehouse>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .warehouses
            .values()
            .filter(|w| w.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Warehouse>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .warehouses
            .values()
            .filter(|w| w.tenant_id == tenant_id)
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
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
        let mut inner = self.inner.lock();
        let warehouse = inner
            .warehouses
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Warehouse {} not found", id)))?;

        if let Some(c) = code {
            warehouse.code = c;
        }
        if let Some(n) = name {
            warehouse.name = n;
        }
        if let Some(a) = address {
            warehouse.address = Some(a);
        }
        if let Some(active) = is_active {
            warehouse.is_active = active;
        }

        Ok(warehouse.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.warehouses.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryStockLevelRepository
struct InMemoryStockLevelInner {
    levels: std::collections::HashMap<(i64, i64), StockLevel>,
    next_id: i64,
}

/// In-memory stock level repository
pub struct InMemoryStockLevelRepository {
    inner: Mutex<InMemoryStockLevelInner>,
}

impl InMemoryStockLevelRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryStockLevelInner {
                levels: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryStockLevelRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StockLevelRepository for InMemoryStockLevelRepository {
    async fn find_by_warehouse_product(
        &self,
        warehouse_id: i64,
        product_id: i64,
    ) -> Result<Option<StockLevel>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.levels.get(&(warehouse_id, product_id)).cloned())
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<StockLevel>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .levels
            .values()
            .filter(|l| l.product_id == product_id)
            .cloned()
            .collect())
    }

    async fn find_by_warehouse(&self, warehouse_id: i64) -> Result<Vec<StockLevel>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .levels
            .values()
            .filter(|l| l.warehouse_id == warehouse_id)
            .cloned()
            .collect())
    }

    async fn update_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<StockLevel, ApiError> {
        let mut inner = self.inner.lock();
        let key = (warehouse_id, product_id);

        if !inner.levels.contains_key(&key) {
            let id = inner.next_id;
            inner.next_id += 1;
            inner.levels.insert(
                key,
                StockLevel {
                    id,
                    warehouse_id,
                    product_id,
                    quantity: Decimal::ZERO,
                    reserved_quantity: Decimal::ZERO,
                    updated_at: chrono::Utc::now(),
                },
            );
        }

        let level = inner.levels.get_mut(&key).unwrap();
        level.quantity = quantity;
        level.updated_at = chrono::Utc::now();
        Ok(level.clone())
    }

    async fn reserve_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<StockLevel, ApiError> {
        let mut inner = self.inner.lock();
        let key = (warehouse_id, product_id);

        if !inner.levels.contains_key(&key) {
            let id = inner.next_id;
            inner.next_id += 1;
            inner.levels.insert(
                key,
                StockLevel {
                    id,
                    warehouse_id,
                    product_id,
                    quantity: Decimal::ZERO,
                    reserved_quantity: Decimal::ZERO,
                    updated_at: chrono::Utc::now(),
                },
            );
        }

        let level = inner.levels.get_mut(&key).unwrap();
        let available = level.quantity - level.reserved_quantity;
        if quantity > available {
            return Err(ApiError::BadRequest(format!(
                "Insufficient stock. Available: {}, requested: {}",
                available, quantity
            )));
        }

        level.reserved_quantity += quantity;
        level.updated_at = chrono::Utc::now();
        Ok(level.clone())
    }

    async fn release_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<StockLevel, ApiError> {
        let mut inner = self.inner.lock();
        let key = (warehouse_id, product_id);

        if !inner.levels.contains_key(&key) {
            let id = inner.next_id;
            inner.next_id += 1;
            inner.levels.insert(
                key,
                StockLevel {
                    id,
                    warehouse_id,
                    product_id,
                    quantity: Decimal::ZERO,
                    reserved_quantity: Decimal::ZERO,
                    updated_at: chrono::Utc::now(),
                },
            );
        }

        let level = inner.levels.get_mut(&key).unwrap();
        level.reserved_quantity = (level.reserved_quantity - quantity).max(Decimal::ZERO);
        level.updated_at = chrono::Utc::now();
        Ok(level.clone())
    }
}

/// Inner state for InMemoryStockMovementRepository
struct InMemoryStockMovementInner {
    movements: std::collections::HashMap<i64, StockMovement>,
    next_id: i64,
}

/// In-memory stock movement repository
pub struct InMemoryStockMovementRepository {
    inner: Mutex<InMemoryStockMovementInner>,
}

impl InMemoryStockMovementRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryStockMovementInner {
                movements: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryStockMovementRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StockMovementRepository for InMemoryStockMovementRepository {
    async fn create(&self, create: CreateStockMovement) -> Result<StockMovement, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let movement = StockMovement {
            id,
            warehouse_id: create.warehouse_id,
            product_id: create.product_id,
            movement_type: create.movement_type,
            quantity: create.quantity,
            reference_type: create.reference_type,
            reference_id: create.reference_id,
            notes: create.notes,
            created_at: chrono::Utc::now(),
            created_by: create.created_by,
        };

        inner.movements.insert(id, movement.clone());
        Ok(movement)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<StockMovement>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.movements.get(&id).cloned())
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<StockMovement>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .movements
            .values()
            .filter(|m| m.product_id == product_id)
            .cloned()
            .collect())
    }

    async fn find_by_warehouse(&self, warehouse_id: i64) -> Result<Vec<StockMovement>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .movements
            .values()
            .filter(|m| m.warehouse_id == warehouse_id)
            .cloned()
            .collect())
    }

    async fn find_by_reference(
        &self,
        reference_type: &str,
        reference_id: i64,
    ) -> Result<Vec<StockMovement>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .movements
            .values()
            .filter(|m| {
                m.reference_type.as_deref() == Some(reference_type)
                    && m.reference_id == Some(reference_id)
            })
            .cloned()
            .collect())
    }
}
