//! PostgreSQL workflow repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::workflow::model::{
    CreateWorkflowTemplate, WorkflowAuditLog, WorkflowEntityType, WorkflowInstance, WorkflowStatus,
    WorkflowStep, WorkflowStepApproval, WorkflowStepStatus, WorkflowTemplate,
};
use crate::domain::workflow::repository::{BoxWorkflowRepository, WorkflowRepository};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Row types
// ---------------------------------------------------------------------------

#[derive(Debug, FromRow)]
struct WorkflowTemplateRow {
    id: i64,
    tenant_id: i64,
    name: String,
    description: String,
    entity_type: String,
    config_json: serde_json::Value,
    is_active: bool,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
}

impl From<WorkflowTemplateRow> for WorkflowTemplate {
    fn from(row: WorkflowTemplateRow) -> Self {
        let entity_type = row.entity_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid entity_type '{}': {}, defaulting to invoice",
                row.entity_type,
                e
            );
            WorkflowEntityType::Invoice
        });
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            description: row.description,
            entity_type,
            config_json: row.config_json,
            is_active: row.is_active,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

#[derive(Debug, FromRow)]
struct WorkflowInstanceRow {
    id: i64,
    tenant_id: i64,
    template_id: i64,
    entity_id: i64,
    entity_type: String,
    status: String,
    current_step: i32,
    assigned_user_id: Option<i64>,
    created_by: i64,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<WorkflowInstanceRow> for WorkflowInstance {
    fn from(row: WorkflowInstanceRow) -> Self {
        let entity_type = row.entity_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid entity_type '{}': {}, defaulting to invoice",
                row.entity_type,
                e
            );
            WorkflowEntityType::Invoice
        });
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid status '{}': {}, defaulting to draft",
                row.status,
                e
            );
            WorkflowStatus::Draft
        });
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            template_id: row.template_id,
            entity_id: row.entity_id,
            entity_type,
            status,
            current_step: row.current_step,
            assigned_user_id: row.assigned_user_id,
            created_by: row.created_by,
            created_at: row.created_at,
            completed_at: row.completed_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct WorkflowStepRow {
    id: i64,
    instance_id: i64,
    step_number: i32,
    step_name: String,
    approver_role: Option<String>,
    approver_user_id: Option<i64>,
    status: String,
    comment: Option<String>,
    created_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<WorkflowStepRow> for WorkflowStep {
    fn from(row: WorkflowStepRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid step status '{}': {}, defaulting to pending",
                row.status,
                e
            );
            WorkflowStepStatus::Pending
        });
        Self {
            id: row.id,
            instance_id: row.instance_id,
            step_number: row.step_number,
            step_name: row.step_name,
            approver_role: row.approver_role,
            approver_user_id: row.approver_user_id,
            status,
            comment: row.comment,
            created_at: row.created_at,
            completed_at: row.completed_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct WorkflowAuditLogRow {
    id: i64,
    instance_id: i64,
    step_id: Option<i64>,
    action: String,
    user_id: i64,
    comment: Option<String>,
    timestamp: DateTime<Utc>,
}

impl From<WorkflowAuditLogRow> for WorkflowAuditLog {
    fn from(row: WorkflowAuditLogRow) -> Self {
        Self {
            id: row.id,
            instance_id: row.instance_id,
            step_id: row.step_id,
            action: row.action,
            user_id: row.user_id,
            comment: row.comment,
            timestamp: row.timestamp,
        }
    }
}

// ---------------------------------------------------------------------------
// PostgresWorkflowRepository
// ---------------------------------------------------------------------------

/// PostgreSQL workflow repository
pub struct PostgresWorkflowRepository {
    pool: Arc<PgPool>,
}

impl PostgresWorkflowRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxWorkflowRepository {
        Arc::new(self) as BoxWorkflowRepository
    }
}

