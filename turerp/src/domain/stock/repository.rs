//! Stock repository

use async_trait::async_trait;
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::common::SoftDeletable;
use crate::domain::stock::model::{
    CreateStockMovement, CreateWarehouse, StockLevel, StockMovement, Warehouse,
};
use crate::error::ApiError;

/// Repository trait for Warehouse operations
#[async_trait]
pub trait WarehouseRepository: Send + Sync {
    async fn create(&self, warehouse: CreateWarehouse) -> Result<Warehouse, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Warehouse>, ApiError>;
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
        tenant_id: i64,
        code: Option<String>,
        name: Option<String>,
        address: Option<String>,
        is_active: Option<bool>,
    ) -> Result<Warehouse, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Soft delete a warehouse
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    /// Restore a soft-deleted warehouse
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Warehouse, ApiError>;

    /// Find soft-deleted warehouses (admin use)
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Warehouse>, ApiError>;

    /// Hard delete a warehouse (permanent destruction — admin only)
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for StockLevel operations.
/// Note: StockLevel does not have a tenant_id field; it is a child entity of Warehouse.
/// Tenant isolation is enforced via the warehouse_id relationship.
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

    /// Soft delete a stock level
    async fn soft_delete(
        &self,
        warehouse_id: i64,
        product_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError>;

    /// Restore a soft-deleted stock level
    async fn restore(&self, warehouse_id: i64, product_id: i64) -> Result<StockLevel, ApiError>;

    /// Find soft-deleted stock levels
    async fn find_deleted(&self, warehouse_id: i64) -> Result<Vec<StockLevel>, ApiError>;

    /// Hard delete a stock level (permanent destruction — admin only)
    async fn destroy(&self, warehouse_id: i64, product_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for StockMovement operations.
/// Note: StockMovement does not have a tenant_id field directly; tenant isolation
/// is enforced via the warehouse relationship. The tenant_id parameter is used to
/// join against warehouses and ensure tenant scoping.
#[async_trait]
pub trait StockMovementRepository: Send + Sync {
    async fn create(&self, movement: CreateStockMovement) -> Result<StockMovement, ApiError>;
    /// Find by ID with tenant isolation (joins warehouses to verify tenant ownership)
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<StockMovement>, ApiError>;
    async fn find_by_product(&self, product_id: i64) -> Result<Vec<StockMovement>, ApiError>;
    async fn find_by_warehouse(&self, warehouse_id: i64) -> Result<Vec<StockMovement>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<StockMovement>, ApiError>;
    async fn find_by_reference(
        &self,
        reference_type: &str,
        reference_id: i64,
    ) -> Result<Vec<StockMovement>, ApiError>;

    /// Soft delete a stock movement with tenant isolation
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    /// Restore a soft-deleted stock movement with tenant isolation
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<StockMovement, ApiError>;

    /// Find soft-deleted stock movements
    async fn find_deleted(&self, warehouse_id: i64) -> Result<Vec<StockMovement>, ApiError>;

    /// Hard delete a stock movement with tenant isolation (permanent destruction — admin only)
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
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
            deleted_at: None,
            deleted_by: None,
        };

        inner.warehouses.insert(id, warehouse.clone());
        Ok(warehouse)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Warehouse>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .warehouses
            .get(&id)
            .filter(|w| w.tenant_id == tenant_id && !w.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Warehouse>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .warehouses
            .values()
            .filter(|w| w.tenant_id == tenant_id && !w.is_deleted())
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
            .filter(|w| w.tenant_id == tenant_id && !w.is_deleted())
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
        tenant_id: i64,
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

        if warehouse.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Warehouse {} not found", id)));
        }

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

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let warehouse = inner
            .warehouses
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Warehouse {} not found", id)))?;

        if warehouse.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Warehouse {} not found", id)));
        }

        inner.warehouses.remove(&id);
        Ok(())
    }

    async fn soft_delete(&self, id: i64, _tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let warehouse = inner
            .warehouses
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Warehouse {} not found", id)))?;
        warehouse.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, _tenant_id: i64) -> Result<Warehouse, ApiError> {
        let mut inner = self.inner.lock();
        let warehouse = inner
            .warehouses
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Warehouse {} not found", id)))?;
        warehouse.restore();
        Ok(warehouse.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Warehouse>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .warehouses
            .values()
            .filter(|w| w.tenant_id == tenant_id && w.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, _tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner
            .warehouses
            .remove(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Warehouse {} not found", id)))?;
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
        Ok(inner
            .levels
            .get(&(warehouse_id, product_id))
            .filter(|l| !l.is_deleted())
            .cloned())
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<StockLevel>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .levels
            .values()
            .filter(|l| l.product_id == product_id && !l.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_warehouse(&self, warehouse_id: i64) -> Result<Vec<StockLevel>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .levels
            .values()
            .filter(|l| l.warehouse_id == warehouse_id && !l.is_deleted())
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
                    deleted_at: None,
                    deleted_by: None,
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
                    deleted_at: None,
                    deleted_by: None,
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
                    deleted_at: None,
                    deleted_by: None,
                },
            );
        }

        let level = inner.levels.get_mut(&key).unwrap();
        level.reserved_quantity = (level.reserved_quantity - quantity).max(Decimal::ZERO);
        level.updated_at = chrono::Utc::now();
        Ok(level.clone())
    }

    async fn soft_delete(
        &self,
        warehouse_id: i64,
        product_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let key = (warehouse_id, product_id);
        let level = inner
            .levels
            .get_mut(&key)
            .ok_or_else(|| ApiError::NotFound("Stock level not found".to_string()))?;
        level.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, warehouse_id: i64, product_id: i64) -> Result<StockLevel, ApiError> {
        let mut inner = self.inner.lock();
        let key = (warehouse_id, product_id);
        let level = inner
            .levels
            .get_mut(&key)
            .ok_or_else(|| ApiError::NotFound("Stock level not found".to_string()))?;
        level.restore();
        Ok(level.clone())
    }

    async fn find_deleted(&self, warehouse_id: i64) -> Result<Vec<StockLevel>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .levels
            .values()
            .filter(|l| l.warehouse_id == warehouse_id && l.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, warehouse_id: i64, product_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let key = (warehouse_id, product_id);
        inner
            .levels
            .remove(&key)
            .ok_or_else(|| ApiError::NotFound("Stock level not found".to_string()))?;
        Ok(())
    }
}

