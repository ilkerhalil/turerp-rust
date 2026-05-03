//! Manufacturing domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    BillOfMaterials, BillOfMaterialsLine, CreateBillOfMaterials, CreateBillOfMaterialsLine,
    CreateInspection, CreateNonConformanceReport, CreateRouting, CreateRoutingOperation,
    CreateWorkOrder, CreateWorkOrderMaterial, CreateWorkOrderOperation, Inspection,
    InspectionStatus, NcrStatus, NcrType, NonConformanceReport, Routing, RoutingOperation,
    UpdateInspection, UpdateNonConformanceReport, WorkOrder, WorkOrderMaterial, WorkOrderOperation,
    WorkOrderPriority, WorkOrderStatus,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::{
    PostgresBillOfMaterialsRepository, PostgresRoutingRepository, PostgresWorkOrderRepository,
};
pub use repository::{
    BillOfMaterialsRepository, BoxBillOfMaterialsRepository, BoxInspectionRepository,
    BoxNcrRepository, BoxRoutingRepository, BoxWorkOrderRepository,
    InMemoryBillOfMaterialsRepository, InMemoryInspectionRepository, InMemoryNcrRepository,
    InMemoryRoutingRepository, InMemoryWorkOrderRepository, InspectionRepository, NcrRepository,
    RoutingRepository, WorkOrderRepository,
};
pub use service::{ManufacturingService, QualityControlService};
