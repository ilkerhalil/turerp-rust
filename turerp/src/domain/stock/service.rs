//! Stock service for business logic
use rust_decimal::Decimal;

use crate::common::pagination::PaginatedResult;
use crate::domain::stock::model::{
    CreateStockMovement, CreateWarehouse, MovementType, StockLevel, StockLevelResponse,
    StockMovement, StockMovementResponse, StockSummary, Warehouse, WarehouseResponse,
};
use crate::domain::stock::repository::{
    BoxStockLevelRepository, BoxStockMovementRepository, BoxWarehouseRepository,
};
use crate::error::ApiError;

/// Stock service
#[derive(Clone)]
pub struct StockService {
    warehouse_repo: BoxWarehouseRepository,
    stock_level_repo: BoxStockLevelRepository,
    stock_movement_repo: BoxStockMovementRepository,
}

impl StockService {
    pub fn new(
        warehouse_repo: BoxWarehouseRepository,
        stock_level_repo: BoxStockLevelRepository,
        stock_movement_repo: BoxStockMovementRepository,
    ) -> Self {
        Self {
            warehouse_repo,
            stock_level_repo,
            stock_movement_repo,
        }
    }

    // Warehouse operations
    pub async fn create_warehouse(
        &self,
        create: CreateWarehouse,
    ) -> Result<WarehouseResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let warehouse = self.warehouse_repo.create(create).await?;
        Ok(warehouse.into())
    }

    pub async fn get_warehouse(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<WarehouseResponse, ApiError> {
        let warehouse = self
            .warehouse_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Warehouse {} not found", id)))?;
        Ok(warehouse.into())
    }

    pub async fn get_warehouses_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<WarehouseResponse>, ApiError> {
        let warehouses = self.warehouse_repo.find_by_tenant(tenant_id).await?;
        Ok(warehouses.into_iter().map(|w| w.into()).collect())
    }

    pub async fn get_warehouses_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<WarehouseResponse>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        let result = self
            .warehouse_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await?;
        Ok(PaginatedResult::new(
            result.items.into_iter().map(|w| w.into()).collect(),
            result.page,
            result.per_page,
            result.total,
        ))
    }

    pub async fn update_warehouse(
        &self,
        id: i64,
        tenant_id: i64,
        code: Option<String>,
        name: Option<String>,
        address: Option<String>,
        is_active: Option<bool>,
    ) -> Result<WarehouseResponse, ApiError> {
        let warehouse = self
            .warehouse_repo
            .update(id, tenant_id, code, name, address, is_active)
            .await?;
        Ok(warehouse.into())
    }

    pub async fn delete_warehouse(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.warehouse_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    /// Restore a soft-deleted warehouse (admin only)
    pub async fn restore_warehouse(&self, id: i64, tenant_id: i64) -> Result<Warehouse, ApiError> {
        self.warehouse_repo.restore(id, tenant_id).await
    }

    /// List soft-deleted warehouses (admin only)
    pub async fn list_deleted_warehouses(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Warehouse>, ApiError> {
        self.warehouse_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a warehouse (admin only, after soft delete)
    pub async fn destroy_warehouse(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.warehouse_repo.destroy(id, tenant_id).await
    }

    // Stock level operations
    pub async fn get_stock_level(
        &self,
        warehouse_id: i64,
        product_id: i64,
    ) -> Result<StockLevelResponse, ApiError> {
        let level = self
            .stock_level_repo
            .find_by_warehouse_product(warehouse_id, product_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Stock level not found".to_string()))?;
        Ok(level.into())
    }

    pub async fn get_stock_by_product(
        &self,
        product_id: i64,
    ) -> Result<Vec<StockLevelResponse>, ApiError> {
        let levels = self.stock_level_repo.find_by_product(product_id).await?;
        Ok(levels.into_iter().map(|l| l.into()).collect())
    }

    pub async fn get_stock_by_warehouse(
        &self,
        warehouse_id: i64,
    ) -> Result<Vec<StockLevelResponse>, ApiError> {
        let levels = self
            .stock_level_repo
            .find_by_warehouse(warehouse_id)
            .await?;
        Ok(levels.into_iter().map(|l| l.into()).collect())
    }

    pub async fn get_stock_summary(
        &self,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<StockSummary, ApiError> {
        let levels = self.stock_level_repo.find_by_product(product_id).await?;

        let total_quantity: Decimal = levels.iter().map(|l| l.quantity).sum();
        let reserved_quantity: Decimal = levels.iter().map(|l| l.reserved_quantity).sum();

        let mut warehouses = Vec::new();
        for level in &levels {
            if let Ok(Some(warehouse)) = self
                .warehouse_repo
                .find_by_id(level.warehouse_id, tenant_id)
                .await
            {
                warehouses.push(crate::domain::stock::model::WarehouseStock {
                    warehouse_id: level.warehouse_id,
                    warehouse_name: warehouse.name,
                    quantity: level.quantity,
                    reserved_quantity: level.reserved_quantity,
                });
            }
        }

        Ok(StockSummary {
            product_id,
            total_quantity,
            reserved_quantity,
            available_quantity: total_quantity - reserved_quantity,
            warehouses,
        })
    }

    /// Soft delete a stock level
    pub async fn delete_stock_level(
        &self,
        warehouse_id: i64,
        product_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.stock_level_repo
            .soft_delete(warehouse_id, product_id, deleted_by)
            .await
    }

    /// Restore a soft-deleted stock level
    pub async fn restore_stock_level(
        &self,
        warehouse_id: i64,
        product_id: i64,
    ) -> Result<StockLevel, ApiError> {
        self.stock_level_repo
            .restore(warehouse_id, product_id)
            .await
    }

    /// List soft-deleted stock levels for a warehouse
    pub async fn list_deleted_stock_levels(
        &self,
        warehouse_id: i64,
    ) -> Result<Vec<StockLevel>, ApiError> {
        self.stock_level_repo.find_deleted(warehouse_id).await
    }

    /// Permanently delete a stock level
    pub async fn destroy_stock_level(
        &self,
        warehouse_id: i64,
        product_id: i64,
    ) -> Result<(), ApiError> {
        self.stock_level_repo
            .destroy(warehouse_id, product_id)
            .await
    }

    // Stock movement operations
    pub async fn create_stock_movement(
        &self,
        create: CreateStockMovement,
        tenant_id: i64,
    ) -> Result<StockMovementResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Verify warehouse exists (with tenant isolation)
        let _ = self
            .warehouse_repo
            .find_by_id(create.warehouse_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Warehouse {} not found", create.warehouse_id))
            })?;

        // Get current stock level
        let current_level = self
            .stock_level_repo
            .find_by_warehouse_product(create.warehouse_id, create.product_id)
            .await?;

        let new_quantity = match create.movement_type {
            // Stock in operations
            MovementType::Purchase | MovementType::Return | MovementType::ProductionIn => {
                current_level
                    .map(|l| l.quantity + create.quantity)
                    .unwrap_or(create.quantity)
            }
            // Stock out operations
            MovementType::Sale | MovementType::ProductionOut | MovementType::Waste => {
                let current = current_level.map(|l| l.quantity).unwrap_or(Decimal::ZERO);
                if create.quantity > current {
                    return Err(ApiError::BadRequest(format!(
                        "Insufficient stock. Available: {}, requested: {}",
                        current, create.quantity
                    )));
                }
                current - create.quantity
            }
            // Neutral operations (doesn't change quantity)
            MovementType::Adjustment | MovementType::Transfer => {
                current_level.map(|l| l.quantity).unwrap_or(Decimal::ZERO)
            }
        };

        // Update stock level
        self.stock_level_repo
            .update_quantity(create.warehouse_id, create.product_id, new_quantity)
            .await?;

        // Create movement record
        let movement = self.stock_movement_repo.create(create).await?;
        Ok(movement.into())
    }

    pub async fn get_stock_movements_by_product(
        &self,
        product_id: i64,
    ) -> Result<Vec<StockMovementResponse>, ApiError> {
        let movements = self.stock_movement_repo.find_by_product(product_id).await?;
        Ok(movements.into_iter().map(|m| m.into()).collect())
    }

    pub async fn get_stock_movements_by_warehouse(
        &self,
        warehouse_id: i64,
    ) -> Result<Vec<StockMovementResponse>, ApiError> {
        let movements = self
            .stock_movement_repo
            .find_by_warehouse(warehouse_id)
            .await?;
        Ok(movements.into_iter().map(|m| m.into()).collect())
    }

    pub async fn get_stock_movements_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<StockMovementResponse>, ApiError> {
        let movements = self.stock_movement_repo.find_by_tenant(tenant_id).await?;
        Ok(movements.into_iter().map(|m| m.into()).collect())
    }

    /// Soft delete a stock movement with tenant isolation
    pub async fn delete_stock_movement(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.stock_movement_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    /// Restore a soft-deleted stock movement with tenant isolation
    pub async fn restore_stock_movement(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<StockMovement, ApiError> {
        self.stock_movement_repo.restore(id, tenant_id).await
    }

    /// List soft-deleted stock movements for a warehouse
    pub async fn list_deleted_stock_movements(
        &self,
        warehouse_id: i64,
    ) -> Result<Vec<StockMovement>, ApiError> {
        self.stock_movement_repo.find_deleted(warehouse_id).await
    }

    /// Permanently delete a stock movement with tenant isolation
    pub async fn destroy_stock_movement(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.stock_movement_repo.destroy(id, tenant_id).await
    }

    // Reservation operations
    pub async fn reserve_stock(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<StockLevelResponse, ApiError> {
        let level = self
            .stock_level_repo
            .reserve_quantity(warehouse_id, product_id, quantity)
            .await?;
        Ok(level.into())
    }

    pub async fn release_stock(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<StockLevelResponse, ApiError> {
        let level = self
            .stock_level_repo
            .release_quantity(warehouse_id, product_id, quantity)
            .await?;
        Ok(level.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::stock::model::MovementType;
    use crate::domain::stock::repository::{
        InMemoryStockLevelRepository, InMemoryStockMovementRepository, InMemoryWarehouseRepository,
    };
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_service() -> StockService {
        let warehouse_repo = Arc::new(InMemoryWarehouseRepository::new()) as BoxWarehouseRepository;
        let stock_level_repo =
            Arc::new(InMemoryStockLevelRepository::new()) as BoxStockLevelRepository;
        let stock_movement_repo =
            Arc::new(InMemoryStockMovementRepository::new()) as BoxStockMovementRepository;
        StockService::new(warehouse_repo, stock_level_repo, stock_movement_repo)
    }

    #[tokio::test]
    async fn test_create_warehouse() {
        let service = create_service();

        let create = CreateWarehouse {
            tenant_id: 1,
            code: "WH001".to_string(),
            name: "Main Warehouse".to_string(),
            address: Some("Address".to_string()),
        };

        let result = service.create_warehouse(create).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Main Warehouse");
    }

    #[tokio::test]
    async fn test_stock_in_movement() {
        let service = create_service();

        // Create warehouse
        let warehouse = service
            .create_warehouse(CreateWarehouse {
                tenant_id: 1,
                code: "WH001".to_string(),
                name: "Main".to_string(),
                address: None,
            })
            .await
            .unwrap();

        // Stock in
        let movement = service
            .create_stock_movement(
                CreateStockMovement {
                    warehouse_id: warehouse.id,
                    product_id: 1,
                    movement_type: MovementType::Purchase,
                    quantity: dec!(100),
                    reference_type: Some("PO".to_string()),
                    reference_id: Some(1),
                    notes: None,
                    created_by: 1,
                },
                1,
            )
            .await
            .unwrap();

        assert_eq!(movement.quantity, dec!(100));

        // Check stock level
        let stock = service.get_stock_level(warehouse.id, 1).await.unwrap();
        assert_eq!(stock.quantity, dec!(100));
    }

    #[tokio::test]
    async fn test_stock_out_movement() {
        let service = create_service();

        let warehouse = service
            .create_warehouse(CreateWarehouse {
                tenant_id: 1,
                code: "WH001".to_string(),
                name: "Main".to_string(),
                address: None,
            })
            .await
            .unwrap();

        // Stock in first
        service
            .create_stock_movement(
                CreateStockMovement {
                    warehouse_id: warehouse.id,
                    product_id: 1,
                    movement_type: MovementType::Purchase,
                    quantity: dec!(100),
                    reference_type: None,
                    reference_id: None,
                    notes: None,
                    created_by: 1,
                },
                1,
            )
            .await
            .unwrap();

        // Stock out
        let result = service
            .create_stock_movement(
                CreateStockMovement {
                    warehouse_id: warehouse.id,
                    product_id: 1,
                    movement_type: MovementType::Sale,
                    quantity: dec!(30),
                    reference_type: Some("SO".to_string()),
                    reference_id: Some(1),
                    notes: None,
                    created_by: 1,
                },
                1,
            )
            .await;

        assert!(result.is_ok());
        let stock = service.get_stock_level(warehouse.id, 1).await.unwrap();
        assert_eq!(stock.quantity, dec!(70));
    }

    #[tokio::test]
    async fn test_insufficient_stock() {
        let service = create_service();

        let warehouse = service
            .create_warehouse(CreateWarehouse {
                tenant_id: 1,
                code: "WH001".to_string(),
                name: "Main".to_string(),
                address: None,
            })
            .await
            .unwrap();

        // Try to stock out more than available
        let result = service
            .create_stock_movement(
                CreateStockMovement {
                    warehouse_id: warehouse.id,
                    product_id: 1,
                    movement_type: MovementType::Sale,
                    quantity: dec!(100),
                    reference_type: None,
                    reference_id: None,
                    notes: None,
                    created_by: 1,
                },
                1,
            )
            .await;

        assert!(result.is_err());
    }
}
