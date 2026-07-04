//! Manufacturing service for business logic

use rust_decimal::Decimal;

use crate::common::pagination::PaginatedResult;
use crate::domain::manufacturing::model::{
    BillOfMaterials, BillOfMaterialsLine, CreateBillOfMaterials, CreateBillOfMaterialsLine,
    CreateRouting, CreateRoutingOperation, CreateWorkOrder, CreateWorkOrderMaterial,
    CreateWorkOrderOperation, Routing, RoutingOperation, WorkOrder, WorkOrderMaterial,
    WorkOrderOperation, WorkOrderStatus,
};
use crate::domain::manufacturing::repository::{
    BoxBillOfMaterialsRepository, BoxRoutingRepository, BoxWorkOrderRepository,
};
use crate::domain::product::repository::BoxProductRepository;
use crate::error::ApiError;

#[derive(Clone)]
pub struct ManufacturingService {
    work_order_repo: BoxWorkOrderRepository,
    bom_repo: BoxBillOfMaterialsRepository,
    routing_repo: BoxRoutingRepository,
    product_repo: BoxProductRepository,
}

impl ManufacturingService {
    pub fn new(
        work_order_repo: BoxWorkOrderRepository,
        bom_repo: BoxBillOfMaterialsRepository,
        routing_repo: BoxRoutingRepository,
        product_repo: BoxProductRepository,
    ) -> Self {
        Self {
            work_order_repo,
            bom_repo,
            routing_repo,
            product_repo,
        }
    }

    // Work Order methods
    #[tracing::instrument(skip(self))]
    pub async fn create_work_order(&self, create: CreateWorkOrder) -> Result<WorkOrder, ApiError> {
        // Parent-ownership precheck: product_id (required) + bom_id/routing_id
        // (optional) must belong to the caller's tenant, else a tenant-A caller
        // could open a work order against tenant-B's product/BOM/routing
        // (cross-tenant orphan write). The handler forces `create.tenant_id`
        // from the auth token, so it is the auth-derived tenant here. Also
        // yields a clean NotFound for a bogus id instead of an FK violation.
        let tenant_id = create.tenant_id;
        self.ensure_product_owned(create.product_id, tenant_id)
            .await?;
        if let Some(bom_id) = create.bom_id {
            self.bom_repo
                .find_by_id(bom_id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::NotFound("BOM not found".to_string()))?;
        }
        if let Some(routing_id) = create.routing_id {
            self.routing_repo
                .find_by_id(routing_id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::NotFound("Routing not found".to_string()))?;
        }
        self.work_order_repo.create(create).await
    }

