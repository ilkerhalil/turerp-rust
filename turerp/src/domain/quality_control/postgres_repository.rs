//! PostgreSQL quality control repository implementation
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::quality_control::model::{
    CreateInspection, CreateNonConformanceReport, Inspection, InspectionStatus, NcrStatus, NcrType,
    NonConformanceReport, UpdateInspection, UpdateNonConformanceReport,
};
use crate::domain::quality_control::repository::{
    BoxInspectionRepository, BoxNcrRepository, InspectionRepository, NcrRepository,
};
use crate::error::ApiError;

/// Database row representation for Inspection
#[derive(Debug, FromRow)]
struct InspectionRow {
    id: i64,
    tenant_id: i64,
    work_order_id: Option<i64>,
    product_id: i64,
    inspection_type: String,
    quantity_inspected: Decimal,
    quantity_passed: Decimal,
    quantity_failed: Decimal,
    status: String,
    inspector_id: Option<i64>,
    inspected_at: Option<chrono::DateTime<chrono::Utc>>,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

/// Convert InspectionStatus to/from database string
fn inspection_status_to_str(status: &InspectionStatus) -> &'static str {
    match status {
        InspectionStatus::Pending => "Pending",
        InspectionStatus::InProgress => "InProgress",
        InspectionStatus::Passed => "Passed",
        InspectionStatus::Failed => "Failed",
        InspectionStatus::Rework => "Rework",
    }
}

fn parse_inspection_status(s: &str) -> Result<InspectionStatus, String> {
    match s {
        "Pending" => Ok(InspectionStatus::Pending),
        "InProgress" => Ok(InspectionStatus::InProgress),
        "Passed" => Ok(InspectionStatus::Passed),
        "Failed" => Ok(InspectionStatus::Failed),
        "Rework" => Ok(InspectionStatus::Rework),
        _ => Err(format!("Invalid inspection status: {}", s)),
    }
}

impl From<InspectionRow> for Inspection {
    fn from(row: InspectionRow) -> Self {
        let status = parse_inspection_status(&row.status).unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid inspection status '{}' in database: {}, defaulting to Pending",
                row.status,
                e
            );
            InspectionStatus::Pending
        });
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            work_order_id: row.work_order_id,
            product_id: row.product_id,
            inspection_type: row.inspection_type,
            quantity_inspected: row.quantity_inspected,
            quantity_passed: row.quantity_passed,
            quantity_failed: row.quantity_failed,
            status,
            inspector_id: row.inspector_id,
            inspected_at: row.inspected_at,
            notes: row.notes,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL inspection repository
pub struct PostgresInspectionRepository {
    pool: Arc<PgPool>,
}

impl PostgresInspectionRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxInspectionRepository {
        Arc::new(self) as BoxInspectionRepository
    }
}

#[async_trait]
impl InspectionRepository for PostgresInspectionRepository {
    async fn create(&self, create: CreateInspection) -> Result<Inspection, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let status_str = inspection_status_to_str(&create.status);
        let inspected_at = if create.status == InspectionStatus::Passed
            || create.status == InspectionStatus::Failed
        {
            Some(chrono::Utc::now())
        } else {
            None
        };

