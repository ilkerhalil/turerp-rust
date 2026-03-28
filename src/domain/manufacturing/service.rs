//! Manufacturing service for business logic

#[allow(unused_imports)]
use std::sync::Arc;

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

    pub async fn get_work_order(&self, id: i64) -> Result<WorkOrder, ApiError> {
        self.work_order_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Work order not found".to_string()))
    }

    pub async fn get_work_orders_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<WorkOrder>, ApiError> {
        self.work_order_repo.find_by_tenant(tenant_id).await
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

    pub async fn get_bom(&self, id: i64) -> Result<BillOfMaterials, ApiError> {
        self.bom_repo
            .find_by_id(id)
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

    pub async fn get_routing(&self, id: i64) -> Result<Routing, ApiError> {
        self.routing_repo
            .find_by_id(id)
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
        quantity: f64,
    ) -> Result<Vec<(i64, f64)>, ApiError> {
        let bom = self.bom_repo.find_primary_by_product(product_id).await?;
        match bom {
            Some(bom) => {
                let lines = self.bom_repo.get_lines(bom.id).await?;
                let mut requirements = Vec::new();
                for line in lines {
                    let scrap_factor = 1.0 + (line.scrap_percentage / 100.0);
                    let required_qty = quantity * line.quantity * scrap_factor;
                    requirements.push((line.component_product_id, required_qty));
                }
                Ok(requirements)
            }
            None => Ok(Vec::new()),
        }
    }

    // Calculate production time from routing
    pub async fn calculate_production_time(&self, product_id: i64) -> Result<f64, ApiError> {
        let routing = self
            .routing_repo
            .find_primary_by_product(product_id)
            .await?;
        match routing {
            Some(r) => {
                let ops = self.routing_repo.get_operations(r.id).await?;
                let total_time: f64 = ops.iter().map(|op| op.setup_hours + op.run_hours).sum();
                Ok(total_time)
            }
            None => Ok(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::manufacturing::repository::{
        InMemoryBillOfMaterialsRepository, InMemoryRoutingRepository, InMemoryWorkOrderRepository,
    };

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
            quantity: 100.0,
            bom_id: None,
            routing_id: None,
            priority: crate::domain::manufacturing::model::WorkOrderPriority::Normal,
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
                quantity: 5.0,
                unit_id: Some(1),
                scrap_percentage: 5.0,
                is_optional: false,
                notes: None,
            })
            .await
            .unwrap();

        assert_eq!(line.quantity, 5.0);
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
                quantity: 2.0,
                unit_id: Some(1),
                scrap_percentage: 10.0,
                is_optional: false,
                notes: None,
            })
            .await
            .unwrap();

        // Calculate for quantity of 10
        let requirements = service
            .calculate_material_requirements(1, 10.0)
            .await
            .unwrap();
        assert_eq!(requirements.len(), 1);
        // 10 * 2 * 1.1 = 22.0 (with 10% scrap)
        assert!((requirements[0].1 - 22.0).abs() < 0.001);
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
                setup_hours: 1.0,
                run_hours: 0.0,
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
                setup_hours: 0.0,
                run_hours: 5.0,
                description: None,
            })
            .await
            .unwrap();

        let time = service.calculate_production_time(1).await.unwrap();
        assert_eq!(time, 6.0); // 1.0 + 5.0
    }
}
