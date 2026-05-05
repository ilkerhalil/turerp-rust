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
use crate::error::ApiError;

#[derive(Clone)]
pub struct ManufacturingService {
    work_order_repo: BoxWorkOrderRepository,
    bom_repo: BoxBillOfMaterialsRepository,
    routing_repo: BoxRoutingRepository,
}

impl ManufacturingService {
    pub fn new(
        work_order_repo: BoxWorkOrderRepository,
        bom_repo: BoxBillOfMaterialsRepository,
        routing_repo: BoxRoutingRepository,
    ) -> Self {
        Self {
            work_order_repo,
            bom_repo,
            routing_repo,
        }
    }

    // Work Order methods
    pub async fn create_work_order(&self, create: CreateWorkOrder) -> Result<WorkOrder, ApiError> {
        self.work_order_repo.create(create).await
    }

    pub async fn get_work_order(&self, id: i64, tenant_id: i64) -> Result<WorkOrder, ApiError> {
        self.work_order_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Work order not found".to_string()))
    }

    pub async fn get_work_orders_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<WorkOrder>, ApiError> {
        self.work_order_repo.find_by_tenant(tenant_id).await
    }

    /// Get work orders by tenant with pagination
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

    pub async fn get_work_orders_by_product(
        &self,
        product_id: i64,
    ) -> Result<Vec<WorkOrder>, ApiError> {
        self.work_order_repo.find_by_product(product_id).await
    }

    pub async fn update_work_order_status(
        &self,
        id: i64,
        status: WorkOrderStatus,
    ) -> Result<WorkOrder, ApiError> {
        self.work_order_repo.update_status(id, status).await
    }

    pub async fn add_work_order_operation(
        &self,
        create: CreateWorkOrderOperation,
    ) -> Result<WorkOrderOperation, ApiError> {
        self.work_order_repo.add_operation(create).await
    }

    pub async fn get_work_order_operations(
        &self,
        work_order_id: i64,
    ) -> Result<Vec<WorkOrderOperation>, ApiError> {
        self.work_order_repo.get_operations(work_order_id).await
    }

    pub async fn add_work_order_material(
        &self,
        create: CreateWorkOrderMaterial,
    ) -> Result<WorkOrderMaterial, ApiError> {
        self.work_order_repo.add_material(create).await
    }

    pub async fn get_work_order_materials(
        &self,
        work_order_id: i64,
    ) -> Result<Vec<WorkOrderMaterial>, ApiError> {
        self.work_order_repo.get_materials(work_order_id).await
    }

    // BOM methods
    pub async fn create_bom(
        &self,
        create: CreateBillOfMaterials,
    ) -> Result<BillOfMaterials, ApiError> {
        self.bom_repo.create(create).await
    }

