//! Manufacturing repository

use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

use crate::domain::manufacturing::model::{
    BillOfMaterials, BillOfMaterialsLine, CreateBillOfMaterials, CreateBillOfMaterialsLine,
    CreateRouting, CreateRoutingOperation, CreateWorkOrder, CreateWorkOrderMaterial,
    CreateWorkOrderOperation, Routing, RoutingOperation, WorkOrder, WorkOrderMaterial,
    WorkOrderOperation, WorkOrderStatus,
};
use crate::error::ApiError;

#[async_trait]
pub trait WorkOrderRepository: Send + Sync {
    async fn create(&self, work_order: CreateWorkOrder) -> Result<WorkOrder, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<WorkOrder>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<WorkOrder>, ApiError>;
    async fn find_by_product(&self, product_id: i64) -> Result<Vec<WorkOrder>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: WorkOrderStatus,
    ) -> Result<Vec<WorkOrder>, ApiError>;
    async fn update_status(&self, id: i64, status: WorkOrderStatus) -> Result<WorkOrder, ApiError>;
    async fn add_operation(
        &self,
        op: CreateWorkOrderOperation,
    ) -> Result<WorkOrderOperation, ApiError>;
    async fn get_operations(&self, work_order_id: i64)
        -> Result<Vec<WorkOrderOperation>, ApiError>;
    async fn add_material(
        &self,
        mat: CreateWorkOrderMaterial,
    ) -> Result<WorkOrderMaterial, ApiError>;
    async fn get_materials(&self, work_order_id: i64) -> Result<Vec<WorkOrderMaterial>, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

