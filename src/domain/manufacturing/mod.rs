//! Manufacturing domain module

pub mod model;
pub mod repository;
pub mod service;

pub use model::{
    BillOfMaterials, BillOfMaterialsLine, CreateBillOfMaterials, CreateBillOfMaterialsLine,
    CreateRouting, CreateRoutingOperation, CreateWorkOrder, CreateWorkOrderMaterial,
    CreateWorkOrderOperation, Inspection, InspectionStatus, NcrStatus, NcrType,
    NonConformanceReport, Routing, RoutingOperation, WorkOrder, WorkOrderMaterial,
    WorkOrderOperation, WorkOrderPriority, WorkOrderStatus,
};
pub use repository::{
    BillOfMaterialsRepository, BoxBillOfMaterialsRepository, BoxRoutingRepository,
    BoxWorkOrderRepository, InMemoryBillOfMaterialsRepository, InMemoryRoutingRepository,
    InMemoryWorkOrderRepository, RoutingRepository, WorkOrderRepository,
};
pub use service::ManufacturingService;
