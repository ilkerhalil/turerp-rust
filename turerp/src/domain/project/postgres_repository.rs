//! PostgreSQL project repository implementation

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::project::model::{
    CostType, CreateProject, CreateProjectCost, CreateWbsItem, Project, ProjectCost, ProjectStatus,
    WbsItem,
};
use crate::domain::project::repository::{
    BoxProjectCostRepository, BoxProjectRepository, BoxWbsItemRepository, ProjectCostRepository,
    ProjectRepository, WbsItemRepository,
};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

// ---------------------------------------------------------------------------
// Project row mapping
// ---------------------------------------------------------------------------

/// Database row representation for Project
#[derive(Debug, FromRow)]
struct ProjectRow {
    id: i64,
    tenant_id: i64,
    name: String,
    description: Option<String>,
    cari_id: Option<i64>,
    status: String,
    start_date: Option<chrono::DateTime<chrono::Utc>>,
    end_date: Option<chrono::DateTime<chrono::Utc>>,
    budget: Decimal,
    actual_cost: Decimal,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
    total_count: Option<i64>,
}

impl From<ProjectRow> for Project {
    fn from(row: ProjectRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid status '{}' in database: {}, defaulting to Planning",
                row.status,
                e
            );
            ProjectStatus::default()
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            description: row.description,
            cari_id: row.cari_id,
            status,
            start_date: row.start_date,
            end_date: row.end_date,
            budget: row.budget,
            actual_cost: row.actual_cost,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

// ---------------------------------------------------------------------------
// WbsItem row mapping
// ---------------------------------------------------------------------------

/// Database row representation for WbsItem
#[derive(Debug, FromRow)]
struct WbsItemRow {
    id: i64,
    project_id: i64,
    parent_id: Option<i64>,
    name: String,
    code: String,
    planned_hours: Decimal,
    actual_hours: Decimal,
    progress: Decimal,
    sort_order: i32,
}

impl From<WbsItemRow> for WbsItem {
    fn from(row: WbsItemRow) -> Self {
        Self {
            id: row.id,
            project_id: row.project_id,
            parent_id: row.parent_id,
            name: row.name,
            code: row.code,
            planned_hours: row.planned_hours,
            actual_hours: row.actual_hours,
            progress: row.progress,
            sort_order: row.sort_order,
        }
    }
}

// ---------------------------------------------------------------------------
// ProjectCost row mapping
// ---------------------------------------------------------------------------

/// Database row representation for ProjectCost
#[derive(Debug, FromRow)]
struct ProjectCostRow {
    id: i64,
    project_id: i64,
    wbs_item_id: Option<i64>,
    cost_type: String,
    amount: Decimal,
    description: String,
    incurred_at: chrono::DateTime<chrono::Utc>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ProjectCostRow> for ProjectCost {
    fn from(row: ProjectCostRow) -> Self {
        let cost_type = row.cost_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid cost_type '{}' in database: {}, defaulting to Other",
                row.cost_type,
                e
            );
            CostType::Other
        });

        Self {
            id: row.id,
            project_id: row.project_id,
            wbs_item_id: row.wbs_item_id,
            cost_type,
            amount: row.amount,
            description: row.description,
            incurred_at: row.incurred_at,
            created_at: row.created_at,
        }
    }
}

// ---------------------------------------------------------------------------
// PostgresProjectRepository
// ---------------------------------------------------------------------------

/// PostgreSQL project repository
pub struct PostgresProjectRepository {
    pool: Arc<PgPool>,
}

impl PostgresProjectRepository {
    /// Create a new PostgreSQL project repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxProjectRepository {
        Arc::new(self) as BoxProjectRepository
    }
}

