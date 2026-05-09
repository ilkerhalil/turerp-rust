//! Workflow service — business logic for approval workflows

use crate::common::{
    BoxJobScheduler, BoxNotificationService, CreateJob, JobPriority, JobType, NotificationChannel,
    NotificationPriority, NotificationRequest,
};
use crate::domain::workflow::model::{
    CreateWorkflowTemplate, WorkflowAuditLog, WorkflowEntityType, WorkflowInstance,
    WorkflowInstanceDetailResponse, WorkflowStatus, WorkflowStep, WorkflowStepStatus,
    WorkflowTemplate,
};
use crate::domain::workflow::repository::BoxWorkflowRepository;
use crate::error::ApiError;

/// Service for managing workflow templates and instances
#[derive(Clone)]
pub struct WorkflowService {
    repo: BoxWorkflowRepository,
    notification_service: BoxNotificationService,
    job_scheduler: BoxJobScheduler,
}

impl WorkflowService {
    pub fn new(
        repo: BoxWorkflowRepository,
        notification_service: BoxNotificationService,
        job_scheduler: BoxJobScheduler,
    ) -> Self {
        Self {
            repo,
            notification_service,
            job_scheduler,
        }
    }

    // ---- Pre-built templates ----

    /// Create a 2-step purchase order approval template: manager -> admin
    pub async fn create_purchase_order_approval_template(
        &self,
        tenant_id: i64,
    ) -> Result<WorkflowTemplate, ApiError> {
        let config = serde_json::json!({
            "steps": [
                {"step_number": 1, "step_name": "Manager Review", "approver_role": "manager"},
                {"step_number": 2, "step_name": "Admin Approval", "approver_role": "admin"}
            ]
        });
        self.create_template(
            CreateWorkflowTemplate {
                name: "Purchase Order Approval".to_string(),
                description: "Standard 2-step purchase order approval workflow".to_string(),
                entity_type: WorkflowEntityType::PurchaseOrder,
                config_json: config,
            },
            tenant_id,
        )
        .await
    }

    /// Create a 3-step expense approval template: manager -> finance -> admin
    pub async fn create_expense_approval_template(
        &self,
        tenant_id: i64,
    ) -> Result<WorkflowTemplate, ApiError> {
        let config = serde_json::json!({
            "steps": [
                {"step_number": 1, "step_name": "Manager Review", "approver_role": "manager"},
                {"step_number": 2, "step_name": "Finance Review", "approver_role": "finance"},
                {"step_number": 3, "step_name": "Admin Approval", "approver_role": "admin"}
            ]
        });
        self.create_template(
            CreateWorkflowTemplate {
                name: "Expense Approval".to_string(),
                description: "Standard 3-step expense approval workflow".to_string(),
                entity_type: WorkflowEntityType::Expense,
                config_json: config,
            },
            tenant_id,
        )
        .await
    }

    /// Create a 2-step invoice verification template: accountant -> admin
    pub async fn create_invoice_verification_template(
        &self,
        tenant_id: i64,
    ) -> Result<WorkflowTemplate, ApiError> {
        let config = serde_json::json!({
            "steps": [
                {"step_number": 1, "step_name": "Accountant Verification", "approver_role": "accountant"},
                {"step_number": 2, "step_name": "Admin Approval", "approver_role": "admin"}
            ]
        });
        self.create_template(
            CreateWorkflowTemplate {
                name: "Invoice Verification".to_string(),
                description: "Standard 2-step invoice verification workflow".to_string(),
                entity_type: WorkflowEntityType::Invoice,
                config_json: config,
            },
            tenant_id,
        )
        .await
    }

    /// Create a 2-step stock transfer approval template: warehouse -> admin
    pub async fn create_stock_transfer_approval_template(
        &self,
        tenant_id: i64,
    ) -> Result<WorkflowTemplate, ApiError> {
        let config = serde_json::json!({
            "steps": [
                {"step_number": 1, "step_name": "Warehouse Review", "approver_role": "warehouse"},
                {"step_number": 2, "step_name": "Admin Approval", "approver_role": "admin"}
            ]
        });
        self.create_template(
            CreateWorkflowTemplate {
                name: "Stock Transfer Approval".to_string(),
                description: "Standard 2-step stock transfer approval workflow".to_string(),
                entity_type: WorkflowEntityType::StockTransfer,
                config_json: config,
            },
            tenant_id,
        )
        .await
    }

