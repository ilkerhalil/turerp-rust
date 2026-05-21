//! Workflow service — business logic for approval workflows

use crate::common::{
    BoxJobScheduler, BoxNotificationService, CreateJob, JobPriority, JobType, NotificationChannel,
    NotificationPriority, NotificationRequest,
};
use crate::domain::workflow::model::{
    Condition, CreateWorkflowTemplate, EscalationRule, ParallelConfig, ParallelMode,
    WorkflowAuditLog, WorkflowEntityType, WorkflowInstance, WorkflowInstanceDetailResponse,
    WorkflowStatus, WorkflowStep, WorkflowStepApproval, WorkflowStepStatus, WorkflowTemplate,
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
                let mut approver_user_id =
                    step_def.get("approver_user_id").and_then(|v| v.as_i64());

                // Role-based assignment: resolve role to user if no explicit user assigned
                if approver_user_id.is_none() {
                    if let Some(ref role) = approver_role {
                        let users = self.assign_by_role(tenant_id, role).await?;
                        if users.len() == 1 {
                            approver_user_id = Some(users[0]);
                        }
                    }
                }

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

        let step = self
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

        // Check if this step has parallel configuration from template
        let template = self
            .repo
            .find_template_by_id(instance.template_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Template {} not found", instance.template_id))
            })?;

        let parallel_config = template
            .config_json
            .get("steps")
            .and_then(|s| s.as_array())
            .and_then(|steps| {
                steps
                    .iter()
                    .find(|def| {
                        def.get("step_number")
                            .and_then(|v| v.as_i64())
                            .map(|n| n as i32 == step.step_number)
                            .unwrap_or(false)
                    })
                    .and_then(|def| def.get("parallel"))
                    .and_then(|p| serde_json::from_value::<ParallelConfig>(p.clone()).ok())
            });

        if let Some(ref config) = parallel_config {
            return self
                .process_parallel_approval(instance, step, user_id, comment, tenant_id, config)
                .await;
        }

        // Sequential approval path
        let mut step = step;
        step.status = WorkflowStepStatus::Approved;
        step.comment = comment.clone();
        step.completed_at = Some(chrono::Utc::now());
        self.repo.update_step(step.clone()).await?;

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

        self.advance_workflow(instance, step, tenant_id).await
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
            // Try to read escalation rules from template config
            let template = self
                .repo
                .find_template_by_id(instance.template_id, instance.tenant_id)
                .await
                .ok()
                .flatten();

            let escalation = template.and_then(|t| {
                t.config_json
                    .get("steps")
                    .and_then(|s| s.as_array())
                    .and_then(|steps| {
                        steps
                            .iter()
                            .find(|def| {
                                def.get("step_number")
                                    .and_then(|v| v.as_i64())
                                    .map(|n| n as i32 == step.step_number)
                                    .unwrap_or(false)
                            })
                            .and_then(|def| def.get("escalation"))
                            .and_then(|e| serde_json::from_value::<EscalationRule>(e.clone()).ok())
                    })
            });

            if let Some(ref rule) = escalation {
                let _ = self.check_escalation(&instance, &step, rule).await;
            } else {
                // Default escalation behavior
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
            }

            escalated.push(instance);
        }
        Ok(escalated)
    }

    // ---- Conditional routing ----

    /// Evaluate a list of conditions against entity data
    pub fn evaluate_conditions(conditions: &[Condition], entity_data: &serde_json::Value) -> bool {
        if conditions.is_empty() {
            return true;
        }
        conditions.iter().all(|c| c.evaluate(entity_data))
    }

    // ---- Role-based assignment ----

    /// Resolve a role to user IDs within a tenant
    pub async fn assign_by_role(&self, tenant_id: i64, role: &str) -> Result<Vec<i64>, ApiError> {
        self.repo.find_users_by_role(tenant_id, role).await
    }

    /// Get pending workflow instances for users with a given role
    pub async fn get_pending_approvals_by_role(
        &self,
        role: String,
        tenant_id: i64,
    ) -> Result<Vec<WorkflowInstance>, ApiError> {
        self.repo
            .find_pending_approvals_by_role(role, tenant_id)
            .await
    }

    // ---- Parallel approval ----

    /// Process an approval for a parallel step
    async fn process_parallel_approval(
        &self,
        instance: WorkflowInstance,
        step: WorkflowStep,
        user_id: i64,
        comment: Option<String>,
        tenant_id: i64,
        parallel_config: &ParallelConfig,
    ) -> Result<WorkflowInstance, ApiError> {
        // Check if user already responded
        let approvals = self.repo.find_step_approvals(step.id).await?;
        if approvals.iter().any(|a| a.user_id == user_id) {
            return Err(ApiError::BadRequest(format!(
                "User {} already responded to step {}",
                user_id, step.id
            )));
        }

        // Validate user is an assignee
        if !parallel_config.assignee_user_ids.is_empty()
            && !parallel_config.assignee_user_ids.contains(&user_id)
        {
            return Err(ApiError::Unauthorized(format!(
                "User {} is not an assignee for step {}",
                user_id, step.id
            )));
        }

        // Record approval
        let approval = WorkflowStepApproval {
            id: 0,
            step_id: step.id,
            user_id,
            status: WorkflowStepStatus::Approved,
            comment: comment.clone(),
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        };
        self.repo.create_step_approval(approval).await?;

        // Re-fetch approvals
        let approvals = self.repo.find_step_approvals(step.id).await?;

        let approved_users: std::collections::HashSet<i64> = approvals
            .iter()
            .filter(|a| a.status == WorkflowStepStatus::Approved)
            .map(|a| a.user_id)
            .collect();

        let step_complete = match parallel_config.mode {
            ParallelMode::AllRequired => {
                let required = &parallel_config.assignee_user_ids;
                if required.is_empty() {
                    false
                } else {
                    required.iter().all(|id| approved_users.contains(id))
                }
            }
            ParallelMode::AnyOne => !approved_users.is_empty(),
        };

        if step_complete {
            let mut step = step;
            step.status = WorkflowStepStatus::Approved;
            step.comment = comment.clone();
            step.completed_at = Some(chrono::Utc::now());
            self.repo.update_step(step.clone()).await?;

            self.repo
                .create_audit_log(WorkflowAuditLog {
                    id: 0,
                    instance_id: instance.id,
                    step_id: Some(step.id),
                    action: "approve".to_string(),
                    user_id,
                    comment: comment.clone(),
                    timestamp: chrono::Utc::now(),
                })
                .await?;

            let _ = self
                .send_notification(
                    tenant_id,
                    instance.created_by,
                    "workflow_step_approved",
                    serde_json::json!({
                        "workflow_id": instance.id,
                        "step_name": step.step_name,
                        "approved_by": user_id
                    }),
                )
                .await;

            self.advance_workflow(instance, step, tenant_id).await
        } else {
            self.repo
                .create_audit_log(WorkflowAuditLog {
                    id: 0,
                    instance_id: instance.id,
                    step_id: Some(step.id),
                    action: "partial_approve".to_string(),
                    user_id,
                    comment: comment.clone(),
                    timestamp: chrono::Utc::now(),
                })
                .await?;

            let _ = self
                .send_notification(
                    tenant_id,
                    instance.created_by,
                    "workflow_partial_approval",
                    serde_json::json!({
                        "workflow_id": instance.id,
                        "step_name": step.step_name,
                        "approved_by": user_id
                    }),
                )
                .await;

            Ok(instance)
        }
    }

    // ---- Escalation ----

    /// Check and execute escalation for a step
    pub async fn check_escalation(
        &self,
        instance: &WorkflowInstance,
        step: &WorkflowStep,
        escalation: &EscalationRule,
    ) -> Result<(), ApiError> {
        let elapsed_hours = (chrono::Utc::now() - step.created_at).num_hours();

        if elapsed_hours >= escalation.timeout_hours {
            self.repo
                .create_audit_log(WorkflowAuditLog {
                    id: 0,
                    instance_id: instance.id,
                    step_id: Some(step.id),
                    action: "escalate".to_string(),
                    user_id: 0,
                    comment: Some(format!(
                        "Step '{}' auto-escalated after {} hours",
                        step.step_name, escalation.timeout_hours
                    )),
                    timestamp: chrono::Utc::now(),
                })
                .await?;

            let _ = self
                .send_notification(
                    instance.tenant_id,
                    instance.created_by,
                    "workflow_escalated",
                    serde_json::json!({
                        "workflow_id": instance.id,
                        "step_name": step.step_name,
                        "entity_type": instance.entity_type.to_string(),
                        "entity_id": instance.entity_id,
                        "escalation_hours": escalation.timeout_hours
                    }),
                )
                .await;

            if escalation.escalate_to_manager {
                let _ = self
                    .send_notification(
                        instance.tenant_id,
                        instance.created_by,
                        "workflow_escalated_manager",
                        serde_json::json!({
                            "workflow_id": instance.id,
                            "step_name": step.step_name,
                            "entity_type": instance.entity_type.to_string(),
                            "entity_id": instance.entity_id
                        }),
                    )
                    .await;
            }

            if let Some(ref role) = escalation.escalate_to_role {
                let users = self
                    .repo
                    .find_users_by_role(instance.tenant_id, role)
                    .await?;
                for uid in users {
                    let _ = self
                        .send_notification(
                            instance.tenant_id,
                            uid,
                            "workflow_escalated_role",
                            serde_json::json!({
                                "workflow_id": instance.id,
                                "step_name": step.step_name,
                                "role": role
                            }),
                        )
                        .await;
                }
            }
        } else if elapsed_hours >= escalation.reminder_hours {
            if let Some(approver_id) = step.approver_user_id {
                let _ = self
                    .send_notification(
                        instance.tenant_id,
                        approver_id,
                        "workflow_reminder",
                        serde_json::json!({
                            "workflow_id": instance.id,
                            "step_name": step.step_name,
                            "hours_pending": elapsed_hours
                        }),
                    )
                    .await;
            }
        }

        Ok(())
    }

    // ---- Internal helpers ----

    /// Advance workflow to next step or mark as completed
    async fn advance_workflow(
        &self,
        mut instance: WorkflowInstance,
        _step: WorkflowStep,
        tenant_id: i64,
    ) -> Result<WorkflowInstance, ApiError> {
        let steps = self.repo.find_steps_by_instance(instance.id).await?;
        let max_step = steps.iter().map(|s| s.step_number).max().unwrap_or(1);

        if instance.current_step >= max_step {
            instance.status = WorkflowStatus::Completed;
            instance.completed_at = Some(chrono::Utc::now());
            instance.assigned_user_id = None;

            let _ = self
                .send_notification(
                    tenant_id,
                    instance.created_by,
                    "workflow_completed",
                    serde_json::json!({
                        "workflow_id": instance.id,
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

            if let Some(next) = next_step {
                if let Some(approver_id) = next.approver_user_id {
                    let _ = self
                        .send_notification(
                            tenant_id,
                            approver_id,
                            "workflow_step_assigned",
                            serde_json::json!({
                                "workflow_id": instance.id,
                                "step_name": next.step_name,
                                "entity_type": instance.entity_type.to_string(),
                                "entity_id": instance.entity_id
                            }),
                        )
                        .await;
                }
            }
        }

        self.repo.update_instance(instance.clone()).await
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
            tracing::warn!(error = %e, "Failed to send workflow notification");
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

    #[test]
    fn test_evaluate_conditions() {
        let entity = serde_json::json!({
            "amount": 1500,
            "department": "engineering",
            "tags": ["urgent", "budget"]
        });

        assert!(Condition {
            field: "amount".to_string(),
            operator: "gt".to_string(),
            value: serde_json::json!(1000),
        }
        .evaluate(&entity));

        assert!(!Condition {
            field: "amount".to_string(),
            operator: "lt".to_string(),
            value: serde_json::json!(1000),
        }
        .evaluate(&entity));

        assert!(Condition {
            field: "department".to_string(),
            operator: "eq".to_string(),
            value: serde_json::json!("engineering"),
        }
        .evaluate(&entity));

        assert!(Condition {
            field: "department".to_string(),
            operator: "contains".to_string(),
            value: serde_json::json!("engine"),
        }
        .evaluate(&entity));

        assert!(WorkflowService::evaluate_conditions(
            &[
                Condition {
                    field: "amount".to_string(),
                    operator: "gte".to_string(),
                    value: serde_json::json!(1500),
                },
                Condition {
                    field: "department".to_string(),
                    operator: "eq".to_string(),
                    value: serde_json::json!("engineering"),
                },
            ],
            &entity
        ));
    }

    #[tokio::test]
    async fn test_parallel_approval_all_required() {
        let svc = make_service();

        let config = serde_json::json!({
            "steps": [
                {
                    "step_number": 1,
                    "step_name": "Dual Approval",
                    "parallel": {
                        "mode": "all_required",
                        "assignee_user_ids": [10, 20],
                        "assignee_roles": []
                    }
                }
            ]
        });

        let template = svc
            .create_template(
                CreateWorkflowTemplate {
                    name: "Parallel Test".to_string(),
                    description: "Test".to_string(),
                    entity_type: WorkflowEntityType::Expense,
                    config_json: config,
                },
                1,
            )
            .await
            .unwrap();

        let instance = svc
            .start_workflow(template.id, 100, WorkflowEntityType::Expense, 5, 1)
            .await
            .unwrap();

        let detail = svc.get_instance(instance.id, 1).await.unwrap();
        let step = detail.steps.iter().find(|s| s.step_number == 1).unwrap();

        // First approval should not complete the step
        let inst = svc
            .approve_step(instance.id, step.id, 10, Some("OK from 10".to_string()), 1)
            .await
            .unwrap();
        assert_eq!(inst.status, WorkflowStatus::Pending);
        assert_eq!(inst.current_step, 1);

        // Step should still be pending
        let detail = svc.get_instance(instance.id, 1).await.unwrap();
        let step_after = detail.steps.iter().find(|s| s.step_number == 1).unwrap();
        assert_eq!(step_after.status, WorkflowStepStatus::Pending);

        // Second approval should complete the step and workflow
        let inst = svc
            .approve_step(instance.id, step.id, 20, Some("OK from 20".to_string()), 1)
            .await
            .unwrap();
        assert_eq!(inst.status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn test_parallel_approval_any_one() {
        let svc = make_service();

        let config = serde_json::json!({
            "steps": [
                {
                    "step_number": 1,
                    "step_name": "Any Approval",
                    "parallel": {
                        "mode": "any_one",
                        "assignee_user_ids": [30, 40],
                        "assignee_roles": []
                    }
                }
            ]
        });

        let template = svc
            .create_template(
                CreateWorkflowTemplate {
                    name: "AnyOne Test".to_string(),
                    description: "Test".to_string(),
                    entity_type: WorkflowEntityType::Invoice,
                    config_json: config,
                },
                1,
            )
            .await
            .unwrap();

        let instance = svc
            .start_workflow(template.id, 200, WorkflowEntityType::Invoice, 5, 1)
            .await
            .unwrap();

        let detail = svc.get_instance(instance.id, 1).await.unwrap();
        let step = detail.steps.iter().find(|s| s.step_number == 1).unwrap();

        // First approval should complete the step
        let inst = svc
            .approve_step(instance.id, step.id, 30, None, 1)
            .await
            .unwrap();
        assert_eq!(inst.status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn test_pending_approvals_by_role() {
        let svc = make_service();

        let template = svc
            .create_purchase_order_approval_template(1)
            .await
            .unwrap();
        let instance = svc
            .start_workflow(template.id, 100, WorkflowEntityType::PurchaseOrder, 5, 1)
            .await
            .unwrap();

        // Steps have approver_role but no user_id, so role-based lookup should find it
        let pending = svc
            .get_pending_approvals_by_role("manager".to_string(), 1)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, instance.id);
    }

    #[tokio::test]
    async fn test_escalation_rule() {
        let svc = make_service();

        let rule = EscalationRule {
            timeout_hours: 24,
            reminder_hours: 12,
            escalate_to_role: Some("admin".to_string()),
            escalate_to_manager: true,
        };

        let instance = WorkflowInstance {
            id: 1,
            tenant_id: 1,
            template_id: 1,
            entity_id: 100,
            entity_type: WorkflowEntityType::Invoice,
            status: WorkflowStatus::Pending,
            current_step: 1,
            assigned_user_id: Some(5),
            created_by: 10,
            created_at: chrono::Utc::now() - chrono::Duration::hours(25),
            completed_at: None,
        };

        let step = WorkflowStep {
            id: 1,
            instance_id: 1,
            step_number: 1,
            step_name: "Review".to_string(),
            approver_role: None,
            approver_user_id: Some(5),
            status: WorkflowStepStatus::Pending,
            comment: None,
            created_at: chrono::Utc::now() - chrono::Duration::hours(25),
            completed_at: None,
        };

        // Should not error even though repo stubs return empty
        let result = svc.check_escalation(&instance, &step, &rule).await;
        assert!(result.is_ok());
    }
}