    /// Tenant-scoped product-ownership precheck. Returns `NotFound` if the
    /// product does not belong to the caller's tenant (or does not exist).
    async fn ensure_product_owned(&self, product_id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.product_repo
            .find_by_id(product_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Product not found".to_string()))?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_work_order(&self, id: i64, tenant_id: i64) -> Result<WorkOrder, ApiError> {
        self.work_order_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Work order not found".to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_work_orders_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<WorkOrder>, ApiError> {
        self.work_order_repo.find_by_tenant(tenant_id).await
    }

    /// Get work orders by tenant with pagination
    #[tracing::instrument(skip(self))]
    pub async fn get_work_orders_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<WorkOrder>, ApiError> {
        self.work_order_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_work_order_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: WorkOrderStatus,
    ) -> Result<WorkOrder, ApiError> {
        self.work_order_repo
            .update_status(id, tenant_id, status)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn add_work_order_operation(
        &self,
        create: CreateWorkOrderOperation,
        tenant_id: i64,
    ) -> Result<WorkOrderOperation, ApiError> {
        // Parent-ownership precheck: the work order must belong to the caller's
        // tenant, else a tenant-A caller could attach an operation to tenant-B's
        // work order (cross-tenant orphan write). Also yields a clean NotFound
        // for a bogus work_order_id instead of an FK violation.
        self.work_order_repo
            .find_by_id(create.work_order_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Work order not found".to_string()))?;
        self.work_order_repo.add_operation(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_work_order_operations(
        &self,
        work_order_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<WorkOrderOperation>, ApiError> {
        self.work_order_repo
            .get_operations(work_order_id, tenant_id)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn add_work_order_material(
        &self,
        create: CreateWorkOrderMaterial,
        tenant_id: i64,
    ) -> Result<WorkOrderMaterial, ApiError> {
        // Parent-ownership precheck (see add_work_order_operation).
        self.work_order_repo
            .find_by_id(create.work_order_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Work order not found".to_string()))?;
        // Parent-ownership precheck: the material's product must belong to the
        // caller's tenant, else a tenant-A caller could attach tenant-B's
        // product as a work-order material (cross-tenant orphan write).
        self.ensure_product_owned(create.product_id, tenant_id)
            .await?;
        self.work_order_repo.add_material(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_work_order_materials(
        &self,
        work_order_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<WorkOrderMaterial>, ApiError> {
        self.work_order_repo
            .get_materials(work_order_id, tenant_id)
            .await
    }

    // BOM methods
    #[tracing::instrument(skip(self))]
    pub async fn create_bom(
        &self,
        create: CreateBillOfMaterials,
    ) -> Result<BillOfMaterials, ApiError> {
        // Parent-ownership precheck: the BOM's product must belong to the
        // caller's tenant, else a tenant-A caller could define a BOM against
        // tenant-B's product (cross-tenant orphan write). The handler forces
        // `create.tenant_id` from the auth token.
        self.ensure_product_owned(create.product_id, create.tenant_id)
            .await?;
        self.bom_repo.create(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_bom(&self, id: i64, tenant_id: i64) -> Result<BillOfMaterials, ApiError> {
        self.bom_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("BOM not found".to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_boms_by_product(
        &self,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<BillOfMaterials>, ApiError> {
        self.bom_repo.find_by_product(product_id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_primary_bom_by_product(
        &self,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<Option<BillOfMaterials>, ApiError> {
        self.bom_repo
            .find_primary_by_product(product_id, tenant_id)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn add_bom_line(
        &self,
        create: CreateBillOfMaterialsLine,
        tenant_id: i64,
    ) -> Result<BillOfMaterialsLine, ApiError> {
        // Parent-ownership precheck: the BOM must belong to the caller's tenant,
        // else a tenant-A caller could attach a line to tenant-B's BOM.
        self.bom_repo
            .find_by_id(create.bom_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("BOM not found".to_string()))?;
        // Parent-ownership precheck: the line's component product must belong
        // to the caller's tenant, else a tenant-A caller could reference
        // tenant-B's product as a BOM component (cross-tenant orphan write).
        self.ensure_product_owned(create.component_product_id, tenant_id)
            .await?;
        self.bom_repo.add_line(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_bom_lines(
        &self,
        bom_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<BillOfMaterialsLine>, ApiError> {
        self.bom_repo.get_lines(bom_id, tenant_id).await
    }

    // Routing methods
    #[tracing::instrument(skip(self))]
    pub async fn create_routing(&self, create: CreateRouting) -> Result<Routing, ApiError> {
        // Parent-ownership precheck: the routing's product must belong to the
        // caller's tenant, else a tenant-A caller could define a routing
        // against tenant-B's product (cross-tenant orphan write). The handler
        // forces `create.tenant_id` from the auth token.
        self.ensure_product_owned(create.product_id, create.tenant_id)
            .await?;
        self.routing_repo.create(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_routing(&self, id: i64, tenant_id: i64) -> Result<Routing, ApiError> {
        self.routing_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Routing not found".to_string()))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_routings_by_product(
        &self,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Routing>, ApiError> {
        self.routing_repo
            .find_by_product(product_id, tenant_id)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_primary_routing_by_product(
        &self,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<Option<Routing>, ApiError> {
        self.routing_repo
            .find_primary_by_product(product_id, tenant_id)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn add_routing_operation(
        &self,
        create: CreateRoutingOperation,
        tenant_id: i64,
    ) -> Result<RoutingOperation, ApiError> {
        // Parent-ownership precheck: the routing must belong to the caller's
        // tenant, else a tenant-A caller could attach an operation to tenant-B's
        // routing.
        self.routing_repo
            .find_by_id(create.routing_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Routing not found".to_string()))?;
        self.routing_repo.add_operation(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_routing_operations(
        &self,
        routing_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<RoutingOperation>, ApiError> {
        self.routing_repo
            .get_operations(routing_id, tenant_id)
            .await
    }

    // Calculate material requirements from BOM
    #[tracing::instrument(skip(self))]
    pub async fn calculate_material_requirements(
        &self,
        product_id: i64,
        quantity: Decimal,
        tenant_id: i64,
    ) -> Result<Vec<(i64, Decimal)>, ApiError> {
        let bom = self
            .bom_repo
            .find_primary_by_product(product_id, tenant_id)
            .await?;
        match bom {
            Some(bom) => {
                let lines = self.bom_repo.get_lines(bom.id, tenant_id).await?;
                let mut requirements = Vec::with_capacity(lines.len());
                for line in lines {
                    let scrap_factor =
                        Decimal::ONE + (line.scrap_percentage / Decimal::ONE_HUNDRED);
                    let required_qty = quantity * line.quantity * scrap_factor;
                    requirements.push((line.component_product_id, required_qty));
                }
                Ok(requirements)
            }
            None => Ok(Vec::new()),
        }
    }

    // Calculate production time from routing
    #[tracing::instrument(skip(self))]
    pub async fn calculate_production_time(
        &self,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<Decimal, ApiError> {
        let routing = self
            .routing_repo
            .find_primary_by_product(product_id, tenant_id)
            .await?;
        match routing {
            Some(r) => {
                let ops = self.routing_repo.get_operations(r.id, tenant_id).await?;
                let total_time: Decimal = ops.iter().map(|op| op.setup_hours + op.run_hours).sum();
                Ok(total_time)
            }
            None => Ok(Decimal::ZERO),
        }
    }

    // Work Order soft-delete methods
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_work_order(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.work_order_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_work_order(&self, id: i64, tenant_id: i64) -> Result<WorkOrder, ApiError> {
        self.work_order_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_work_orders(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<WorkOrder>, ApiError> {
        self.work_order_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_work_order(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.work_order_repo.destroy(id, tenant_id).await
    }

    // BOM soft-delete methods
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_bom(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.bom_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_bom(&self, id: i64, tenant_id: i64) -> Result<BillOfMaterials, ApiError> {
        self.bom_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_boms(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<BillOfMaterials>, ApiError> {
        self.bom_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_bom(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.bom_repo.destroy(id, tenant_id).await
    }

    // Routing soft-delete methods
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_routing(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.routing_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_routing(&self, id: i64, tenant_id: i64) -> Result<Routing, ApiError> {
        self.routing_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_routings(&self, tenant_id: i64) -> Result<Vec<Routing>, ApiError> {
        self.routing_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_routing(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.routing_repo.destroy(id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::manufacturing::model::WorkOrderPriority;
    use crate::domain::manufacturing::repository::{
        InMemoryBillOfMaterialsRepository, InMemoryRoutingRepository, InMemoryWorkOrderRepository,
    };
    use crate::domain::product::model::CreateProduct;
    use crate::domain::product::repository::{BoxProductRepository, InMemoryProductRepository};
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_service() -> (ManufacturingService, BoxProductRepository) {
        let wo_repo = Arc::new(InMemoryWorkOrderRepository::new()) as BoxWorkOrderRepository;
        let bom_repo =
            Arc::new(InMemoryBillOfMaterialsRepository::new()) as BoxBillOfMaterialsRepository;
        let routing_repo = Arc::new(InMemoryRoutingRepository::new()) as BoxRoutingRepository;
        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        let service =
            ManufacturingService::new(wo_repo, bom_repo, routing_repo, product_repo.clone());
        (service, product_repo)
    }

    /// Seed a product on `tenant_id` and return its id. The InMemory repo uses
    /// a GLOBAL auto-id counter, so the first seeded product is id 1, the next
    /// id 2, etc. — matching the cross-tenant IDOR negative tests (a product
    /// seeded on tenant 1 has an id that does not exist on tenant 2).
    async fn seed_product(repo: &BoxProductRepository, tenant_id: i64) -> i64 {
        let product = repo
            .create(CreateProduct {
                tenant_id,
                company_id: 1,
                code: format!("PROD-{}-{}", tenant_id, uuid::Uuid::new_v4()),
                name: format!("Product for tenant {}", tenant_id),
                description: None,
                category_id: None,
                unit_id: None,
                barcode: None,
                purchase_price: Decimal::ZERO,
                sale_price: Decimal::ZERO,
                tax_rate: Decimal::ZERO,
            })
            .await
            .expect("seed product");
        product.id
    }

    #[tokio::test]
    async fn test_create_work_order() {
        let (service, products) = create_service();
        let product_id = seed_product(&products, 1).await;
        let create = CreateWorkOrder {
            tenant_id: 1,
            name: "WO-001".to_string(),
            product_id,
            quantity: dec!(100),
            bom_id: None,
            routing_id: None,
            priority: WorkOrderPriority::Normal,
            planned_start: Some(chrono::Utc::now()),
            planned_end: None,
        };
        let result = service.create_work_order(create).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, WorkOrderStatus::Draft);
    }

    #[tokio::test]
    async fn test_create_work_order_rejects_foreign_product() {
        let (service, products) = create_service();
        // product exists only on tenant 1; a tenant-2 caller cannot open a
        // work order against it (cross-tenant orphan write).
        let product_id = seed_product(&products, 1).await;
        let create = CreateWorkOrder {
            tenant_id: 2,
            name: "WO-foreign".to_string(),
            product_id,
            quantity: dec!(100),
            bom_id: None,
            routing_id: None,
            priority: WorkOrderPriority::Normal,
            planned_start: None,
            planned_end: None,
        };
        let result = service.create_work_order(create).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_create_bom() {
        let (service, products) = create_service();
        let product_id = seed_product(&products, 1).await;
        let create = CreateBillOfMaterials {
            tenant_id: 1,
            product_id,
            version: "1.0".to_string(),
            is_active: true,
            is_primary: true,
            valid_from: Some(chrono::Utc::now()),
            valid_to: None,
        };
        let result = service.create_bom(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_bom_rejects_foreign_product() {
        let (service, products) = create_service();
        let product_id = seed_product(&products, 1).await;
        let result = service
            .create_bom(CreateBillOfMaterials {
                tenant_id: 2,
                product_id,
                version: "1.0".to_string(),
                is_active: true,
                is_primary: true,
                valid_from: None,
                valid_to: None,
            })
            .await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_add_bom_line() {
        let (service, products) = create_service();
        let bom_product = seed_product(&products, 1).await;
        let component_product = seed_product(&products, 1).await;
        let bom = service
            .create_bom(CreateBillOfMaterials {
                tenant_id: 1,
                product_id: bom_product,
                version: "1.0".to_string(),
                is_active: true,
                is_primary: true,
                valid_from: Some(chrono::Utc::now()),
                valid_to: None,
            })
            .await
            .unwrap();

        let line = service
            .add_bom_line(
                CreateBillOfMaterialsLine {
                    tenant_id: 1,
                    bom_id: bom.id,
                    component_product_id: component_product,
                    quantity: dec!(5),
                    unit_id: Some(1),
                    scrap_percentage: dec!(5),
                    is_optional: false,
                    notes: None,
                },
                1,
            )
            .await
            .unwrap();

        assert_eq!(line.quantity, dec!(5));
    }

    #[tokio::test]
    async fn test_add_bom_line_rejects_foreign_component() {
        let (service, products) = create_service();
        // Tenant-2 BOM (product seeded on tenant 2) ...
        let t2_product = seed_product(&products, 2).await;
        let bom = service
            .create_bom(CreateBillOfMaterials {
                tenant_id: 2,
                product_id: t2_product,
                version: "1.0".to_string(),
                is_active: true,
                is_primary: true,
                valid_from: None,
                valid_to: None,
            })
            .await
            .unwrap();
        // ... referencing a tenant-1 component product (cross-tenant orphan).
        let t1_component = seed_product(&products, 1).await;
        let result = service
            .add_bom_line(
                CreateBillOfMaterialsLine {
                    tenant_id: 2,
                    bom_id: bom.id,
                    component_product_id: t1_component,
                    quantity: dec!(5),
                    unit_id: Some(1),
                    scrap_percentage: dec!(5),
                    is_optional: false,
                    notes: None,
                },
                2,
            )
            .await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_create_routing() {
        let (service, products) = create_service();
        let product_id = seed_product(&products, 1).await;
        let create = CreateRouting {
            tenant_id: 1,
            product_id,
            version: "1.0".to_string(),
            is_active: true,
            is_primary: true,
        };
        let result = service.create_routing(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_routing_rejects_foreign_product() {
        let (service, products) = create_service();
        let product_id = seed_product(&products, 1).await;
        let result = service
            .create_routing(CreateRouting {
                tenant_id: 2,
                product_id,
                version: "1.0".to_string(),
                is_active: true,
                is_primary: true,
            })
            .await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_material_requirements_calculation() {
        let (service, products) = create_service();
        let bom_product = seed_product(&products, 1).await;
        let component_product = seed_product(&products, 1).await;

        // Create BOM
        let bom = service
            .create_bom(CreateBillOfMaterials {
                tenant_id: 1,
                product_id: bom_product,
                version: "1.0".to_string(),
                is_active: true,
                is_primary: true,
                valid_from: Some(chrono::Utc::now()),
                valid_to: None,
            })
            .await
            .unwrap();

        // Add BOM lines
        service
            .add_bom_line(
                CreateBillOfMaterialsLine {
                    tenant_id: 1,
                    bom_id: bom.id,
                    component_product_id: component_product,
                    quantity: dec!(2),
                    unit_id: Some(1),
                    scrap_percentage: dec!(10),
                    is_optional: false,
                    notes: None,
                },
                1,
            )
            .await
            .unwrap();

        // Calculate for quantity of 10
        let requirements = service
            .calculate_material_requirements(bom_product, dec!(10), 1)
            .await
            .unwrap();
        assert_eq!(requirements.len(), 1);
        // 10 * 2 * 1.1 = 22.0 (with 10% scrap)
        assert_eq!(requirements[0].1, dec!(22));
    }

    #[tokio::test]
    async fn test_production_time_calculation() {
        let (service, products) = create_service();
        let product_id = seed_product(&products, 1).await;

        // Create routing
        let routing = service
            .create_routing(CreateRouting {
                tenant_id: 1,
                product_id,
                version: "1.0".to_string(),
                is_active: true,
                is_primary: true,
            })
            .await
            .unwrap();

        // Add operations
        service
            .add_routing_operation(
                CreateRoutingOperation {
                    tenant_id: 1,
                    routing_id: routing.id,
                    sequence: 1,
                    operation_name: "Setup".to_string(),
                    work_center_id: Some(1),
                    setup_hours: dec!(1),
                    run_hours: Decimal::ZERO,
                    description: None,
                },
                1,
            )
            .await
            .unwrap();

        service
            .add_routing_operation(
                CreateRoutingOperation {
                    tenant_id: 1,
                    routing_id: routing.id,
                    sequence: 2,
                    operation_name: "Assembly".to_string(),
                    work_center_id: Some(1),
                    setup_hours: Decimal::ZERO,
                    run_hours: dec!(5),
                    description: None,
                },
                1,
            )
            .await
            .unwrap();

        let time = service.calculate_production_time(1, 1).await.unwrap();
        assert_eq!(time, dec!(6)); // 1 + 5
    }
}