    // ---- Template operations ----

    /// Create a new workflow template
    pub async fn create_template(
        &self,
        create: CreateWorkflowTemplate,
        tenant_id: i64,
    ) -> Result<WorkflowTemplate, ApiError> {
        self.repo.create_template(create, tenant_id).await
    }

    /// List all templates for a tenant
    pub async fn list_templates(&self, tenant_id: i64) -> Result<Vec<WorkflowTemplate>, ApiError> {
        self.repo.find_templates(tenant_id).await
    }

    // ---- Workflow execution ----

    /// Start a new workflow instance from a template
    pub async fn start_workflow(
        &self,
        template_id: i64,
        entity_id: i64,
        entity_type: WorkflowEntityType,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<WorkflowInstance, ApiError> {
        let template = self
            .repo
            .find_template_by_id(template_id, tenant_id)
            .await?
            .ok_or_else(|| {
                tracing::warn!(tenant_id, template_id, "Workflow template not found");
                ApiError::NotFound(format!("Template {} not found", template_id))
            })?;

        if template.entity_type != entity_type {
            tracing::warn!(tenant_id, template_id, "Workflow entity type mismatch");
            return Err(ApiError::Validation(format!(
                "Template entity type {:?} does not match requested {:?}",
                template.entity_type, entity_type
            )));
        }

        let instance = self
            .repo
            .create_instance(
                template_id,
                entity_id,
                entity_type.clone(),
                tenant_id,
                user_id,
            )
            .await?;

        // Create steps from template config
        if let Some(steps) = template.config_json.get("steps").and_then(|s| s.as_array()) {
            for step_def in steps {
                let step_number = step_def
                    .get("step_number")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32;
                let step_name = step_def
                    .get("step_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Review")
                    .to_string();
                let approver_role = step_def
                    .get("approver_role")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let approver_user_id = step_def.get("approver_user_id").and_then(|v| v.as_i64());

                self.repo
                    .create_step(WorkflowStep {
                        id: 0,
                        instance_id: instance.id,
                        step_number,
                        step_name,
                        approver_role,
                        approver_user_id,
                        status: WorkflowStepStatus::Pending,
                        comment: None,
                        created_at: chrono::Utc::now(),
                        completed_at: None,
                    })
                    .await?;
            }
        }

        // Set initial assigned user from first step
        let steps = self.repo.find_steps_by_instance(instance.id).await?;
        if let Some(first_step) = steps.iter().min_by_key(|s| s.step_number) {
            let mut instance = instance;
            instance.assigned_user_id = first_step.approver_user_id;
            let instance = self.repo.update_instance(instance).await?;

            // Notify approver
            if let Some(approver_id) = first_step.approver_user_id {
                let _ = self
                    .send_notification(
                        tenant_id,
                        approver_id,
                        "workflow_step_assigned",
                        serde_json::json!({
                            "workflow_id": instance.id,
                            "step_name": first_step.step_name,
                            "entity_type": entity_type.to_string(),
                            "entity_id": entity_id
                        }),
                    )
                    .await;
            }

            // Audit log
            self.repo
                .create_audit_log(WorkflowAuditLog {
                    id: 0,
                    instance_id: instance.id,
                    step_id: None,
                    action: "start".to_string(),
                    user_id,
                    comment: None,
                    timestamp: chrono::Utc::now(),
                })
                .await?;

            // Schedule escalation check job
            let _ = self.schedule_escalation_job(instance.id, tenant_id).await;
            tracing::info!(tenant_id, instance_id = instance.id, "Started workflow");

            return Ok(instance);
        }

        Ok(instance)
    }

    /// Approve the current step and advance to next or complete
    pub async fn approve_step(
        &self,
        instance_id: i64,
        step_id: i64,
        user_id: i64,
        comment: Option<String>,
        tenant_id: i64,
    ) -> Result<WorkflowInstance, ApiError> {
        let instance = self
            .repo
            .find_instance_by_id(instance_id, tenant_id)
            .await?
            .ok_or_else(|| {
                tracing::warn!(tenant_id, instance_id, "Workflow instance not found");
                ApiError::NotFound(format!("Instance {} not found", instance_id))
            })?;

        if instance.status != WorkflowStatus::Pending {
            tracing::warn!(
                tenant_id,
                instance_id,
                "Workflow instance not pending for approval"
            );
            return Err(ApiError::BadRequest(format!(
                "Instance {} is not pending",
                instance_id
            )));
        }

        let mut step = self
            .repo
            .find_steps_by_instance(instance_id)
            .await?
            .into_iter()
            .find(|s| s.id == step_id)
            .ok_or_else(|| ApiError::NotFound(format!("Step {} not found", step_id)))?;

        if step.status != WorkflowStepStatus::Pending {
            return Err(ApiError::BadRequest(format!(
                "Step {} is not pending",
                step_id
            )));
        }

        step.status = WorkflowStepStatus::Approved;
        step.comment = comment.clone();
        step.completed_at = Some(chrono::Utc::now());
        self.repo.update_step(step.clone()).await?;

        // Audit log
        self.repo
            .create_audit_log(WorkflowAuditLog {
                id: 0,
                instance_id,
                step_id: Some(step_id),
                action: "approve".to_string(),
                user_id,
                comment: comment.clone(),
                timestamp: chrono::Utc::now(),
            })
            .await?;

        // Notify initiator
        let _ = self
            .send_notification(
                tenant_id,
                instance.created_by,
                "workflow_step_approved",
                serde_json::json!({
                    "workflow_id": instance_id,
                    "step_name": step.step_name,
                    "approved_by": user_id
                }),
            )
            .await;

        // Advance or complete
        let steps = self.repo.find_steps_by_instance(instance_id).await?;
        let max_step = steps.iter().map(|s| s.step_number).max().unwrap_or(1);

        let mut instance = instance;
        if instance.current_step >= max_step {
            instance.status = WorkflowStatus::Completed;
            instance.completed_at = Some(chrono::Utc::now());
            instance.assigned_user_id = None;

            // Notify initiator of completion
            let _ = self
                .send_notification(
                    tenant_id,
                    instance.created_by,
                    "workflow_completed",
                    serde_json::json!({
                        "workflow_id": instance_id,
                        "entity_type": instance.entity_type.to_string(),
                        "entity_id": instance.entity_id
                    }),
                )
                .await;
        } else {
            instance.current_step += 1;
            let next_step = steps
                .iter()
                .find(|s| s.step_number == instance.current_step);
            instance.assigned_user_id = next_step.and_then(|s| s.approver_user_id);

            // Notify next approver
            if let Some(next) = next_step {
                if let Some(approver_id) = next.approver_user_id {
                    let _ = self
                        .send_notification(
                            tenant_id,
                            approver_id,
                            "workflow_step_assigned",
                            serde_json::json!({
                                "workflow_id": instance_id,
                                "step_name": next.step_name,
                                "entity_type": instance.entity_type.to_string(),
                                "entity_id": instance.entity_id
                            }),
                        )
                        .await;
                }
            }
        }

        let instance = self.repo.update_instance(instance.clone()).await?;
        tracing::info!(tenant_id, instance_id, step_id, "Approved workflow step");
        Ok(instance)
    }

    /// Reject the current step and mark instance as rejected
    pub async fn reject_step(
        &self,
        instance_id: i64,
        step_id: i64,
        user_id: i64,
        comment: Option<String>,
        tenant_id: i64,
    ) -> Result<WorkflowInstance, ApiError> {
        let instance = self
            .repo
            .find_instance_by_id(instance_id, tenant_id)
            .await?
            .ok_or_else(|| {
                tracing::warn!(tenant_id, instance_id, "Workflow instance not found");
                ApiError::NotFound(format!("Instance {} not found", instance_id))
            })?;

        if instance.status != WorkflowStatus::Pending {
            tracing::warn!(
                tenant_id,
                instance_id,
                "Workflow instance not pending for rejection"
            );
            return Err(ApiError::BadRequest(format!(
                "Instance {} is not pending",
                instance_id
            )));
        }

        let mut step = self
            .repo
            .find_steps_by_instance(instance_id)
            .await?
            .into_iter()
            .find(|s| s.id == step_id)
            .ok_or_else(|| {
                tracing::warn!(tenant_id, step_id, "Workflow step not found");
                ApiError::NotFound(format!("Step {} not found", step_id))
            })?;

        if step.status != WorkflowStepStatus::Pending {
            tracing::warn!(
                tenant_id,
                step_id,
                "Workflow step not pending for rejection"
            );
            return Err(ApiError::BadRequest(format!(
                "Step {} is not pending",
                step_id
            )));
        }

        step.status = WorkflowStepStatus::Rejected;
        step.comment = comment.clone();
        step.completed_at = Some(chrono::Utc::now());
        self.repo.update_step(step.clone()).await?;

        // Audit log
        self.repo
            .create_audit_log(WorkflowAuditLog {
                id: 0,
                instance_id,
                step_id: Some(step_id),
                action: "reject".to_string(),
                user_id,
                comment: comment.clone(),
                timestamp: chrono::Utc::now(),
            })
            .await?;

        // Notify initiator
        let _ = self
            .send_notification(
                tenant_id,
                instance.created_by,
                "workflow_rejected",
                serde_json::json!({
                    "workflow_id": instance_id,
                    "step_name": step.step_name,
                    "rejected_by": user_id,
                    "comment": comment.clone().unwrap_or_default()
                }),
            )
            .await;

        let mut instance = instance;
        instance.status = WorkflowStatus::Rejected;
        instance.assigned_user_id = None;
        let instance = self.repo.update_instance(instance.clone()).await?;
        tracing::info!(tenant_id, instance_id, step_id, "Rejected workflow step");
        Ok(instance)
    }

    /// Resubmit a rejected workflow back to pending
    pub async fn resubmit(
        &self,
        instance_id: i64,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<WorkflowInstance, ApiError> {
        let instance = self
            .repo
            .find_instance_by_id(instance_id, tenant_id)
            .await?
            .ok_or_else(|| {
                tracing::warn!(tenant_id, instance_id, "Workflow instance not found");
                ApiError::NotFound(format!("Instance {} not found", instance_id))
            })?;

        if instance.status != WorkflowStatus::Rejected {
            tracing::warn!(
                tenant_id,
                instance_id,
                "Workflow instance not rejected for resubmit"
            );
            return Err(ApiError::BadRequest(format!(
                "Instance {} is not rejected",
                instance_id
            )));
        }

        // Reset all steps to pending
        let steps = self.repo.find_steps_by_instance(instance_id).await?;
        for mut step in steps {
            step.status = WorkflowStepStatus::Pending;
            step.comment = None;
            step.completed_at = None;
            self.repo.update_step(step).await?;
        }

        // Audit log
        self.repo
            .create_audit_log(WorkflowAuditLog {
                id: 0,
                instance_id,
                step_id: None,
                action: "resubmit".to_string(),
                user_id,
                comment: None,
                timestamp: chrono::Utc::now(),
            })
            .await?;

        let mut instance = instance;
        instance.status = WorkflowStatus::Pending;
        instance.current_step = 1;
        instance.completed_at = None;

        let first_step = self
            .repo
            .find_steps_by_instance(instance_id)
            .await?
            .into_iter()
            .min_by_key(|s| s.step_number);
        instance.assigned_user_id = first_step.as_ref().and_then(|s| s.approver_user_id);

        let instance = self.repo.update_instance(instance.clone()).await?;
        tracing::info!(tenant_id, instance_id, "Resubmitted workflow");

        // Notify first approver
        if let Some(step) = first_step {
            if let Some(approver_id) = step.approver_user_id {
                let _ = self
                    .send_notification(
                        tenant_id,
                        approver_id,
                        "workflow_step_assigned",
                        serde_json::json!({
                            "workflow_id": instance_id,
                            "step_name": step.step_name,
                            "entity_type": instance.entity_type.to_string(),
                            "entity_id": instance.entity_id
                        }),
                    )
                    .await;
            }
        }

        Ok(instance)
    }

    /// Get pending approvals for a user
    pub async fn get_pending_approvals(
        &self,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<WorkflowInstance>, ApiError> {
        self.repo
            .find_pending_approvals_for_user(user_id, tenant_id)
            .await
    }

    /// Escalate overdue steps (pending > 24h)
    pub async fn escalate_overdue(&self) -> Result<Vec<WorkflowInstance>, ApiError> {
        let overdue = self.repo.find_overdue_steps(24).await?;
        let mut escalated = Vec::new();
        tracing::info!(
            overdue_count = overdue.len(),
            "Escalating overdue workflows"
        );
        for (instance, step) in overdue {
            // Audit log escalation
            let _ = self
                .repo
                .create_audit_log(WorkflowAuditLog {
                    id: 0,
                    instance_id: instance.id,
                    step_id: Some(step.id),
                    action: "escalate".to_string(),
                    user_id: 0,
                    comment: Some(format!("Step '{}' pending for >24h", step.step_name)),
                    timestamp: chrono::Utc::now(),
                })
                .await;

            // Notify admin about escalation
            let _ = self
                .send_notification(
                    instance.tenant_id,
                    instance.created_by,
                    "workflow_escalated",
                    serde_json::json!({
                        "workflow_id": instance.id,
                        "step_name": step.step_name,
                        "entity_type": instance.entity_type.to_string(),
                        "entity_id": instance.entity_id
                    }),
                )
                .await;

            escalated.push(instance);
        }
        Ok(escalated)
    }

    // ---- Read operations ----

    /// Get a workflow instance by ID
    pub async fn get_instance(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<WorkflowInstanceDetailResponse, ApiError> {
        let instance = self
            .repo
            .find_instance_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| {
                tracing::warn!(tenant_id, instance_id = id, "Workflow instance not found");
                ApiError::NotFound(format!("Instance {} not found", id))
            })?;
        let steps = self
            .repo
            .find_steps_by_instance(id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        let audit_log = self
            .repo
            .find_audit_logs_by_instance(id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        Ok(WorkflowInstanceDetailResponse {
            instance: instance.into(),
            steps,
            audit_log,
        })
    }

    /// Get audit trail for an instance
    pub async fn get_instance_audit_log(
        &self,
        instance_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::workflow::model::WorkflowAuditLogResponse>, ApiError> {
        // Verify instance exists and belongs to tenant
        let _ = self
            .repo
            .find_instance_by_id(instance_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Instance {} not found", instance_id)))?;
        let logs = self.repo.find_audit_logs_by_instance(instance_id).await?;
        Ok(logs.into_iter().map(Into::into).collect())
    }

    // ---- Helpers ----

    async fn send_notification(
        &self,
        tenant_id: i64,
        user_id: i64,
        template_key: &str,
        template_vars: serde_json::Value,
    ) {
        let req = NotificationRequest {
            tenant_id,
            user_id: Some(user_id),
            channel: NotificationChannel::InApp,
            priority: NotificationPriority::High,
            template_key: template_key.to_string(),
            template_vars,
            recipient: format!("user:{}", user_id),
        };
        if let Err(e) = self.notification_service.send(req).await {
            tracing::warn!("Failed to send workflow notification: {}", e);
        }
    }

    async fn schedule_escalation_job(
        &self,
        instance_id: i64,
        tenant_id: i64,
    ) -> Result<(), ApiError> {
        let job = CreateJob::new(
            JobType::Custom {
                name: "workflow_escalation".to_string(),
                payload: format!("{},{}", instance_id, tenant_id),
            },
            tenant_id,
        )
        .with_priority(JobPriority::Normal)
        .with_scheduled_at(chrono::Utc::now() + chrono::Duration::hours(24));

        self.job_scheduler
            .schedule(job)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to schedule escalation job: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{InMemoryJobScheduler, InMemoryNotificationService};
    use crate::domain::workflow::repository::InMemoryWorkflowRepository;
    use std::sync::Arc;

    fn make_service() -> WorkflowService {
        let repo = Arc::new(InMemoryWorkflowRepository::new());
        let notif = Arc::new(InMemoryNotificationService::new()) as BoxNotificationService;
        let jobs = Arc::new(InMemoryJobScheduler::new()) as BoxJobScheduler;
        WorkflowService::new(repo, notif, jobs)
    }

    #[tokio::test]
    async fn test_start_and_approve_workflow() {
        let svc = make_service();

        // Create template
        let template = svc
            .create_purchase_order_approval_template(1)
            .await
            .unwrap();
        assert_eq!(template.entity_type, WorkflowEntityType::PurchaseOrder);

        // Start workflow
        let instance = svc
            .start_workflow(template.id, 100, WorkflowEntityType::PurchaseOrder, 5, 1)
            .await
            .unwrap();
        assert_eq!(instance.status, WorkflowStatus::Pending);
        assert_eq!(instance.current_step, 1);

        // Get steps
        let detail = svc.get_instance(instance.id, 1).await.unwrap();
        assert_eq!(detail.steps.len(), 2);

        let step1 = detail.steps.iter().find(|s| s.step_number == 1).unwrap();

        // Approve step 1
        let instance = svc
            .approve_step(instance.id, step1.id, 5, Some("Looks good".to_string()), 1)
            .await
            .unwrap();
        assert_eq!(instance.current_step, 2);
        assert_eq!(instance.status, WorkflowStatus::Pending);

        // Approve step 2
        let detail = svc.get_instance(instance.id, 1).await.unwrap();
        let step2 = detail.steps.iter().find(|s| s.step_number == 2).unwrap();

        let instance = svc
            .approve_step(instance.id, step2.id, 5, None, 1)
            .await
            .unwrap();
        assert_eq!(instance.status, WorkflowStatus::Completed);
        assert!(instance.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_start_reject_resubmit() {
        let svc = make_service();

        let template = svc.create_expense_approval_template(1).await.unwrap();
        let instance = svc
            .start_workflow(template.id, 200, WorkflowEntityType::Expense, 5, 1)
            .await
            .unwrap();

        let detail = svc.get_instance(instance.id, 1).await.unwrap();
        let step1 = detail.steps.iter().find(|s| s.step_number == 1).unwrap();

        // Reject step 1
        let instance = svc
            .reject_step(
                instance.id,
                step1.id,
                5,
                Some("Missing receipt".to_string()),
                1,
            )
            .await
            .unwrap();
        assert_eq!(instance.status, WorkflowStatus::Rejected);

        // Resubmit
        let instance = svc.resubmit(instance.id, 5, 1).await.unwrap();
        assert_eq!(instance.status, WorkflowStatus::Pending);
        assert_eq!(instance.current_step, 1);

        // Steps should be reset
        let detail = svc.get_instance(instance.id, 1).await.unwrap();
        for step in &detail.steps {
            assert_eq!(step.status, WorkflowStepStatus::Pending);
        }
    }

    #[tokio::test]
    async fn test_get_pending_approvals() {
        let svc = make_service();

        let template = svc.create_invoice_verification_template(1).await.unwrap();
        let instance = svc
            .start_workflow(template.id, 300, WorkflowEntityType::Invoice, 5, 1)
            .await
            .unwrap();

        // No assigned user_id on steps, so pending approvals should be empty
        let pending = svc.get_pending_approvals(5, 1).await.unwrap();
        assert_eq!(pending.len(), 0);

        // Manually assign
        let mut inst = instance;
        inst.assigned_user_id = Some(5);
        svc.repo.update_instance(inst).await.unwrap();

        let pending = svc.get_pending_approvals(5, 1).await.unwrap();
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_prebuilt_templates() {
        let svc = make_service();

        let po = svc
            .create_purchase_order_approval_template(1)
            .await
            .unwrap();
        assert!(
            po.config_json
                .get("steps")
                .unwrap()
                .as_array()
                .unwrap()
                .len()
                == 2
        );

        let exp = svc.create_expense_approval_template(1).await.unwrap();
        assert!(
            exp.config_json
                .get("steps")
                .unwrap()
                .as_array()
                .unwrap()
                .len()
                == 3
        );

        let inv = svc.create_invoice_verification_template(1).await.unwrap();
        assert!(
            inv.config_json
                .get("steps")
                .unwrap()
                .as_array()
                .unwrap()
                .len()
                == 2
        );

        let st = svc
            .create_stock_transfer_approval_template(1)
            .await
            .unwrap();
        assert!(
            st.config_json
                .get("steps")
                .unwrap()
                .as_array()
                .unwrap()
                .len()
                == 2
        );
    }
}