#[async_trait]
pub trait BillOfMaterialsRepository: Send + Sync {
    async fn create(&self, bom: CreateBillOfMaterials) -> Result<BillOfMaterials, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<BillOfMaterials>, ApiError>;
    async fn find_by_product(&self, product_id: i64) -> Result<Vec<BillOfMaterials>, ApiError>;
    async fn find_primary_by_product(
        &self,
        product_id: i64,
    ) -> Result<Option<BillOfMaterials>, ApiError>;
    async fn add_line(
        &self,
        line: CreateBillOfMaterialsLine,
    ) -> Result<BillOfMaterialsLine, ApiError>;
    async fn get_lines(&self, bom_id: i64) -> Result<Vec<BillOfMaterialsLine>, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

#[async_trait]
pub trait RoutingRepository: Send + Sync {
    async fn create(&self, routing: CreateRouting) -> Result<Routing, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Routing>, ApiError>;
    async fn find_by_product(&self, product_id: i64) -> Result<Vec<Routing>, ApiError>;
    async fn find_primary_by_product(&self, product_id: i64) -> Result<Option<Routing>, ApiError>;
    async fn add_operation(
        &self,
        create: CreateRoutingOperation,
    ) -> Result<RoutingOperation, ApiError>;
    async fn get_operations(&self, routing_id: i64) -> Result<Vec<RoutingOperation>, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

pub type BoxWorkOrderRepository = Arc<dyn WorkOrderRepository>;
pub type BoxBillOfMaterialsRepository = Arc<dyn BillOfMaterialsRepository>;
pub type BoxRoutingRepository = Arc<dyn RoutingRepository>;

// ==================== IN-MEMORY IMPLEMENTATIONS ====================

pub struct InMemoryWorkOrderRepository {
    work_orders: std::sync::Mutex<std::collections::HashMap<i64, WorkOrder>>,
    operations: std::sync::Mutex<std::collections::HashMap<i64, Vec<WorkOrderOperation>>>,
    materials: std::sync::Mutex<std::collections::HashMap<i64, Vec<WorkOrderMaterial>>>,
    next_id: std::sync::Mutex<i64>,
    next_op_id: std::sync::Mutex<i64>,
    next_mat_id: std::sync::Mutex<i64>,
}

impl InMemoryWorkOrderRepository {
    pub fn new() -> Self {
        Self {
            work_orders: std::sync::Mutex::new(std::collections::HashMap::new()),
            operations: std::sync::Mutex::new(std::collections::HashMap::new()),
            materials: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
            next_op_id: std::sync::Mutex::new(1),
            next_mat_id: std::sync::Mutex::new(1),
        }
    }
}
impl Default for InMemoryWorkOrderRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkOrderRepository for InMemoryWorkOrderRepository {
    async fn create(&self, create: CreateWorkOrder) -> Result<WorkOrder, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let now = Utc::now();
        let work_order = WorkOrder {
            id,
            tenant_id: create.tenant_id,
            name: create.name,
            product_id: create.product_id,
            quantity: create.quantity,
            bom_id: create.bom_id,
            routing_id: create.routing_id,
            status: WorkOrderStatus::Draft,
            priority: create.priority,
            planned_start: create.planned_start,
            planned_end: create.planned_end,
            actual_start: None,
            actual_end: None,
            created_at: now,
            updated_at: now,
        };
        self.work_orders
            .lock()
            .unwrap()
            .insert(id, work_order.clone());
        Ok(work_order)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<WorkOrder>, ApiError> {
        Ok(self.work_orders.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<WorkOrder>, ApiError> {
        let wo = self.work_orders.lock().unwrap();
        Ok(wo
            .values()
            .filter(|x| x.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<WorkOrder>, ApiError> {
        let wo = self.work_orders.lock().unwrap();
        Ok(wo
            .values()
            .filter(|x| x.product_id == product_id)
            .cloned()
            .collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: WorkOrderStatus,
    ) -> Result<Vec<WorkOrder>, ApiError> {
        let wo = self.work_orders.lock().unwrap();
        Ok(wo
            .values()
            .filter(|x| x.tenant_id == tenant_id && x.status == status)
            .cloned()
            .collect())
    }

    async fn update_status(&self, id: i64, status: WorkOrderStatus) -> Result<WorkOrder, ApiError> {
        let mut wo = self.work_orders.lock().unwrap();
        let order = wo
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound("Work order not found".to_string()))?;
        order.status = status;
        order.updated_at = Utc::now();
        Ok(order.clone())
    }

    async fn add_operation(
        &self,
        op: CreateWorkOrderOperation,
    ) -> Result<WorkOrderOperation, ApiError> {
        op.validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_op_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let operation = WorkOrderOperation {
            id,
            work_order_id: op.work_order_id,
            operation_sequence: op.operation_sequence,
            operation_name: op.operation_name,
            work_center_id: op.work_center_id,
            planned_hours: op.planned_hours,
            actual_hours: 0.0,
            status: "Pending".to_string(),
            started_at: None,
            completed_at: None,
        };
        self.operations
            .lock()
            .unwrap()
            .entry(op.work_order_id)
            .or_default()
            .push(operation.clone());
        Ok(operation)
    }

    async fn get_operations(
        &self,
        work_order_id: i64,
    ) -> Result<Vec<WorkOrderOperation>, ApiError> {
        Ok(self
            .operations
            .lock()
            .unwrap()
            .get(&work_order_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn add_material(
        &self,
        mat: CreateWorkOrderMaterial,
    ) -> Result<WorkOrderMaterial, ApiError> {
        mat.validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_mat_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let material = WorkOrderMaterial {
            id,
            work_order_id: mat.work_order_id,
            product_id: mat.product_id,
            quantity_required: mat.quantity_required,
            quantity_issued: 0.0,
            is_issued: false,
        };
        self.materials
            .lock()
            .unwrap()
            .entry(mat.work_order_id)
            .or_default()
            .push(material.clone());
        Ok(material)
    }

    async fn get_materials(&self, work_order_id: i64) -> Result<Vec<WorkOrderMaterial>, ApiError> {
        Ok(self
            .materials
            .lock()
            .unwrap()
            .get(&work_order_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.work_orders.lock().unwrap().remove(&id);
        Ok(())
    }
}

pub struct InMemoryBillOfMaterialsRepository {
    boms: std::sync::Mutex<std::collections::HashMap<i64, BillOfMaterials>>,
    lines: std::sync::Mutex<std::collections::HashMap<i64, Vec<BillOfMaterialsLine>>>,
    next_id: std::sync::Mutex<i64>,
    next_line_id: std::sync::Mutex<i64>,
}

impl InMemoryBillOfMaterialsRepository {
    pub fn new() -> Self {
        Self {
            boms: std::sync::Mutex::new(std::collections::HashMap::new()),
            lines: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
            next_line_id: std::sync::Mutex::new(1),
        }
    }
}
impl Default for InMemoryBillOfMaterialsRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BillOfMaterialsRepository for InMemoryBillOfMaterialsRepository {
    async fn create(&self, create: CreateBillOfMaterials) -> Result<BillOfMaterials, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let now = Utc::now();
        let bom = BillOfMaterials {
            id,
            tenant_id: create.tenant_id,
            product_id: create.product_id,
            version: create.version,
            is_active: create.is_active,
            is_primary: create.is_primary,
            valid_from: create.valid_from,
            valid_to: create.valid_to,
            created_at: now,
            updated_at: now,
        };
        self.boms.lock().unwrap().insert(id, bom.clone());
        Ok(bom)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<BillOfMaterials>, ApiError> {
        Ok(self.boms.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<BillOfMaterials>, ApiError> {
        let bom = self.boms.lock().unwrap();
        Ok(bom
            .values()
            .filter(|x| x.product_id == product_id)
            .cloned()
            .collect())
    }

    async fn find_primary_by_product(
        &self,
        product_id: i64,
    ) -> Result<Option<BillOfMaterials>, ApiError> {
        let bom = self.boms.lock().unwrap();
        Ok(bom
            .values()
            .filter(|x| x.product_id == product_id && x.is_primary && x.is_active)
            .cloned()
            .collect::<Vec<_>>()
            .pop())
    }

    async fn add_line(
        &self,
        line: CreateBillOfMaterialsLine,
    ) -> Result<BillOfMaterialsLine, ApiError> {
        line.validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_line_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let bom_line = BillOfMaterialsLine {
            id,
            bom_id: line.bom_id,
            component_product_id: line.component_product_id,
            quantity: line.quantity,
            unit_id: line.unit_id,
            scrap_percentage: line.scrap_percentage,
            is_optional: line.is_optional,
            notes: line.notes,
        };
        self.lines
            .lock()
            .unwrap()
            .entry(line.bom_id)
            .or_default()
            .push(bom_line.clone());
        Ok(bom_line)
    }

    async fn get_lines(&self, bom_id: i64) -> Result<Vec<BillOfMaterialsLine>, ApiError> {
        Ok(self
            .lines
            .lock()
            .unwrap()
            .get(&bom_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.boms.lock().unwrap().remove(&id);
        Ok(())
    }
}

pub struct InMemoryRoutingRepository {
    routings: std::sync::Mutex<std::collections::HashMap<i64, Routing>>,
    operations: std::sync::Mutex<std::collections::HashMap<i64, Vec<RoutingOperation>>>,
    next_id: std::sync::Mutex<i64>,
    next_op_id: std::sync::Mutex<i64>,
}

impl InMemoryRoutingRepository {
    pub fn new() -> Self {
        Self {
            routings: std::sync::Mutex::new(std::collections::HashMap::new()),
            operations: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
            next_op_id: std::sync::Mutex::new(1),
        }
    }
}
impl Default for InMemoryRoutingRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RoutingRepository for InMemoryRoutingRepository {
    async fn create(&self, create: CreateRouting) -> Result<Routing, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let now = Utc::now();
        let routing = Routing {
            id,
            tenant_id: create.tenant_id,
            product_id: create.product_id,
            version: create.version,
            is_active: create.is_active,
            is_primary: create.is_primary,
            created_at: now,
            updated_at: now,
        };
        self.routings.lock().unwrap().insert(id, routing.clone());
        Ok(routing)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Routing>, ApiError> {
        Ok(self.routings.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<Routing>, ApiError> {
        let r = self.routings.lock().unwrap();
        Ok(r.values()
            .filter(|x| x.product_id == product_id)
            .cloned()
            .collect())
    }

    async fn find_primary_by_product(&self, product_id: i64) -> Result<Option<Routing>, ApiError> {
        let r = self.routings.lock().unwrap();
        Ok(r.values()
            .filter(|x| x.product_id == product_id && x.is_primary && x.is_active)
            .cloned()
            .collect::<Vec<_>>()
            .pop())
    }

    async fn add_operation(
        &self,
        create: CreateRoutingOperation,
    ) -> Result<RoutingOperation, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let mut next_id = self.next_op_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;
        let op = RoutingOperation {
            id,
            routing_id: create.routing_id,
            sequence: create.sequence,
            operation_name: create.operation_name,
            work_center_id: create.work_center_id,
            setup_hours: create.setup_hours,
            run_hours: create.run_hours,
            description: create.description,
        };
        self.operations
            .lock()
            .unwrap()
            .entry(create.routing_id)
            .or_default()
            .push(op.clone());
        Ok(op)
    }

    async fn get_operations(&self, routing_id: i64) -> Result<Vec<RoutingOperation>, ApiError> {
        Ok(self
            .operations
            .lock()
            .unwrap()
            .get(&routing_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.routings.lock().unwrap().remove(&id);
        Ok(())
    }
}
