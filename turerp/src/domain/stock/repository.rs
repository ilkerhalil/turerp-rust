//! Stock repository

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

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
        quantity: f64,
    ) -> Result<StockLevel, ApiError>;
    async fn reserve_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: f64,
    ) -> Result<StockLevel, ApiError>;
    async fn release_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: f64,
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

/// In-memory warehouse repository
pub struct InMemoryWarehouseRepository {
    warehouses: Mutex<std::collections::HashMap<i64, Warehouse>>,
    next_id: Mutex<i64>,
}

impl InMemoryWarehouseRepository {
    pub fn new() -> Self {
        Self {
            warehouses: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
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
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;

        let warehouse = Warehouse {
            id,
            tenant_id: create.tenant_id,
            code: create.code,
            name: create.name,
            address: create.address,
            is_active: true,
            created_at: chrono::Utc::now(),
        };

        self.warehouses.lock().insert(id, warehouse.clone());
        Ok(warehouse)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Warehouse>, ApiError> {
        Ok(self.warehouses.lock().get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Warehouse>, ApiError> {
        let warehouses = self.warehouses.lock();
        Ok(warehouses
            .values()
            .filter(|w| w.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        id: i64,
        code: Option<String>,
        name: Option<String>,
        address: Option<String>,
        is_active: Option<bool>,
    ) -> Result<Warehouse, ApiError> {
        let mut warehouses = self.warehouses.lock();
        let warehouse = warehouses
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Warehouse {} not found", id)))?;

        if let Some(code) = code {
            warehouse.code = code;
        }
        if let Some(name) = name {
            warehouse.name = name;
        }
        if let Some(address) = address {
            warehouse.address = Some(address);
        }
        if let Some(is_active) = is_active {
            warehouse.is_active = is_active;
        }

        Ok(warehouse.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.warehouses.lock().remove(&id);
        Ok(())
    }
}

/// In-memory stock level repository
pub struct InMemoryStockLevelRepository {
    levels: Mutex<std::collections::HashMap<(i64, i64), StockLevel>>,
    next_id: Mutex<i64>,
}

impl InMemoryStockLevelRepository {
    pub fn new() -> Self {
        Self {
            levels: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
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
        Ok(self.levels.lock().get(&(warehouse_id, product_id)).cloned())
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<StockLevel>, ApiError> {
        let levels = self.levels.lock();
        Ok(levels
            .values()
            .filter(|l| l.product_id == product_id)
            .cloned()
            .collect())
    }

    async fn find_by_warehouse(&self, warehouse_id: i64) -> Result<Vec<StockLevel>, ApiError> {
        let levels = self.levels.lock();
        Ok(levels
            .values()
            .filter(|l| l.warehouse_id == warehouse_id)
            .cloned()
            .collect())
    }

    async fn update_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: f64,
    ) -> Result<StockLevel, ApiError> {
        let mut levels = self.levels.lock();
        let key = (warehouse_id, product_id);

        let level = levels.entry(key).or_insert_with(|| StockLevel {
            id: *self.next_id.lock(),
            warehouse_id,
            product_id,
            quantity: 0.0,
            reserved_quantity: 0.0,
            updated_at: chrono::Utc::now(),
        });

        level.quantity = quantity;
        level.updated_at = chrono::Utc::now();
        Ok(level.clone())
    }

    async fn reserve_quantity(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: f64,
    ) -> Result<StockLevel, ApiError> {
        let mut levels = self.levels.lock();
        let key = (warehouse_id, product_id);

        let level = levels.entry(key).or_insert_with(|| StockLevel {
            id: *self.next_id.lock(),
            warehouse_id,
            product_id,
            quantity: 0.0,
            reserved_quantity: 0.0,
            updated_at: chrono::Utc::now(),
        });

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
        quantity: f64,
    ) -> Result<StockLevel, ApiError> {
        let mut levels = self.levels.lock();
        let key = (warehouse_id, product_id);

        let level = levels.entry(key).or_insert_with(|| StockLevel {
            id: *self.next_id.lock(),
            warehouse_id,
            product_id,
            quantity: 0.0,
            reserved_quantity: 0.0,
            updated_at: chrono::Utc::now(),
        });

        level.reserved_quantity = (level.reserved_quantity - quantity).max(0.0);
        level.updated_at = chrono::Utc::now();
        Ok(level.clone())
    }
}

/// In-memory stock movement repository
pub struct InMemoryStockMovementRepository {
    movements: Mutex<std::collections::HashMap<i64, StockMovement>>,
    next_id: Mutex<i64>,
}

impl InMemoryStockMovementRepository {
    pub fn new() -> Self {
        Self {
            movements: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
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

        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;

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

        self.movements.lock().insert(id, movement.clone());
        Ok(movement)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<StockMovement>, ApiError> {
        Ok(self.movements.lock().get(&id).cloned())
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<StockMovement>, ApiError> {
        let movements = self.movements.lock();
        Ok(movements
            .values()
            .filter(|m| m.product_id == product_id)
            .cloned()
            .collect())
    }

    async fn find_by_warehouse(&self, warehouse_id: i64) -> Result<Vec<StockMovement>, ApiError> {
        let movements = self.movements.lock();
        Ok(movements
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
        let movements = self.movements.lock();
        Ok(movements
            .values()
            .filter(|m| {
                m.reference_type.as_deref() == Some(reference_type)
                    && m.reference_id == Some(reference_id)
            })
            .cloned()
            .collect())
    }
}