    pub async fn get_bom(&self, id: i64, tenant_id: i64) -> Result<BillOfMaterials, ApiError> {
        self.bom_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("BOM not found".to_string()))
    }

    pub async fn get_boms_by_product(
        &self,
        product_id: i64,
    ) -> Result<Vec<BillOfMaterials>, ApiError> {
        self.bom_repo.find_by_product(product_id).await
    }

    pub async fn get_primary_bom_by_product(
        &self,
        product_id: i64,
    ) -> Result<Option<BillOfMaterials>, ApiError> {
        self.bom_repo.find_primary_by_product(product_id).await
    }

    pub async fn add_bom_line(
        &self,
        create: CreateBillOfMaterialsLine,
    ) -> Result<BillOfMaterialsLine, ApiError> {
        self.bom_repo.add_line(create).await
    }

    pub async fn get_bom_lines(&self, bom_id: i64) -> Result<Vec<BillOfMaterialsLine>, ApiError> {
        self.bom_repo.get_lines(bom_id).await
    }

    // Routing methods
    pub async fn create_routing(&self, create: CreateRouting) -> Result<Routing, ApiError> {
        self.routing_repo.create(create).await
    }

    pub async fn get_routing(&self, id: i64, tenant_id: i64) -> Result<Routing, ApiError> {
        self.routing_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Routing not found".to_string()))
    }

    pub async fn get_routings_by_product(&self, product_id: i64) -> Result<Vec<Routing>, ApiError> {
        self.routing_repo.find_by_product(product_id).await
    }

    pub async fn get_primary_routing_by_product(
        &self,
        product_id: i64,
    ) -> Result<Option<Routing>, ApiError> {
        self.routing_repo.find_primary_by_product(product_id).await
    }

    pub async fn add_routing_operation(
        &self,
        create: CreateRoutingOperation,
    ) -> Result<RoutingOperation, ApiError> {
        self.routing_repo.add_operation(create).await
    }

    pub async fn get_routing_operations(
        &self,
        routing_id: i64,
    ) -> Result<Vec<RoutingOperation>, ApiError> {
        self.routing_repo.get_operations(routing_id).await
    }

    // Calculate material requirements from BOM
    pub async fn calculate_material_requirements(
        &self,
        product_id: i64,
        quantity: Decimal,
    ) -> Result<Vec<(i64, Decimal)>, ApiError> {
        let bom = self.bom_repo.find_primary_by_product(product_id).await?;
        match bom {
            Some(bom) => {
                let lines = self.bom_repo.get_lines(bom.id).await?;
                let mut requirements = Vec::new();
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
    pub async fn calculate_production_time(&self, product_id: i64) -> Result<Decimal, ApiError> {
        let routing = self
            .routing_repo
            .find_primary_by_product(product_id)
            .await?;
        match routing {
            Some(r) => {
                let ops = self.routing_repo.get_operations(r.id).await?;
                let total_time: Decimal = ops.iter().map(|op| op.setup_hours + op.run_hours).sum();
                Ok(total_time)
            }
            None => Ok(Decimal::ZERO),
        }
    }

    // Work Order soft-delete methods
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

    pub async fn restore_work_order(&self, id: i64, tenant_id: i64) -> Result<WorkOrder, ApiError> {
        self.work_order_repo.restore(id, tenant_id).await
    }

    pub async fn list_deleted_work_orders(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<WorkOrder>, ApiError> {
        self.work_order_repo.find_deleted(tenant_id).await
    }

    pub async fn destroy_work_order(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.work_order_repo.destroy(id, tenant_id).await
    }

    // BOM soft-delete methods
    pub async fn soft_delete_bom(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.bom_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    pub async fn restore_bom(&self, id: i64, tenant_id: i64) -> Result<BillOfMaterials, ApiError> {
        self.bom_repo.restore(id, tenant_id).await
    }

    pub async fn list_deleted_boms(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<BillOfMaterials>, ApiError> {
        self.bom_repo.find_deleted(tenant_id).await
    }

    pub async fn destroy_bom(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.bom_repo.destroy(id, tenant_id).await
    }

    // Routing soft-delete methods
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

    pub async fn restore_routing(&self, id: i64, tenant_id: i64) -> Result<Routing, ApiError> {
        self.routing_repo.restore(id, tenant_id).await
    }

    pub async fn list_deleted_routings(&self, tenant_id: i64) -> Result<Vec<Routing>, ApiError> {
        self.routing_repo.find_deleted(tenant_id).await
    }

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
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_service() -> ManufacturingService {
        let wo_repo = Arc::new(InMemoryWorkOrderRepository::new()) as BoxWorkOrderRepository;
        let bom_repo =
            Arc::new(InMemoryBillOfMaterialsRepository::new()) as BoxBillOfMaterialsRepository;
        let routing_repo = Arc::new(InMemoryRoutingRepository::new()) as BoxRoutingRepository;
        ManufacturingService::new(wo_repo, bom_repo, routing_repo)
    }

    #[tokio::test]
    async fn test_create_work_order() {
        let service = create_service();
        let create = CreateWorkOrder {
            tenant_id: 1,
            name: "WO-001".to_string(),
            product_id: 1,
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
    async fn test_create_bom() {
        let service = create_service();
        let create = CreateBillOfMaterials {
            tenant_id: 1,
            product_id: 1,
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
    async fn test_add_bom_line() {
        let service = create_service();
        let bom = service
            .create_bom(CreateBillOfMaterials {
                tenant_id: 1,
                product_id: 1,
                version: "1.0".to_string(),
                is_active: true,
                is_primary: true,
                valid_from: Some(chrono::Utc::now()),
                valid_to: None,
            })
            .await
            .unwrap();

        let line = service
            .add_bom_line(CreateBillOfMaterialsLine {
                bom_id: bom.id,
                component_product_id: 2,
                quantity: dec!(5),
                unit_id: Some(1),
                scrap_percentage: dec!(5),
                is_optional: false,
                notes: None,
            })
            .await
            .unwrap();

        assert_eq!(line.quantity, dec!(5));
    }

    #[tokio::test]
    async fn test_create_routing() {
        let service = create_service();
        let create = CreateRouting {
            tenant_id: 1,
            product_id: 1,
            version: "1.0".to_string(),
            is_active: true,
            is_primary: true,
        };
        let result = service.create_routing(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_material_requirements_calculation() {
        let service = create_service();

        // Create BOM
        let bom = service
            .create_bom(CreateBillOfMaterials {
                tenant_id: 1,
                product_id: 1,
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
            .add_bom_line(CreateBillOfMaterialsLine {
                bom_id: bom.id,
                component_product_id: 2,
                quantity: dec!(2),
                unit_id: Some(1),
                scrap_percentage: dec!(10),
                is_optional: false,
                notes: None,
            })
            .await
            .unwrap();

        // Calculate for quantity of 10
        let requirements = service
            .calculate_material_requirements(1, dec!(10))
            .await
            .unwrap();
        assert_eq!(requirements.len(), 1);
        // 10 * 2 * 1.1 = 22.0 (with 10% scrap)
        assert_eq!(requirements[0].1, dec!(22));
    }

    #[tokio::test]
    async fn test_production_time_calculation() {
        let service = create_service();

        // Create routing
        let routing = service
            .create_routing(CreateRouting {
                tenant_id: 1,
                product_id: 1,
                version: "1.0".to_string(),
                is_active: true,
                is_primary: true,
            })
            .await
            .unwrap();

        // Add operations
        service
            .add_routing_operation(CreateRoutingOperation {
                routing_id: routing.id,
                sequence: 1,
                operation_name: "Setup".to_string(),
                work_center_id: Some(1),
                setup_hours: dec!(1),
                run_hours: Decimal::ZERO,
                description: None,
            })
            .await
            .unwrap();

        service
            .add_routing_operation(CreateRoutingOperation {
                routing_id: routing.id,
                sequence: 2,
                operation_name: "Assembly".to_string(),
                work_center_id: Some(1),
                setup_hours: Decimal::ZERO,
                run_hours: dec!(5),
                description: None,
            })
            .await
            .unwrap();

        let time = service.calculate_production_time(1).await.unwrap();
        assert_eq!(time, dec!(6)); // 1 + 5
    }
}

// ==================== Quality Control Service ====================

use crate::domain::manufacturing::model::{
    CreateInspection, CreateNonConformanceReport, Inspection, NonConformanceReport,
    UpdateInspection, UpdateNonConformanceReport,
};
use crate::domain::manufacturing::repository::{BoxInspectionRepository, BoxNcrRepository};

#[derive(Clone)]
pub struct QualityControlService {
    inspection_repo: BoxInspectionRepository,
    ncr_repo: BoxNcrRepository,
}

impl QualityControlService {
    pub fn new(inspection_repo: BoxInspectionRepository, ncr_repo: BoxNcrRepository) -> Self {
        Self {
            inspection_repo,
            ncr_repo,
        }
    }

    // Inspection methods
    pub async fn create_inspection(
        &self,
        create: CreateInspection,
    ) -> Result<Inspection, ApiError> {
        self.inspection_repo.create(create).await
    }

    pub async fn get_inspection(&self, id: i64, tenant_id: i64) -> Result<Inspection, ApiError> {
        self.inspection_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Inspection not found".to_string()))
    }

    pub async fn get_inspections_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Inspection>, ApiError> {
        self.inspection_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_inspections_by_work_order(
        &self,
        work_order_id: i64,
    ) -> Result<Vec<Inspection>, ApiError> {
        self.inspection_repo.find_by_work_order(work_order_id).await
    }

    pub async fn update_inspection(
        &self,
        id: i64,
        update: UpdateInspection,
    ) -> Result<Inspection, ApiError> {
        self.inspection_repo.update(id, update).await
    }

    pub async fn delete_inspection(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.inspection_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    // NCR methods
    pub async fn create_ncr(
        &self,
        create: CreateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError> {
        self.ncr_repo.create(create).await
    }

    pub async fn get_ncr(&self, id: i64, tenant_id: i64) -> Result<NonConformanceReport, ApiError> {
        self.ncr_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("NCR not found".to_string()))
    }

    pub async fn get_ncrs_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<NonConformanceReport>, ApiError> {
        self.ncr_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_ncrs_by_inspection(
        &self,
        inspection_id: i64,
    ) -> Result<Vec<NonConformanceReport>, ApiError> {
        self.ncr_repo.find_by_inspection(inspection_id).await
    }

    pub async fn update_ncr(
        &self,
        id: i64,
        update: UpdateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError> {
        self.ncr_repo.update(id, update).await
    }

    pub async fn delete_ncr(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.ncr_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    // Inspection soft-delete methods
    pub async fn restore_inspection(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Inspection, ApiError> {
        self.inspection_repo.restore(id, tenant_id).await
    }

    pub async fn list_deleted_inspections(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Inspection>, ApiError> {
        self.inspection_repo.find_deleted(tenant_id).await
    }

    pub async fn destroy_inspection(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.inspection_repo.destroy(id, tenant_id).await
    }

    // NCR soft-delete methods
    pub async fn restore_ncr(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<NonConformanceReport, ApiError> {
        self.ncr_repo.restore(id, tenant_id).await
    }

    pub async fn list_deleted_ncrs(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<NonConformanceReport>, ApiError> {
        self.ncr_repo.find_deleted(tenant_id).await
    }

    pub async fn destroy_ncr(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.ncr_repo.destroy(id, tenant_id).await
    }
}

#[cfg(test)]
mod qc_tests {
    use super::*;
    use crate::domain::manufacturing::model::{
        CreateInspection, CreateNonConformanceReport, InspectionStatus, NcrType,
    };
    use crate::domain::manufacturing::repository::{
        InMemoryInspectionRepository, InMemoryNcrRepository,
    };
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_qc_service() -> QualityControlService {
        let inspection_repo =
            Arc::new(InMemoryInspectionRepository::new()) as BoxInspectionRepository;
        let ncr_repo = Arc::new(InMemoryNcrRepository::new()) as BoxNcrRepository;
        QualityControlService::new(inspection_repo, ncr_repo)
    }

    #[tokio::test]
    async fn test_create_inspection() {
        let service = create_qc_service();
        let create = CreateInspection {
            tenant_id: 1,
            work_order_id: Some(1),
            product_id: 1,
            inspection_type: "Visual".to_string(),
            quantity_inspected: dec!(100),
            quantity_passed: dec!(95),
            quantity_failed: dec!(5),
            status: InspectionStatus::Passed,
            inspector_id: Some(1),
            notes: None,
        };
        let result = service.create_inspection(create).await;
        assert!(result.is_ok());
        let inspection = result.unwrap();
        assert_eq!(inspection.quantity_inspected, dec!(100));
        assert_eq!(inspection.status, InspectionStatus::Passed);
    }

    #[tokio::test]
    async fn test_create_ncr() {
        let service = create_qc_service();
        let create = CreateNonConformanceReport {
            tenant_id: 1,
            inspection_id: Some(1),
            product_id: 1,
            ncr_type: NcrType::Minor,
            description: "Scratch on surface".to_string(),
            root_cause: None,
            corrective_action: None,
            raised_by: 1,
        };
        let result = service.create_ncr(create).await;
        assert!(result.is_ok());
        let ncr = result.unwrap();
        assert_eq!(ncr.description, "Scratch on surface");
    }
}
