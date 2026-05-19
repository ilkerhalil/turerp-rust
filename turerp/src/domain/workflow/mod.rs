//! Workflow engine domain module
//!
//! Provides configurable approval workflows for documents and business processes.

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    ApproveStep, Condition, CreateWorkflowInstance, CreateWorkflowTemplate, EscalationRule,
    ParallelConfig, ParallelMode, RejectStep, RoleAssignment, WorkflowAuditLog,
    WorkflowAuditLogResponse, WorkflowEntityType, WorkflowInstance, WorkflowInstanceDetailResponse,
    WorkflowInstanceResponse, WorkflowStatus, WorkflowStep, WorkflowStepApproval,
    WorkflowStepResponse, WorkflowStepStatus, WorkflowTemplate, WorkflowTemplateResponse,
};
pub use postgres_repository::PostgresWorkflowRepository;
pub use repository::{BoxWorkflowRepository, InMemoryWorkflowRepository, WorkflowRepository};
pub use service::WorkflowService;
