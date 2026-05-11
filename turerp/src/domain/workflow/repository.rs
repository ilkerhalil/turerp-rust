//! Workflow repository traits and in-memory implementations

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::common::SoftDeletable;
use crate::domain::workflow::model::{
    CreateWorkflowTemplate, WorkflowAuditLog, WorkflowEntityType, WorkflowInstance, WorkflowStatus,
    WorkflowStep, WorkflowStepApproval, WorkflowStepStatus, WorkflowTemplate,
};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// WorkflowRepository
// ---------------------------------------------------------------------------

/// Repository trait for workflow operations
#[async_trait]
pub trait WorkflowRepository: Send + Sync {
    /// Create a new workflow template
    async fn create_template(
        &self,
        template: CreateWorkflowTemplate,
        tenant_id: i64,
    ) -> Result<WorkflowTemplate, ApiError>;

    /// Find a template by ID
    async fn find_template_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<WorkflowTemplate>, ApiError>;

    /// List all templates for a tenant
    async fn find_templates(&self, tenant_id: i64) -> Result<Vec<WorkflowTemplate>, ApiError>;

    /// Delete a template
    async fn delete_template(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    // Instances

    /// Create a new workflow instance
    async fn create_instance(
        &self,
        template_id: i64,
        entity_id: i64,
        entity_type: WorkflowEntityType,
        tenant_id: i64,
        created_by: i64,
    ) -> Result<WorkflowInstance, ApiError>;

    /// Find an instance by ID
    async fn find_instance_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<WorkflowInstance>, ApiError>;

    /// Update a workflow instance
    async fn update_instance(
        &self,
        instance: WorkflowInstance,
    ) -> Result<WorkflowInstance, ApiError>;

    /// List instances by optional status
    async fn find_instances(
        &self,
        tenant_id: i64,
        status: Option<WorkflowStatus>,
    ) -> Result<Vec<WorkflowInstance>, ApiError>;

    // Steps

    /// Create a workflow step
    async fn create_step(&self, step: WorkflowStep) -> Result<WorkflowStep, ApiError>;

    /// Find steps by instance ID
    async fn find_steps_by_instance(&self, instance_id: i64)
        -> Result<Vec<WorkflowStep>, ApiError>;

    /// Update a workflow step
    async fn update_step(&self, step: WorkflowStep) -> Result<WorkflowStep, ApiError>;

    // Audit log

    /// Create an audit log entry
    async fn create_audit_log(&self, log: WorkflowAuditLog) -> Result<WorkflowAuditLog, ApiError>;

    /// Find audit logs by instance ID
    async fn find_audit_logs_by_instance(
        &self,
        instance_id: i64,
    ) -> Result<Vec<WorkflowAuditLog>, ApiError>;

    // Queries

    /// Find pending approvals for a specific user
    async fn find_pending_approvals_for_user(
        &self,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<WorkflowInstance>, ApiError>;

    /// Find overdue steps (pending for more than given hours)
    async fn find_overdue_steps(
        &self,
        hours: i64,
    ) -> Result<Vec<(WorkflowInstance, WorkflowStep)>, ApiError>;

    // Role-based assignment

    /// Find user IDs assigned to a role within a tenant
    async fn find_users_by_role(&self, tenant_id: i64, role: &str) -> Result<Vec<i64>, ApiError>;

    /// Find pending workflow instances for users with a given role
    async fn find_pending_approvals_by_role(
        &self,
        role: String,
        tenant_id: i64,
    ) -> Result<Vec<WorkflowInstance>, ApiError>;

    // Parallel approval tracking

    /// Create a step approval record
    async fn create_step_approval(
        &self,
        approval: WorkflowStepApproval,
    ) -> Result<WorkflowStepApproval, ApiError>;

    /// Find all approval records for a step
    async fn find_step_approvals(
        &self,
        step_id: i64,
    ) -> Result<Vec<WorkflowStepApproval>, ApiError>;
}

/// Type alias for boxed WorkflowRepository
pub type BoxWorkflowRepository = Arc<dyn WorkflowRepository>;

// ---------------------------------------------------------------------------
// InMemoryWorkflowRepository
// ---------------------------------------------------------------------------

struct Inner {
    templates: HashMap<i64, WorkflowTemplate>,
    instances: HashMap<i64, WorkflowInstance>,
    steps: HashMap<i64, WorkflowStep>,
    audit_logs: HashMap<i64, WorkflowAuditLog>,
    step_approvals: HashMap<i64, WorkflowStepApproval>,
    next_template_id: AtomicI64,
    next_instance_id: AtomicI64,
    next_step_id: AtomicI64,
    next_log_id: AtomicI64,
    next_approval_id: AtomicI64,
}

/// In-memory workflow repository for testing and development
pub struct InMemoryWorkflowRepository {
    inner: Mutex<Inner>,
}

impl InMemoryWorkflowRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                templates: HashMap::new(),
                instances: HashMap::new(),
                steps: HashMap::new(),
                audit_logs: HashMap::new(),
                step_approvals: HashMap::new(),
                next_template_id: AtomicI64::new(1),
                next_instance_id: AtomicI64::new(1),
                next_step_id: AtomicI64::new(1),
                next_log_id: AtomicI64::new(1),
                next_approval_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryWorkflowRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowRepository for InMemoryWorkflowRepository {
    async fn create_template(
        &self,
        create: CreateWorkflowTemplate,
        tenant_id: i64,
    ) -> Result<WorkflowTemplate, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_template_id.fetch_add(1, Ordering::SeqCst);
        let template = WorkflowTemplate {
            id,
            tenant_id,
            name: create.name,
            description: create.description,
            entity_type: create.entity_type,
            config_json: create.config_json,
            is_active: true,
            created_at: chrono::Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };
        inner.templates.insert(id, template.clone());
        Ok(template)
    }

    async fn find_template_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<WorkflowTemplate>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .templates
            .get(&id)
            .filter(|t| t.tenant_id == tenant_id && !t.is_deleted())
            .cloned())
    }

    async fn find_templates(&self, tenant_id: i64) -> Result<Vec<WorkflowTemplate>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .templates
            .values()
            .filter(|t| t.tenant_id == tenant_id && !t.is_deleted())
            .cloned()
            .collect())
    }

    async fn delete_template(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let template = inner
            .templates
            .get_mut(&id)
            .filter(|t| t.tenant_id == tenant_id && !t.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Template {} not found", id)))?;
        template.mark_deleted(0);
        Ok(())
    }

    async fn create_instance(
        &self,
        template_id: i64,
        entity_id: i64,
        entity_type: WorkflowEntityType,
        tenant_id: i64,
        created_by: i64,
    ) -> Result<WorkflowInstance, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_instance_id.fetch_add(1, Ordering::SeqCst);
        let instance = WorkflowInstance {
            id,
            tenant_id,
            template_id,
            entity_id,
            entity_type,
            status: WorkflowStatus::Pending,
            current_step: 1,
            assigned_user_id: None,
            created_by,
            created_at: chrono::Utc::now(),
            completed_at: None,
        };
        inner.instances.insert(id, instance.clone());
        Ok(instance)
    }

    async fn find_instance_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<WorkflowInstance>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .instances
            .get(&id)
            .filter(|i| i.tenant_id == tenant_id)
            .cloned())
    }

    async fn update_instance(
        &self,
        instance: WorkflowInstance,
    ) -> Result<WorkflowInstance, ApiError> {
        let mut inner = self.inner.lock();
        let exists = inner
            .instances
            .get(&instance.id)
            .filter(|i| i.tenant_id == instance.tenant_id)
            .is_some();
        if !exists {
            return Err(ApiError::NotFound(format!(
                "Instance {} not found",
                instance.id
            )));
        }
        inner.instances.insert(instance.id, instance.clone());
        Ok(instance)
    }

    async fn find_instances(
        &self,
        tenant_id: i64,
        status: Option<WorkflowStatus>,
    ) -> Result<Vec<WorkflowInstance>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .instances
            .values()
            .filter(|i| i.tenant_id == tenant_id)
            .filter(|i| status.as_ref().is_none_or(|s| i.status == *s))
            .cloned()
            .collect())
    }

    async fn create_step(&self, step: WorkflowStep) -> Result<WorkflowStep, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_step_id.fetch_add(1, Ordering::SeqCst);
        let step = WorkflowStep { id, ..step };
        inner.steps.insert(id, step.clone());
        Ok(step)
    }

    async fn find_steps_by_instance(
        &self,
        instance_id: i64,
    ) -> Result<Vec<WorkflowStep>, ApiError> {
        let inner = self.inner.lock();
        let mut steps: Vec<WorkflowStep> = inner
            .steps
            .values()
            .filter(|s| s.instance_id == instance_id)
            .cloned()
            .collect();
        steps.sort_by_key(|s| s.step_number);
        Ok(steps)
    }

    async fn update_step(&self, step: WorkflowStep) -> Result<WorkflowStep, ApiError> {
        let mut inner = self.inner.lock();
        let exists = inner.steps.contains_key(&step.id);
        if !exists {
            return Err(ApiError::NotFound(format!("Step {} not found", step.id)));
        }
        inner.steps.insert(step.id, step.clone());
        Ok(step)
    }

    async fn create_audit_log(&self, log: WorkflowAuditLog) -> Result<WorkflowAuditLog, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_log_id.fetch_add(1, Ordering::SeqCst);
        let log = WorkflowAuditLog { id, ..log };
        inner.audit_logs.insert(id, log.clone());
        Ok(log)
    }

    async fn find_audit_logs_by_instance(
        &self,
        instance_id: i64,
    ) -> Result<Vec<WorkflowAuditLog>, ApiError> {
        let inner = self.inner.lock();
        let mut logs: Vec<WorkflowAuditLog> = inner
            .audit_logs
            .values()
            .filter(|l| l.instance_id == instance_id)
            .cloned()
            .collect();
        logs.sort_by_key(|l| l.timestamp);
        Ok(logs)
    }

    async fn find_pending_approvals_for_user(
        &self,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<WorkflowInstance>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .instances
            .values()
            .filter(|i| {
                i.tenant_id == tenant_id
                    && i.status == WorkflowStatus::Pending
                    && i.assigned_user_id == Some(user_id)
            })
            .cloned()
            .collect())
    }

    async fn find_overdue_steps(
        &self,
        hours: i64,
    ) -> Result<Vec<(WorkflowInstance, WorkflowStep)>, ApiError> {
        let inner = self.inner.lock();
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
        let mut result = Vec::new();
        for step in inner.steps.values() {
            if step.status != WorkflowStepStatus::Pending || step.created_at > cutoff {
                continue;
            }
            if let Some(instance) = inner.instances.get(&step.instance_id) {
                if instance.status == WorkflowStatus::Pending {
                    result.push((instance.clone(), step.clone()));
                }
            }
        }
        Ok(result)
    }

    async fn find_users_by_role(&self, _tenant_id: i64, _role: &str) -> Result<Vec<i64>, ApiError> {
        // Stub: no user/role domain integration in this repository
        Ok(Vec::new())
    }

    async fn find_pending_approvals_by_role(
        &self,
        role: String,
        tenant_id: i64,
    ) -> Result<Vec<WorkflowInstance>, ApiError> {
        let inner = self.inner.lock();
        let mut result = Vec::new();
        for instance in inner.instances.values() {
            if instance.tenant_id != tenant_id || instance.status != WorkflowStatus::Pending {
                continue;
            }
            let steps: Vec<&WorkflowStep> = inner
                .steps
                .values()
                .filter(|s| s.instance_id == instance.id && s.status == WorkflowStepStatus::Pending)
                .collect();
            if steps
                .iter()
                .any(|s| s.approver_role.as_deref() == Some(&role))
            {
                result.push(instance.clone());
            }
        }
        Ok(result)
    }

    async fn create_step_approval(
        &self,
        approval: WorkflowStepApproval,
    ) -> Result<WorkflowStepApproval, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_approval_id.fetch_add(1, Ordering::SeqCst);
        let approval = WorkflowStepApproval { id, ..approval };
        inner.step_approvals.insert(id, approval.clone());
        Ok(approval)
    }

    async fn find_step_approvals(
        &self,
        step_id: i64,
    ) -> Result<Vec<WorkflowStepApproval>, ApiError> {
        let inner = self.inner.lock();
        let mut approvals: Vec<WorkflowStepApproval> = inner
            .step_approvals
            .values()
            .filter(|a| a.step_id == step_id)
            .cloned()
            .collect();
        approvals.sort_by_key(|a| a.created_at);
        Ok(approvals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_template_crud() {
        let repo = InMemoryWorkflowRepository::new();
        let create = CreateWorkflowTemplate {
            name: "Test".to_string(),
            description: "Desc".to_string(),
            entity_type: WorkflowEntityType::Invoice,
            config_json: serde_json::json!({}),
        };
        let template = repo.create_template(create, 1).await.unwrap();
        assert_eq!(template.id, 1);

        let found = repo.find_template_by_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.name, "Test");

        let list = repo.find_templates(1).await.unwrap();
        assert_eq!(list.len(), 1);

        repo.delete_template(1, 1).await.unwrap();
        assert!(repo.find_template_by_id(1, 1).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_instance_lifecycle() {
        let repo = InMemoryWorkflowRepository::new();
        let instance = repo
            .create_instance(1, 100, WorkflowEntityType::Invoice, 1, 5)
            .await
            .unwrap();
        assert_eq!(instance.status, WorkflowStatus::Pending);

        let found = repo
            .find_instance_by_id(instance.id, 1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.entity_id, 100);

        let mut updated = found;
        updated.status = WorkflowStatus::Completed;
        repo.update_instance(updated).await.unwrap();

        let found = repo
            .find_instance_by_id(instance.id, 1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn test_step_and_audit() {
        let repo = InMemoryWorkflowRepository::new();
        let step = repo
            .create_step(WorkflowStep {
                id: 0,
                instance_id: 1,
                step_number: 1,
                step_name: "Review".to_string(),
                approver_role: Some("manager".to_string()),
                approver_user_id: None,
                status: WorkflowStepStatus::Pending,
                comment: None,
                created_at: chrono::Utc::now(),
                completed_at: None,
            })
            .await
            .unwrap();
        assert_eq!(step.id, 1);

        let steps = repo.find_steps_by_instance(1).await.unwrap();
        assert_eq!(steps.len(), 1);

        let log = repo
            .create_audit_log(WorkflowAuditLog {
                id: 0,
                instance_id: 1,
                step_id: Some(step.id),
                action: "approve".to_string(),
                user_id: 5,
                comment: Some("Looks good".to_string()),
                timestamp: chrono::Utc::now(),
            })
            .await
            .unwrap();
        assert_eq!(log.id, 1);

        let logs = repo.find_audit_logs_by_instance(1).await.unwrap();
        assert_eq!(logs.len(), 1);
    }
}