#[async_trait]
impl ProjectRepository for PostgresProjectRepository {
    async fn create(&self, create: CreateProject) -> Result<Project, ApiError> {
        let status_str = ProjectStatus::default().to_string();

        let row: ProjectRow = sqlx::query_as(
            r#"
            INSERT INTO projects (tenant_id, name, description, cari_id, status,
                                   start_date, end_date, budget, actual_cost, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
            RETURNING id, tenant_id, name, description, cari_id, status,
                      start_date, end_date, budget, actual_cost, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.name)
        .bind(&create.description)
        .bind(create.cari_id)
        .bind(&status_str)
        .bind(create.start_date)
        .bind(create.end_date)
        .bind(create.budget)
        .bind(Decimal::ZERO)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Project"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Project>, ApiError> {
        let result: Option<ProjectRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, cari_id, status,
                   start_date, end_date, budget, actual_cost, created_at, updated_at, deleted_at, deleted_by
            FROM projects
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find project by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Project>, ApiError> {
        let rows: Vec<ProjectRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, cari_id, status,
                   start_date, end_date, budget, actual_cost, created_at, updated_at, deleted_at, deleted_by
            FROM projects
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find projects by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Project>, ApiError> {
        let offset = (page.saturating_sub(1)) * per_page;
        let rows: Vec<ProjectRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, cari_id, status,
                   start_date, end_date, budget, actual_cost, created_at, updated_at, deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM projects
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Project"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<Project> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_cari(&self, cari_id: i64, tenant_id: i64) -> Result<Vec<Project>, ApiError> {
        let rows: Vec<ProjectRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, cari_id, status,
                   start_date, end_date, budget, actual_cost, created_at, updated_at, deleted_at, deleted_by
            FROM projects
            WHERE cari_id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(cari_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find projects by cari: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: ProjectStatus,
    ) -> Result<Vec<Project>, ApiError> {
        let status_str = status.to_string();

        let rows: Vec<ProjectRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, cari_id, status,
                   start_date, end_date, budget, actual_cost, created_at, updated_at, deleted_at, deleted_by
            FROM projects
            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find projects by status: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: ProjectStatus,
    ) -> Result<Project, ApiError> {
        let status_str = status.to_string();

        let row: ProjectRow = sqlx::query_as(
            r#"
            UPDATE projects
            SET status = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3 AND deleted_at IS NULL
            RETURNING id, tenant_id, name, description, cari_id, status,
                      start_date, end_date, budget, actual_cost, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Project"))?;

        Ok(row.into())
    }

    async fn update_actual_cost(
        &self,
        id: i64,
        tenant_id: i64,
        cost: Decimal,
    ) -> Result<Project, ApiError> {
        let row: ProjectRow = sqlx::query_as(
            r#"
            UPDATE projects
            SET actual_cost = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3 AND deleted_at IS NULL
            RETURNING id, tenant_id, name, description, cari_id, status,
                      start_date, end_date, budget, actual_cost, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(cost)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Project"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE projects
            SET deleted_at = NOW(), deleted_by = $3, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete project: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Project not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Project, ApiError> {
        let row: ProjectRow = sqlx::query_as(
            r#"
            UPDATE projects
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING id, tenant_id, name, description, cari_id, status,
                      start_date, end_date, budget, actual_cost, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Project"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Project>, ApiError> {
        let rows: Vec<ProjectRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, cari_id, status,
                   start_date, end_date, budget, actual_cost, created_at, updated_at, deleted_at, deleted_by
            FROM projects
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted projects: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM projects
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy project: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Project not found".to_string()));
        }

        Ok(())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM projects
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete project: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Project not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PostgresWbsItemRepository
// ---------------------------------------------------------------------------

/// PostgreSQL WBS item repository
pub struct PostgresWbsItemRepository {
    pool: Arc<PgPool>,
}

impl PostgresWbsItemRepository {
    /// Create a new PostgreSQL WBS item repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxWbsItemRepository {
        Arc::new(self) as BoxWbsItemRepository
    }
}

#[async_trait]
impl WbsItemRepository for PostgresWbsItemRepository {
    async fn create(&self, create: CreateWbsItem) -> Result<WbsItem, ApiError> {
        let row: WbsItemRow = sqlx::query_as(
            r#"
            INSERT INTO wbs_items (project_id, parent_id, name, code,
                                  planned_hours, actual_hours, progress, sort_order)
            VALUES ($1, $2, $3, $4, $5, $6, $7,
                    (SELECT COALESCE(MAX(sort_order), 0) + 1 FROM wbs_items WHERE project_id = $1))
            RETURNING id, project_id, parent_id, name, code,
                      planned_hours, actual_hours, progress, sort_order
            "#,
        )
        .bind(create.project_id)
        .bind(create.parent_id)
        .bind(&create.name)
        .bind(&create.code)
        .bind(create.planned_hours)
        .bind(Decimal::ZERO)
        .bind(Decimal::ZERO)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WbsItem"))?;

        Ok(row.into())
    }

    async fn find_by_project(&self, project_id: i64) -> Result<Vec<WbsItem>, ApiError> {
        let rows: Vec<WbsItemRow> = sqlx::query_as(
            r#"
            SELECT id, project_id, parent_id, name, code,
                   planned_hours, actual_hours, progress, sort_order
            FROM wbs_items
            WHERE project_id = $1
            ORDER BY sort_order
            "#,
        )
        .bind(project_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find WBS items by project: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<WbsItem>, ApiError> {
        let result: Option<WbsItemRow> = sqlx::query_as(
            r#"
            SELECT id, project_id, parent_id, name, code,
                   planned_hours, actual_hours, progress, sort_order
            FROM wbs_items
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find WBS item by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn update_progress(
        &self,
        id: i64,
        progress: Decimal,
        hours: Decimal,
    ) -> Result<WbsItem, ApiError> {
        let row: WbsItemRow = sqlx::query_as(
            r#"
            UPDATE wbs_items
            SET progress = $1, actual_hours = $2
            WHERE id = $3
            RETURNING id, project_id, parent_id, name, code,
                      planned_hours, actual_hours, progress, sort_order
            "#,
        )
        .bind(progress)
        .bind(hours)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WbsItem"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM wbs_items
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete WBS item: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("WBS item not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PostgresProjectCostRepository
// ---------------------------------------------------------------------------

/// PostgreSQL project cost repository
pub struct PostgresProjectCostRepository {
    pool: Arc<PgPool>,
}

impl PostgresProjectCostRepository {
    /// Create a new PostgreSQL project cost repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxProjectCostRepository {
        Arc::new(self) as BoxProjectCostRepository
    }
}

#[async_trait]
impl ProjectCostRepository for PostgresProjectCostRepository {
    async fn create(&self, create: CreateProjectCost) -> Result<ProjectCost, ApiError> {
        let cost_type_str = create.cost_type.to_string();

        let row: ProjectCostRow = sqlx::query_as(
            r#"
            INSERT INTO project_costs (project_id, wbs_item_id, cost_type, amount,
                                       description, incurred_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            RETURNING id, project_id, wbs_item_id, cost_type, amount,
                      description, incurred_at, created_at
            "#,
        )
        .bind(create.project_id)
        .bind(create.wbs_item_id)
        .bind(&cost_type_str)
        .bind(create.amount)
        .bind(&create.description)
        .bind(create.incurred_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ProjectCost"))?;

        Ok(row.into())
    }

    async fn find_by_project(&self, project_id: i64) -> Result<Vec<ProjectCost>, ApiError> {
        let rows: Vec<ProjectCostRow> = sqlx::query_as(
            r#"
            SELECT id, project_id, wbs_item_id, cost_type, amount,
                   description, incurred_at, created_at
            FROM project_costs
            WHERE project_id = $1
            ORDER BY incurred_at DESC
            "#,
        )
        .bind(project_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find project costs by project: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_total_by_project(&self, project_id: i64) -> Result<Decimal, ApiError> {
        let result: (Decimal,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(amount), 0) FROM project_costs WHERE project_id = $1
            "#,
        )
        .bind(project_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find total cost by project: {}", e)))?;

        Ok(result.0)
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM project_costs
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete project cost: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Project cost not found".to_string()));
        }

        Ok(())
    }
}
