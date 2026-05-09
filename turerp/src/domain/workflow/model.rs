//! Workflow domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Entity types that can have workflow approval processes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowEntityType {
    Invoice,
    PurchaseOrder,
    Expense,
    StockTransfer,
}

impl std::fmt::Display for WorkflowEntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowEntityType::Invoice => write!(f, "invoice"),
            WorkflowEntityType::PurchaseOrder => write!(f, "purchase_order"),
            WorkflowEntityType::Expense => write!(f, "expense"),
            WorkflowEntityType::StockTransfer => write!(f, "stock_transfer"),
        }
    }
}

impl std::str::FromStr for WorkflowEntityType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "invoice" => Ok(WorkflowEntityType::Invoice),
            "purchase_order" => Ok(WorkflowEntityType::PurchaseOrder),
            "expense" => Ok(WorkflowEntityType::Expense),
            "stock_transfer" => Ok(WorkflowEntityType::StockTransfer),
            _ => Err(format!("Invalid workflow entity type: {}", s)),
        }
    }
}

/// Overall status of a workflow instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    #[default]
    Draft,
    Pending,
    Approved,
    Rejected,
    Completed,
}

impl std::fmt::Display for WorkflowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowStatus::Draft => write!(f, "draft"),
            WorkflowStatus::Pending => write!(f, "pending"),
            WorkflowStatus::Approved => write!(f, "approved"),
            WorkflowStatus::Rejected => write!(f, "rejected"),
            WorkflowStatus::Completed => write!(f, "completed"),
        }
    }
}

impl std::str::FromStr for WorkflowStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(WorkflowStatus::Draft),
            "pending" => Ok(WorkflowStatus::Pending),
            "approved" => Ok(WorkflowStatus::Approved),
            "rejected" => Ok(WorkflowStatus::Rejected),
            "completed" => Ok(WorkflowStatus::Completed),
            _ => Err(format!("Invalid workflow status: {}", s)),
        }
    }
}

/// Status of an individual workflow step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStepStatus {
    #[default]
    Pending,
    Approved,
    Rejected,
}

impl std::fmt::Display for WorkflowStepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowStepStatus::Pending => write!(f, "pending"),
            WorkflowStepStatus::Approved => write!(f, "approved"),
            WorkflowStepStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl std::str::FromStr for WorkflowStepStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(WorkflowStepStatus::Pending),
            "approved" => Ok(WorkflowStepStatus::Approved),
            "rejected" => Ok(WorkflowStepStatus::Rejected),
            _ => Err(format!("Invalid workflow step status: {}", s)),
        }
    }
}

