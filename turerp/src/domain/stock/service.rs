//! Stock service for business logic
use rust_decimal::Decimal;

use crate::common::pagination::PaginatedResult;
use crate::domain::company::service::ensure_company_owned;
use crate::domain::company::BoxCompanyRepository;
use crate::domain::product::repository::BoxProductRepository;
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
    product_repo: BoxProductRepository,
    company_repo: BoxCompanyRepository,
}

impl StockService {
    pub fn new(
        warehouse_repo: BoxWarehouseRepository,
        stock_level_repo: BoxStockLevelRepository,
        stock_movement_repo: BoxStockMovementRepository,
        product_repo: BoxProductRepository,
        company_repo: BoxCompanyRepository,
    ) -> Self {
        Self {
            warehouse_repo,
            stock_level_repo,
            stock_movement_repo,
            product_repo,
            company_repo,
        }
    }

    // Warehouse operations
    #[tracing::instrument(skip(self))]
    pub async fn create_warehouse(
        &self,
        create: CreateWarehouse,
    ) -> Result<WarehouseResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        // Parent-ownership precheck: body company_id must belong to the caller's
        // tenant (legacy `1` sentinel skipped for backward compat).
        ensure_company_owned(&self.company_repo, create.company_id, create.tenant_id).await?;
        let warehouse = self.warehouse_repo.create(create).await?;
        Ok(warehouse.into())
    }

    #[tracing::instrument(skip(self))]
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

    #[tracing::instrument(skip(self))]
    pub async fn get_warehouses_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<WarehouseResponse>, ApiError> {
        let warehouses = self.warehouse_repo.find_by_tenant(tenant_id).await?;
        Ok(warehouses.into_iter().map(|w| w.into()).collect())
    }

    #[tracing::instrument(skip(self))]
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

    #[tracing::instrument(skip(self))]
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

    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
    pub async fn restore_warehouse(&self, id: i64, tenant_id: i64) -> Result<Warehouse, ApiError> {
        self.warehouse_repo.restore(id, tenant_id).await
    }

    /// List soft-deleted warehouses (admin only)
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_warehouses(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Warehouse>, ApiError> {
        self.warehouse_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a warehouse (admin only, after soft delete)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_warehouse(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.warehouse_repo.destroy(id, tenant_id).await
    }

    // Stock level operations
    #[tracing::instrument(skip(self))]
    pub async fn get_stock_level(
        &self,
        warehouse_id: i64,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<StockLevelResponse, ApiError> {
        let level = self
            .stock_level_repo
            .find_by_warehouse_product(tenant_id, warehouse_id, product_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Stock level not found".to_string()))?;
        Ok(level.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_stock_by_product(
        &self,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<StockLevelResponse>, ApiError> {
        let levels = self
            .stock_level_repo
            .find_by_product(product_id, tenant_id)
            .await?;
        Ok(levels.into_iter().map(|l| l.into()).collect())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_stock_by_warehouse(
        &self,
        warehouse_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<StockLevelResponse>, ApiError> {
        let levels = self
            .stock_level_repo
            .find_by_warehouse(warehouse_id, tenant_id)
            .await?;
        Ok(levels.into_iter().map(|l| l.into()).collect())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_stock_summary(
        &self,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<StockSummary, ApiError> {
        let levels = self
            .stock_level_repo
            .find_by_product(product_id, tenant_id)
            .await?;

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
    #[tracing::instrument(skip(self))]
    pub async fn delete_stock_level(
        &self,
        warehouse_id: i64,
        product_id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.stock_level_repo
            .soft_delete(tenant_id, warehouse_id, product_id, deleted_by)
            .await
    }

    /// Restore a soft-deleted stock level
    #[tracing::instrument(skip(self))]
    pub async fn restore_stock_level(
        &self,
        warehouse_id: i64,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<StockLevel, ApiError> {
        self.stock_level_repo
            .restore(tenant_id, warehouse_id, product_id)
            .await
    }

    /// List soft-deleted stock levels for a warehouse
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_stock_levels(
        &self,
        warehouse_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<StockLevel>, ApiError> {
        self.stock_level_repo
            .find_deleted(warehouse_id, tenant_id)
            .await
    }

    /// Permanently delete a stock level
    #[tracing::instrument(skip(self))]
    pub async fn destroy_stock_level(
        &self,
        warehouse_id: i64,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<(), ApiError> {
        self.stock_level_repo
            .destroy(tenant_id, warehouse_id, product_id)
            .await
    }

    // Stock movement operations
    #[tracing::instrument(skip(self))]
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

        // Parent-ownership precheck: the body-controlled `product_id` must belong
        // to the caller's tenant before we touch stock levels. Without this, the
        // `find_by_warehouse_product` lookup below returns None for a foreign
        // product without rejecting, so a Purchase/Return/ProductionIn movement
        // would orphan a `stock_level` row onto a foreign `product_id` (and a
        // Sale/ProductionOut/Waste movement would silently no-op on a phantom
        // level). `tenant_id` is the auth-overwritten service arg (set into
        // create.tenant_id below). Mirrors the established precheck pattern.
        self.product_repo
            .find_by_id(create.product_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Product {} not found", create.product_id))
            })?;

        // Parent-ownership precheck: body company_id must belong to the caller's
        // tenant (legacy `1` sentinel skipped for backward compat). `tenant_id`
        // is the auth-overwritten service param (same as the product precheck).
        ensure_company_owned(&self.company_repo, create.company_id, tenant_id).await?;

        // Get current stock level
        let current_level = self
            .stock_level_repo
            .find_by_warehouse_product(tenant_id, create.warehouse_id, create.product_id)
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
            .update_quantity(
                tenant_id,
                create.warehouse_id,
                create.product_id,
                new_quantity,
            )
            .await?;

        // Set tenant_id and create movement record
        let mut create = create;
        create.tenant_id = tenant_id;
        let movement = self.stock_movement_repo.create(create).await?;
        Ok(movement.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_stock_movements_by_product(
        &self,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<StockMovementResponse>, ApiError> {
        let movements = self
            .stock_movement_repo
            .find_by_product(product_id, tenant_id)
            .await?;
        Ok(movements.into_iter().map(|m| m.into()).collect())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_stock_movements_by_warehouse(
        &self,
        warehouse_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<StockMovementResponse>, ApiError> {
        let movements = self
            .stock_movement_repo
            .find_by_warehouse(warehouse_id, tenant_id)
            .await?;
        Ok(movements.into_iter().map(|m| m.into()).collect())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_stock_movements_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<StockMovementResponse>, ApiError> {
        let movements = self.stock_movement_repo.find_by_tenant(tenant_id).await?;
        Ok(movements.into_iter().map(|m| m.into()).collect())
    }

    /// Soft delete a stock movement with tenant isolation
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
    pub async fn restore_stock_movement(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<StockMovement, ApiError> {
        self.stock_movement_repo.restore(id, tenant_id).await
    }

    /// List soft-deleted stock movements for a warehouse
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_stock_movements(
        &self,
        warehouse_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<StockMovement>, ApiError> {
        self.stock_movement_repo
            .find_deleted(warehouse_id, tenant_id)
            .await
    }

    /// Permanently delete a stock movement with tenant isolation
    #[tracing::instrument(skip(self))]
    pub async fn destroy_stock_movement(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.stock_movement_repo.destroy(id, tenant_id).await
    }

    // Reservation operations
    #[tracing::instrument(skip(self))]
    pub async fn reserve_stock(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
        tenant_id: i64,
    ) -> Result<StockLevelResponse, ApiError> {
        let level = self
            .stock_level_repo
            .reserve_quantity(tenant_id, warehouse_id, product_id, quantity)
            .await?;
        Ok(level.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn release_stock(
        &self,
        warehouse_id: i64,
        product_id: i64,
        quantity: Decimal,
        tenant_id: i64,
    ) -> Result<StockLevelResponse, ApiError> {
        let level = self
            .stock_level_repo
            .release_quantity(tenant_id, warehouse_id, product_id, quantity)
            .await?;
        Ok(level.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::company::repository::InMemoryCompanyRepository;
    use crate::domain::company::service::LEGACY_COMPANY_ID;
    use crate::domain::company::CreateCompany;
    use crate::domain::product::model::CreateProduct;
    use crate::domain::product::repository::InMemoryProductRepository;
    use crate::domain::stock::model::MovementType;
    use crate::domain::stock::repository::{
        InMemoryStockLevelRepository, InMemoryStockMovementRepository, InMemoryWarehouseRepository,
    };
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    async fn create_service() -> StockService {
        let warehouse_repo = Arc::new(InMemoryWarehouseRepository::new()) as BoxWarehouseRepository;
        let stock_level_repo =
            Arc::new(InMemoryStockLevelRepository::new()) as BoxStockLevelRepository;
        let stock_movement_repo =
            Arc::new(InMemoryStockMovementRepository::new()) as BoxStockMovementRepository;

        // Seed the parent product the create-stock-movement precheck validates
        // against. InMemory repo create() auto-assigns ids starting at 1,
        // matching the product_id = 1 used by the happy-path tests below. A
        // second product is seeded on tenant 2 (id 2) so the cross-tenant IDOR
        // rejection test can reference a foreign product.
        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        product_repo
            .create(CreateProduct {
                tenant_id: 1,
                code: "P1".to_string(),
                name: "Test Product T1".to_string(),
                purchase_price: Decimal::ZERO,
                sale_price: Decimal::ZERO,
                tax_rate: Decimal::ZERO,
                ..Default::default()
            })
            .await
            .expect("seed product t1");
        product_repo
            .create(CreateProduct {
                tenant_id: 2,
                code: "P2".to_string(),
                name: "Test Product T2".to_string(),
                purchase_price: Decimal::ZERO,
                sale_price: Decimal::ZERO,
                tax_rate: Decimal::ZERO,
                ..Default::default()
            })
            .await
            .expect("seed product t2");

        // Seed a company per tenant so the InMemory auto-id counter yields id=1
        // for tenant-1 (the LEGACY_COMPANY_ID sentinel, skipped by the precheck)
        // and id=2 for tenant-2 (a non-sentinel foreign company the reject tests
        // target).
        let company_repo = Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository;
        for tenant in [1, 2] {
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
                .expect("seed company");
        }

        StockService::new(
            warehouse_repo,
            stock_level_repo,
            stock_movement_repo,
            product_repo,
            company_repo,
        )
    }

    /// Returns the tenant-2 company id (a non-sentinel foreign company) for the
    /// reject tests, guarding that the seeded id is not the LEGACY sentinel.
    async fn foreign_company_id(service: &StockService) -> i64 {
        let id = service
            .company_repo
            .find_by_tenant(2)
            .await
            .expect("list tenant-2 companies")
            .into_iter()
            .map(|c| c.id)
            .next()
            .expect("tenant-2 company seeded");
        assert_ne!(id, LEGACY_COMPANY_ID);
        id
    }

    #[tokio::test]
    async fn test_create_warehouse() {
        let service = create_service().await;

        let create = CreateWarehouse {
            tenant_id: 1,
            company_id: 1,
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
        let service = create_service().await;

        // Create warehouse
        let warehouse = service
            .create_warehouse(CreateWarehouse {
                tenant_id: 1,
                company_id: 1,
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
                    tenant_id: 1,
                    company_id: 1,
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
        let stock = service.get_stock_level(warehouse.id, 1, 1).await.unwrap();
        assert_eq!(stock.quantity, dec!(100));
    }

    #[tokio::test]
    async fn test_stock_out_movement() {
        let service = create_service().await;

        let warehouse = service
            .create_warehouse(CreateWarehouse {
                tenant_id: 1,
                company_id: 1,
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
                    tenant_id: 1,
                    company_id: 1,
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
                    tenant_id: 1,
                    company_id: 1,
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
        let stock = service.get_stock_level(warehouse.id, 1, 1).await.unwrap();
        assert_eq!(stock.quantity, dec!(70));
    }

    #[tokio::test]
    async fn test_insufficient_stock() {
        let service = create_service().await;

        let warehouse = service
            .create_warehouse(CreateWarehouse {
                tenant_id: 1,
                company_id: 1,
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
                    tenant_id: 1,
                    company_id: 1,
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

    #[tokio::test]
    async fn test_create_stock_movement_rejects_foreign_product() {
        // Tenant 1 owns warehouse id 1 but references product 2, which belongs to
        // tenant 2 -> the product-ownership precheck returns NotFound before any
        // stock_level row is touched (cross-tenant IDOR / orphan level prevented).
        let service = create_service().await;
        let warehouse = service
            .create_warehouse(CreateWarehouse {
                tenant_id: 1,
                company_id: 1,
                code: "WH001".to_string(),
                name: "Main".to_string(),
                address: None,
            })
            .await
            .unwrap();

        let err = service
            .create_stock_movement(
                CreateStockMovement {
                    tenant_id: 1,
                    company_id: 1,
                    warehouse_id: warehouse.id,
                    product_id: 2,
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
            .unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)), "got {:?}", err);
    }

    /// Rejects a warehouse stamped onto a foreign-tenant company.
    #[tokio::test]
    async fn test_create_warehouse_rejects_foreign_company() {
        let service = create_service().await;
        let foreign = foreign_company_id(&service).await;
        let create = CreateWarehouse {
            tenant_id: 1,
            company_id: foreign,
            code: "WH-FOR".to_string(),
            name: "Foreign-stamped warehouse".to_string(),
            address: None,
        };
        let result = service.create_warehouse(create).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "expected NotFound for foreign company_id, got {:?}",
            result
        );
    }

    /// Rejects a stock movement stamped onto a foreign-tenant company. Uses an
    /// own-tenant warehouse + product (id=1) so the warehouse + product
    /// prechecks pass; the foreign company_id is the sole rejection source.
    #[tokio::test]
    async fn test_create_stock_movement_rejects_foreign_company() {
        let service = create_service().await;
        let foreign = foreign_company_id(&service).await;
        let warehouse = service
            .create_warehouse(CreateWarehouse {
                tenant_id: 1,
                company_id: 1,
                code: "WH001".to_string(),
                name: "Main".to_string(),
                address: None,
            })
            .await
            .unwrap();
        let err = service
            .create_stock_movement(
                CreateStockMovement {
                    tenant_id: 1,
                    company_id: foreign,
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
            .unwrap_err();
        assert!(
            matches!(err, ApiError::NotFound(_)),
            "expected NotFound for foreign company_id, got {:?}",
            err
        );
    }
}