#[async_trait]
impl WorkflowRepository for PostgresWorkflowRepository {
    async fn create_template(
        &self,
        create: CreateWorkflowTemplate,
        tenant_id: i64,
    ) -> Result<WorkflowTemplate, ApiError> {
        let row: WorkflowTemplateRow = sqlx::query_as(
            r#"
            INSERT INTO workflow_templates (tenant_id, name, description, entity_type, config_json, is_active)
            VALUES ($1, $2, $3, $4, $5, true)
            RETURNING id, tenant_id, name, description, entity_type, config_json, is_active, created_at, deleted_at, deleted_by
            "#
        )
        .bind(tenant_id)
        .bind(&create.name)
        .bind(&create.description)
        .bind(create.entity_type.to_string())
        .bind(create.config_json)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowTemplate"))?;
        Ok(row.into())
    }

    async fn find_template_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<WorkflowTemplate>, ApiError> {
        let row: Option<WorkflowTemplateRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, entity_type, config_json, is_active, created_at, deleted_at, deleted_by
            FROM workflow_templates
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowTemplate"))?;
        Ok(row.map(Into::into))
    }

    async fn find_templates(&self, tenant_id: i64) -> Result<Vec<WorkflowTemplate>, ApiError> {
        let rows: Vec<WorkflowTemplateRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, entity_type, config_json, is_active, created_at, deleted_at, deleted_by
            FROM workflow_templates
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowTemplate"))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn delete_template(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE workflow_templates
            SET deleted_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowTemplate"))?;
        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Template {} not found", id)));
        }
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
        let row: WorkflowInstanceRow = sqlx::query_as(
            r#"
            INSERT INTO workflow_instances (tenant_id, template_id, entity_id, entity_type, status, current_step, assigned_user_id, created_by)
            VALUES ($1, $2, $3, $4, 'pending', 1, NULL, $5)
            RETURNING id, tenant_id, template_id, entity_id, entity_type, status, current_step, assigned_user_id, created_by, created_at, completed_at
            "#
        )
        .bind(tenant_id)
        .bind(template_id)
        .bind(entity_id)
        .bind(entity_type.to_string())
        .bind(created_by)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowInstance"))?;
        Ok(row.into())
    }

    async fn find_instance_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<WorkflowInstance>, ApiError> {
        let row: Option<WorkflowInstanceRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, template_id, entity_id, entity_type, status, current_step, assigned_user_id, created_by, created_at, completed_at
            FROM workflow_instances
            WHERE id = $1 AND tenant_id = $2
            "#
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowInstance"))?;
        Ok(row.map(Into::into))
    }

    async fn update_instance(
        &self,
        instance: WorkflowInstance,
    ) -> Result<WorkflowInstance, ApiError> {
        let row: WorkflowInstanceRow = sqlx::query_as(
            r#"
            UPDATE workflow_instances
            SET status = $1, current_step = $2, assigned_user_id = $3, completed_at = $4
            WHERE id = $5 AND tenant_id = $6
            RETURNING id, tenant_id, template_id, entity_id, entity_type, status, current_step, assigned_user_id, created_by, created_at, completed_at
            "#
        )
        .bind(instance.status.to_string())
        .bind(instance.current_step)
        .bind(instance.assigned_user_id)
        .bind(instance.completed_at)
        .bind(instance.id)
        .bind(instance.tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowInstance"))?;
        Ok(row.into())
    }

    async fn find_instances(
        &self,
        tenant_id: i64,
        status: Option<WorkflowStatus>,
    ) -> Result<Vec<WorkflowInstance>, ApiError> {
        let rows: Vec<WorkflowInstanceRow> = if let Some(s) = status {
            sqlx::query_as(
                r#"
                SELECT id, tenant_id, template_id, entity_id, entity_type, status, current_step, assigned_user_id, created_by, created_at, completed_at
                FROM workflow_instances
                WHERE tenant_id = $1 AND status = $2
                ORDER BY created_at DESC
                "#
            )
            .bind(tenant_id)
            .bind(s.to_string())
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "WorkflowInstance"))?
        } else {
            sqlx::query_as(
                r#"
                SELECT id, tenant_id, template_id, entity_id, entity_type, status, current_step, assigned_user_id, created_by, created_at, completed_at
                FROM workflow_instances
                WHERE tenant_id = $1
                ORDER BY created_at DESC
                "#
            )
            .bind(tenant_id)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "WorkflowInstance"))?
        };
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create_step(&self, step: WorkflowStep) -> Result<WorkflowStep, ApiError> {
        let row: WorkflowStepRow = sqlx::query_as(
            r#"
            INSERT INTO workflow_steps (instance_id, step_number, step_name, approver_role, approver_user_id, status, comment, created_at, completed_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, instance_id, step_number, step_name, approver_role, approver_user_id, status, comment, created_at, completed_at
            "#
        )
        .bind(step.instance_id)
        .bind(step.step_number)
        .bind(&step.step_name)
        .bind(&step.approver_role)
        .bind(step.approver_user_id)
        .bind(step.status.to_string())
        .bind(&step.comment)
        .bind(step.created_at)
        .bind(step.completed_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowStep"))?;
        Ok(row.into())
    }

    async fn find_steps_by_instance(
        &self,
        instance_id: i64,
    ) -> Result<Vec<WorkflowStep>, ApiError> {
        let rows: Vec<WorkflowStepRow> = sqlx::query_as(
            r#"
            SELECT id, instance_id, step_number, step_name, approver_role, approver_user_id, status, comment, created_at, completed_at
            FROM workflow_steps
            WHERE instance_id = $1
            ORDER BY step_number ASC
            "#
        )
        .bind(instance_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowStep"))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update_step(&self, step: WorkflowStep) -> Result<WorkflowStep, ApiError> {
        let row: WorkflowStepRow = sqlx::query_as(
            r#"
            UPDATE workflow_steps
            SET status = $1, comment = $2, completed_at = $3
            WHERE id = $4
            RETURNING id, instance_id, step_number, step_name, approver_role, approver_user_id, status, comment, created_at, completed_at
            "#
        )
        .bind(step.status.to_string())
        .bind(&step.comment)
        .bind(step.completed_at)
        .bind(step.id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowStep"))?;
        Ok(row.into())
    }

    async fn create_audit_log(&self, log: WorkflowAuditLog) -> Result<WorkflowAuditLog, ApiError> {
        let row: WorkflowAuditLogRow = sqlx::query_as(
            r#"
            INSERT INTO workflow_audit_log (instance_id, step_id, action, user_id, comment, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, instance_id, step_id, action, user_id, comment, timestamp
            "#
        )
        .bind(log.instance_id)
        .bind(log.step_id)
        .bind(&log.action)
        .bind(log.user_id)
        .bind(&log.comment)
        .bind(log.timestamp)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowAuditLog"))?;
        Ok(row.into())
    }

    async fn find_audit_logs_by_instance(
        &self,
        instance_id: i64,
    ) -> Result<Vec<WorkflowAuditLog>, ApiError> {
        let rows: Vec<WorkflowAuditLogRow> = sqlx::query_as(
            r#"
            SELECT id, instance_id, step_id, action, user_id, comment, timestamp
            FROM workflow_audit_log
            WHERE instance_id = $1
            ORDER BY timestamp ASC
            "#,
        )
        .bind(instance_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowAuditLog"))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_pending_approvals_for_user(
        &self,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<WorkflowInstance>, ApiError> {
        let rows: Vec<WorkflowInstanceRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, template_id, entity_id, entity_type, status, current_step, assigned_user_id, created_by, created_at, completed_at
            FROM workflow_instances
            WHERE tenant_id = $1 AND status = 'pending' AND assigned_user_id = $2
            ORDER BY created_at DESC
            "#
        )
        .bind(tenant_id)
        .bind(user_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowInstance"))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_overdue_steps(
        &self,
        hours: i64,
    ) -> Result<Vec<(WorkflowInstance, WorkflowStep)>, ApiError> {
        #[derive(Debug, FromRow)]
        struct OverdueRow {
            i_id: i64,
            i_tenant_id: i64,
            i_template_id: i64,
            i_entity_id: i64,
            i_entity_type: String,
            i_status: String,
            i_current_step: i32,
            i_assigned_user_id: Option<i64>,
            i_created_by: i64,
            i_created_at: DateTime<Utc>,
            i_completed_at: Option<DateTime<Utc>>,
            s_id: i64,
            s_instance_id: i64,
            s_step_number: i32,
            s_step_name: String,
            s_approver_role: Option<String>,
            s_approver_user_id: Option<i64>,
            s_status: String,
            s_comment: Option<String>,
            s_created_at: DateTime<Utc>,
            s_completed_at: Option<DateTime<Utc>>,
        }

        let rows: Vec<OverdueRow> = sqlx::query_as(
            r#"
            SELECT
                i.id as i_id, i.tenant_id as i_tenant_id, i.template_id as i_template_id, i.entity_id as i_entity_id, i.entity_type as i_entity_type, i.status as i_status, i.current_step as i_current_step, i.assigned_user_id as i_assigned_user_id, i.created_by as i_created_by, i.created_at as i_created_at, i.completed_at as i_completed_at,
                s.id as s_id, s.instance_id as s_instance_id, s.step_number as s_step_number, s.step_name as s_step_name, s.approver_role as s_approver_role, s.approver_user_id as s_approver_user_id, s.status as s_status, s.comment as s_comment, s.created_at as s_created_at, s.completed_at as s_completed_at
            FROM workflow_steps s
            JOIN workflow_instances i ON i.id = s.instance_id
            WHERE s.status = 'pending' AND s.created_at < NOW() - INTERVAL '1 hour' * $1
            AND i.status = 'pending'
            "#
        )
        .bind(hours)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkflowStep"))?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let instance = WorkflowInstance {
                    id: r.i_id,
                    tenant_id: r.i_tenant_id,
                    template_id: r.i_template_id,
                    entity_id: r.i_entity_id,
                    entity_type: r
                        .i_entity_type
                        .parse()
                        .unwrap_or(WorkflowEntityType::Invoice),
                    status: r.i_status.parse().unwrap_or(WorkflowStatus::Draft),
                    current_step: r.i_current_step,
                    assigned_user_id: r.i_assigned_user_id,
                    created_by: r.i_created_by,
                    created_at: r.i_created_at,
                    completed_at: r.i_completed_at,
                };
                let step = WorkflowStep {
                    id: r.s_id,
                    instance_id: r.s_instance_id,
                    step_number: r.s_step_number,
                    step_name: r.s_step_name,
                    approver_role: r.s_approver_role,
                    approver_user_id: r.s_approver_user_id,
                    status: r.s_status.parse().unwrap_or(WorkflowStepStatus::Pending),
                    comment: r.s_comment,
                    created_at: r.s_created_at,
                    completed_at: r.s_completed_at,
                };
                (instance, step)
            })
            .collect())
    }

    async fn find_users_by_role(&self, _tenant_id: i64, _role: &str) -> Result<Vec<i64>, ApiError> {
        // Stub: user/role domain not integrated in this repository
        Ok(Vec::new())
    }

    async fn find_pending_approvals_by_role(
        &self,
        _role: String,
        _tenant_id: i64,
    ) -> Result<Vec<WorkflowInstance>, ApiError> {
        // Stub: would need join with workflow_steps on approver_role
        Ok(Vec::new())
    }

    async fn create_step_approval(
        &self,
        _approval: WorkflowStepApproval,
    ) -> Result<WorkflowStepApproval, ApiError> {
        Err(ApiError::Internal(
            "Step approvals require a dedicated table; not yet migrated".to_string(),
        ))
    }

    async fn find_step_approvals(
        &self,
        _step_id: i64,
    ) -> Result<Vec<WorkflowStepApproval>, ApiError> {
        Ok(Vec::new())
    }
}