/// Inner state for InMemoryStockMovementRepository
struct InMemoryStockMovementInner {
    movements: std::collections::HashMap<i64, StockMovement>,
    /// Maps warehouse_id -> tenant_id for tenant isolation of movements
    warehouse_tenants: std::collections::HashMap<i64, i64>,
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
                warehouse_tenants: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }

    /// Register a warehouse's tenant_id for tenant isolation of movements.
    /// This must be called before creating movements for a warehouse.
    pub fn register_warehouse_tenant(&self, warehouse_id: i64, tenant_id: i64) {
        let mut inner = self.inner.lock();
        inner.warehouse_tenants.insert(warehouse_id, tenant_id);
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
            deleted_at: None,
            deleted_by: None,
        };

        inner.movements.insert(id, movement.clone());
        Ok(movement)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<StockMovement>, ApiError> {
        let inner = self.inner.lock();
        // StockMovement does not have tenant_id directly; look up via warehouse
        let movement = inner
            .movements
            .get(&id)
            .filter(|m| !m.is_deleted())
            .cloned();
        if let Some(ref m) = movement {
            // Verify the warehouse belongs to the tenant
            match inner.warehouse_tenants.get(&m.warehouse_id) {
                Some(&w_tenant_id) if w_tenant_id == tenant_id => {}
                _ => return Ok(None),
            }
        }
        Ok(movement)
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<StockMovement>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .movements
            .values()
            .filter(|m| m.product_id == product_id && !m.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_warehouse(&self, warehouse_id: i64) -> Result<Vec<StockMovement>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .movements
            .values()
            .filter(|m| m.warehouse_id == warehouse_id && !m.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<StockMovement>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .movements
            .values()
            .filter(|m| {
                !m.is_deleted()
                    && inner
                        .warehouse_tenants
                        .get(&m.warehouse_id)
                        .is_some_and(|&t| t == tenant_id)
            })
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
                !m.is_deleted()
                    && m.reference_type.as_deref() == Some(reference_type)
                    && m.reference_id == Some(reference_id)
            })
            .cloned()
            .collect())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        // First, look up the warehouse_id immutably
        let warehouse_id = inner
            .movements
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Stock movement {} not found", id)))?
            .warehouse_id;

        // Verify tenant ownership via warehouse
        match inner.warehouse_tenants.get(&warehouse_id) {
            Some(&w_tenant_id) if w_tenant_id == tenant_id => {}
            _ => {
                return Err(ApiError::NotFound(format!(
                    "Stock movement {} not found",
                    id
                )))
            }
        }

        inner
            .movements
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Stock movement {} not found", id)))?
            .mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<StockMovement, ApiError> {
        let mut inner = self.inner.lock();
        // First, look up the warehouse_id immutably
        let warehouse_id = inner
            .movements
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Stock movement {} not found", id)))?
            .warehouse_id;

        // Verify tenant ownership via warehouse
        match inner.warehouse_tenants.get(&warehouse_id) {
            Some(&w_tenant_id) if w_tenant_id == tenant_id => {}
            _ => {
                return Err(ApiError::NotFound(format!(
                    "Stock movement {} not found",
                    id
                )))
            }
        }

        let movement = inner
            .movements
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Stock movement {} not found", id)))?;
        movement.restore();
        Ok(movement.clone())
    }

    async fn find_deleted(&self, warehouse_id: i64) -> Result<Vec<StockMovement>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .movements
            .values()
            .filter(|m| m.warehouse_id == warehouse_id && m.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        // First, look up the warehouse_id and verify tenant
        let warehouse_id = inner
            .movements
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Stock movement {} not found", id)))?
            .warehouse_id;

        match inner.warehouse_tenants.get(&warehouse_id) {
            Some(&w_tenant_id) if w_tenant_id == tenant_id => {}
            _ => {
                return Err(ApiError::NotFound(format!(
                    "Stock movement {} not found",
                    id
                )))
            }
        }

        inner
            .movements
            .remove(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Stock movement {} not found", id)))?;
        Ok(())
    }
}