        let row: InspectionRow = sqlx::query_as(
            r#"
            INSERT INTO inspections (tenant_id, work_order_id, product_id, inspection_type,
                                     quantity_inspected, quantity_passed, quantity_failed,
                                     status, inspector_id, inspected_at, notes, created_at,
                                     deleted_at, deleted_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NULL, NULL)
            RETURNING id, tenant_id, work_order_id, product_id, inspection_type,
                      quantity_inspected, quantity_passed, quantity_failed,
                      status, inspector_id, inspected_at, notes, created_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(create.work_order_id)
        .bind(create.product_id)
        .bind(&create.inspection_type)
        .bind(create.quantity_inspected)
        .bind(create.quantity_passed)
        .bind(create.quantity_failed)
        .bind(status_str)
        .bind(create.inspector_id)
        .bind(inspected_at)
        .bind(&create.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Inspection"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Inspection>, ApiError> {
        let result: Option<InspectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, work_order_id, product_id, inspection_type,
                   quantity_inspected, quantity_passed, quantity_failed,
                   status, inspector_id, inspected_at, notes, created_at,
                   deleted_at, deleted_by
            FROM inspections
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find inspection by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Inspection>, ApiError> {
        let rows: Vec<InspectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, work_order_id, product_id, inspection_type,
                   quantity_inspected, quantity_passed, quantity_failed,
                   status, inspector_id, inspected_at, notes, created_at,
                   deleted_at, deleted_by
            FROM inspections
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find inspections by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_work_order(&self, work_order_id: i64) -> Result<Vec<Inspection>, ApiError> {
        let rows: Vec<InspectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, work_order_id, product_id, inspection_type,
                   quantity_inspected, quantity_passed, quantity_failed,
                   status, inspector_id, inspected_at, notes, created_at,
                   deleted_at, deleted_by
            FROM inspections
            WHERE work_order_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(work_order_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find inspections by work order: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateInspection,
    ) -> Result<Inspection, ApiError> {
        let row: InspectionRow = sqlx::query_as(
            r#"
            UPDATE inspections
            SET status = COALESCE($2, status),
                quantity_passed = COALESCE($3, quantity_passed),
                quantity_failed = COALESCE($4, quantity_failed),
                inspector_id = COALESCE($5, inspector_id),
                notes = COALESCE($6, notes),
                inspected_at = CASE
                    WHEN $2 IN ('Passed', 'Failed') THEN NOW()
                    ELSE inspected_at
                END
            WHERE id = $1 AND tenant_id = $7 AND deleted_at IS NULL
            RETURNING id, tenant_id, work_order_id, product_id, inspection_type,
                      quantity_inspected, quantity_passed, quantity_failed,
                      status, inspector_id, inspected_at, notes, created_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(id)
        .bind(
            update
                .status
                .map(|s| inspection_status_to_str(&s).to_string()),
        )
        .bind(update.quantity_passed)
        .bind(update.quantity_failed)
        .bind(update.inspector_id)
        .bind(&update.notes)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Inspection"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE inspections
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete inspection: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Inspection not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Inspection, ApiError> {
        let row: InspectionRow = sqlx::query_as(
            r#"
            UPDATE inspections
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING id, tenant_id, work_order_id, product_id, inspection_type,
                      quantity_inspected, quantity_passed, quantity_failed,
                      status, inspector_id, inspected_at, notes, created_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Inspection"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Inspection>, ApiError> {
        let rows: Vec<InspectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, work_order_id, product_id, inspection_type,
                   quantity_inspected, quantity_passed, quantity_failed,
                   status, inspector_id, inspected_at, notes, created_at,
                   deleted_at, deleted_by
            FROM inspections
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted inspections: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM inspections
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy inspection: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Inspection not found".to_string()));
        }

        Ok(())
    }
}

/// Database row representation for NonConformanceReport
#[derive(Debug, FromRow)]
struct NcrRow {
    id: i64,
    tenant_id: i64,
    inspection_id: Option<i64>,
    product_id: i64,
    ncr_type: String,
    description: String,
    root_cause: Option<String>,
    corrective_action: Option<String>,
    status: String,
    raised_by: i64,
    raised_at: chrono::DateTime<chrono::Utc>,
    closed_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

fn ncr_type_to_str(ncr_type: &NcrType) -> &'static str {
    match ncr_type {
        NcrType::Minor => "Minor",
        NcrType::Major => "Major",
        NcrType::Critical => "Critical",
    }
}

fn parse_ncr_type(s: &str) -> Result<NcrType, String> {
    match s {
        "Minor" => Ok(NcrType::Minor),
        "Major" => Ok(NcrType::Major),
        "Critical" => Ok(NcrType::Critical),
        _ => Err(format!("Invalid NCR type: {}", s)),
    }
}

fn ncr_status_to_str(status: &NcrStatus) -> &'static str {
    match status {
        NcrStatus::Open => "Open",
        NcrStatus::UnderReview => "UnderReview",
        NcrStatus::CorrectiveAction => "CorrectiveAction",
        NcrStatus::Closed => "Closed",
        NcrStatus::Rejected => "Rejected",
    }
}

fn parse_ncr_status(s: &str) -> Result<NcrStatus, String> {
    match s {
        "Open" => Ok(NcrStatus::Open),
        "UnderReview" => Ok(NcrStatus::UnderReview),
        "CorrectiveAction" => Ok(NcrStatus::CorrectiveAction),
        "Closed" => Ok(NcrStatus::Closed),
        "Rejected" => Ok(NcrStatus::Rejected),
        _ => Err(format!("Invalid NCR status: {}", s)),
    }
}

impl From<NcrRow> for NonConformanceReport {
    fn from(row: NcrRow) -> Self {
        let ncr_type = parse_ncr_type(&row.ncr_type).unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid NCR type '{}' in database: {}, defaulting to Minor",
                row.ncr_type,
                e
            );
            NcrType::Minor
        });
        let status = parse_ncr_status(&row.status).unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid NCR status '{}' in database: {}, defaulting to Open",
                row.status,
                e
            );
            NcrStatus::Open
        });
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            inspection_id: row.inspection_id,
            product_id: row.product_id,
            ncr_type,
            description: row.description,
            root_cause: row.root_cause,
            corrective_action: row.corrective_action,
            status,
            raised_by: row.raised_by,
            raised_at: row.raised_at,
            closed_at: row.closed_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL NCR repository
pub struct PostgresNcrRepository {
    pool: Arc<PgPool>,
}

impl PostgresNcrRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxNcrRepository {
        Arc::new(self) as BoxNcrRepository
    }
}