/// Reusable workflow template defining steps for an entity type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowTemplate {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub description: String,
    pub entity_type: WorkflowEntityType,
    pub config_json: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for WorkflowTemplate {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

/// Running instance of a workflow attached to a specific entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowInstance {
    pub id: i64,
    pub tenant_id: i64,
    pub template_id: i64,
    pub entity_id: i64,
    pub entity_type: WorkflowEntityType,
    pub status: WorkflowStatus,
    pub current_step: i32,
    pub assigned_user_id: Option<i64>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Individual step within a workflow instance
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowStep {
    pub id: i64,
    pub instance_id: i64,
    pub step_number: i32,
    pub step_name: String,
    pub approver_role: Option<String>,
    pub approver_user_id: Option<i64>,
    pub status: WorkflowStepStatus,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Immutable audit trail entry for workflow actions
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowAuditLog {
    pub id: i64,
    pub instance_id: i64,
    pub step_id: Option<i64>,
    pub action: String,
    pub user_id: i64,
    pub comment: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// ---- DTOs ----

/// Create a new workflow template
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkflowTemplate {
    pub name: String,
    pub description: String,
    pub entity_type: WorkflowEntityType,
    pub config_json: serde_json::Value,
}

/// Start a new workflow instance
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkflowInstance {
    pub template_id: i64,
    pub entity_id: i64,
    pub entity_type: WorkflowEntityType,
}

/// Approve the current step
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApproveStep {
    pub comment: Option<String>,
}

/// Reject the current step
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RejectStep {
    pub comment: Option<String>,
}

// ---- Responses ----

/// Workflow template response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowTemplateResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub description: String,
    pub entity_type: WorkflowEntityType,
    pub config_json: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<WorkflowTemplate> for WorkflowTemplateResponse {
    fn from(t: WorkflowTemplate) -> Self {
        Self {
            id: t.id,
            tenant_id: t.tenant_id,
            name: t.name,
            description: t.description,
            entity_type: t.entity_type,
            config_json: t.config_json,
            is_active: t.is_active,
            created_at: t.created_at,
        }
    }
}

/// Workflow instance response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowInstanceResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub template_id: i64,
    pub entity_id: i64,
    pub entity_type: WorkflowEntityType,
    pub status: WorkflowStatus,
    pub current_step: i32,
    pub assigned_user_id: Option<i64>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<WorkflowInstance> for WorkflowInstanceResponse {
    fn from(i: WorkflowInstance) -> Self {
        Self {
            id: i.id,
            tenant_id: i.tenant_id,
            template_id: i.template_id,
            entity_id: i.entity_id,
            entity_type: i.entity_type,
            status: i.status,
            current_step: i.current_step,
            assigned_user_id: i.assigned_user_id,
            created_by: i.created_by,
            created_at: i.created_at,
            completed_at: i.completed_at,
        }
    }
}

/// Workflow step response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowStepResponse {
    pub id: i64,
    pub instance_id: i64,
    pub step_number: i32,
    pub step_name: String,
    pub approver_role: Option<String>,
    pub approver_user_id: Option<i64>,
    pub status: WorkflowStepStatus,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<WorkflowStep> for WorkflowStepResponse {
    fn from(s: WorkflowStep) -> Self {
        Self {
            id: s.id,
            instance_id: s.instance_id,
            step_number: s.step_number,
            step_name: s.step_name,
            approver_role: s.approver_role,
            approver_user_id: s.approver_user_id,
            status: s.status,
            comment: s.comment,
            created_at: s.created_at,
            completed_at: s.completed_at,
        }
    }
}

/// Workflow audit log response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowAuditLogResponse {
    pub id: i64,
    pub instance_id: i64,
    pub step_id: Option<i64>,
    pub action: String,
    pub user_id: i64,
    pub comment: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl From<WorkflowAuditLog> for WorkflowAuditLogResponse {
    fn from(l: WorkflowAuditLog) -> Self {
        Self {
            id: l.id,
            instance_id: l.instance_id,
            step_id: l.step_id,
            action: l.action,
            user_id: l.user_id,
            comment: l.comment,
            timestamp: l.timestamp,
        }
    }
}

/// Detailed workflow instance with steps and audit log
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowInstanceDetailResponse {
    #[serde(flatten)]
    pub instance: WorkflowInstanceResponse,
    pub steps: Vec<WorkflowStepResponse>,
    pub audit_log: Vec<WorkflowAuditLogResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_entity_type_display() {
        assert_eq!(WorkflowEntityType::Invoice.to_string(), "invoice");
        assert_eq!(
            WorkflowEntityType::PurchaseOrder.to_string(),
            "purchase_order"
        );
        assert_eq!(WorkflowEntityType::Expense.to_string(), "expense");
        assert_eq!(
            WorkflowEntityType::StockTransfer.to_string(),
            "stock_transfer"
        );
    }

    #[test]
    fn test_workflow_status_display() {
        assert_eq!(WorkflowStatus::Draft.to_string(), "draft");
        assert_eq!(WorkflowStatus::Pending.to_string(), "pending");
        assert_eq!(WorkflowStatus::Approved.to_string(), "approved");
        assert_eq!(WorkflowStatus::Rejected.to_string(), "rejected");
        assert_eq!(WorkflowStatus::Completed.to_string(), "completed");
    }

    #[test]
    fn test_workflow_step_status_display() {
        assert_eq!(WorkflowStepStatus::Pending.to_string(), "pending");
        assert_eq!(WorkflowStepStatus::Approved.to_string(), "approved");
        assert_eq!(WorkflowStepStatus::Rejected.to_string(), "rejected");
    }

    #[test]
    fn test_entity_type_from_str() {
        assert_eq!(
            "invoice".parse::<WorkflowEntityType>().unwrap(),
            WorkflowEntityType::Invoice
        );
        assert_eq!(
            "purchase_order".parse::<WorkflowEntityType>().unwrap(),
            WorkflowEntityType::PurchaseOrder
        );
        assert!("invalid".parse::<WorkflowEntityType>().is_err());
    }
}
