//! Workflow engine domain module
//!
//! Provides configurable approval workflows for documents and business processes.

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    ApproveStep, CreateWorkflowInstance, CreateWorkflowTemplate, RejectStep, WorkflowAuditLog,
    WorkflowAuditLogResponse, WorkflowEntityType, WorkflowInstance, WorkflowInstanceDetailResponse,
    WorkflowInstanceResponse, WorkflowStatus, WorkflowStep, WorkflowStepResponse,
    WorkflowStepStatus, WorkflowTemplate, WorkflowTemplateResponse,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresWorkflowRepository;
pub use repository::{BoxWorkflowRepository, InMemoryWorkflowRepository, WorkflowRepository};
pub use service::WorkflowService;