#[async_trait]
impl NcrRepository for PostgresNcrRepository {
    async fn create(
        &self,
        create: CreateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let ncr_type_str = ncr_type_to_str(&create.ncr_type);

        let row: NcrRow = sqlx::query_as(
            r#"
            INSERT INTO non_conformance_reports (tenant_id, inspection_id, product_id, ncr_type,
                                                 description, root_cause, corrective_action,
                                                 status, raised_by, raised_at, closed_at,
                                                 deleted_at, deleted_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'Open', $8, NOW(), NULL, NULL, NULL)
            RETURNING id, tenant_id, inspection_id, product_id, ncr_type,
                      description, root_cause, corrective_action,
                      status, raised_by, raised_at, closed_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(create.inspection_id)
        .bind(create.product_id)
        .bind(ncr_type_str)
        .bind(&create.description)
        .bind(&create.root_cause)
        .bind(&create.corrective_action)
        .bind(create.raised_by)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "NonConformanceReport"))?;

        Ok(row.into())
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<NonConformanceReport>, ApiError> {
        let result: Option<NcrRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, inspection_id, product_id, ncr_type,
                   description, root_cause, corrective_action,
                   status, raised_by, raised_at, closed_at,
                   deleted_at, deleted_by
            FROM non_conformance_reports
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find NCR by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<NonConformanceReport>, ApiError> {
        let rows: Vec<NcrRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, inspection_id, product_id, ncr_type,
                   description, root_cause, corrective_action,
                   status, raised_by, raised_at, closed_at,
                   deleted_at, deleted_by
            FROM non_conformance_reports
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY raised_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find NCRs by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_inspection(
        &self,
        inspection_id: i64,
    ) -> Result<Vec<NonConformanceReport>, ApiError> {
        let rows: Vec<NcrRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, inspection_id, product_id, ncr_type,
                   description, root_cause, corrective_action,
                   status, raised_by, raised_at, closed_at,
                   deleted_at, deleted_by
            FROM non_conformance_reports
            WHERE inspection_id = $1 AND deleted_at IS NULL
            ORDER BY raised_at DESC
            "#,
        )
        .bind(inspection_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find NCRs by inspection: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError> {
        let row: NcrRow = sqlx::query_as(
            r#"
            UPDATE non_conformance_reports
            SET ncr_type = COALESCE($2, ncr_type),
                description = COALESCE($3, description),
                root_cause = COALESCE($4, root_cause),
                corrective_action = COALESCE($5, corrective_action),
                status = COALESCE($6, status),
                closed_at = CASE
                    WHEN $6 = 'Closed' THEN NOW()
                    ELSE closed_at
                END
            WHERE id = $1 AND tenant_id = $7 AND deleted_at IS NULL
            RETURNING id, tenant_id, inspection_id, product_id, ncr_type,
                      description, root_cause, corrective_action,
                      status, raised_by, raised_at, closed_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(id)
        .bind(update.ncr_type.map(|t| ncr_type_to_str(&t).to_string()))
        .bind(&update.description)
        .bind(&update.root_cause)
        .bind(&update.corrective_action)
        .bind(update.status.map(|s| ncr_status_to_str(&s).to_string()))
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "NonConformanceReport"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE non_conformance_reports
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete NCR: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("NCR not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<NonConformanceReport, ApiError> {
        let row: NcrRow = sqlx::query_as(
            r#"
            UPDATE non_conformance_reports
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING id, tenant_id, inspection_id, product_id, ncr_type,
                      description, root_cause, corrective_action,
                      status, raised_by, raised_at, closed_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "NonConformanceReport"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<NonConformanceReport>, ApiError> {
        let rows: Vec<NcrRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, inspection_id, product_id, ncr_type,
                   description, root_cause, corrective_action,
                   status, raised_by, raised_at, closed_at,
                   deleted_at, deleted_by
            FROM non_conformance_reports
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted NCRs: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM non_conformance_reports
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy NCR: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("NCR not found".to_string()));
        }

        Ok(())
    }
}
