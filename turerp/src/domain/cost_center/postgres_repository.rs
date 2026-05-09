//! PostgreSQL cost center repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::db::error::map_sqlx_error;
use crate::domain::cost_center::model::{
    CostCenter, CostCenterAllocation, CostCenterType, CreateAllocation, CreateCostCenter,
    ProfitabilityReport, UpdateCostCenter,
};
use crate::domain::cost_center::repository::{BoxCostCenterRepository, CostCenterRepository};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// CostCenterRow / CostCenter conversion
// ---------------------------------------------------------------------------

/// Database row representation for CostCenter
#[derive(Debug, FromRow)]
struct CostCenterRow {
    id: i64,
    tenant_id: i64,
    code: String,
    name: String,
    description: Option<String>,
    center_type: String,
    parent_id: Option<i64>,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
    total_count: Option<i64>,
}

impl From<CostCenterRow> for CostCenter {
    fn from(row: CostCenterRow) -> Self {
        let center_type = row.center_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid center_type '{}' in database: {}, defaulting to Cost",
                row.center_type,
                e
            );
            CostCenterType::Cost
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            code: row.code,
            name: row.name,
            description: row.description,
            center_type,
            parent_id: row.parent_id,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// Database row representation for CostCenterAllocation
#[derive(Debug, FromRow)]
struct AllocationRow {
    id: i64,
    tenant_id: i64,
    source_type: String,
    source_id: i64,
    cost_center_id: i64,
    amount: Decimal,
    percentage: Decimal,
    allocation_date: DateTime<Utc>,
    description: Option<String>,
    created_at: DateTime<Utc>,
}

impl From<AllocationRow> for CostCenterAllocation {
    fn from(row: AllocationRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            source_type: row.source_type,
            source_id: row.source_id,
            cost_center_id: row.cost_center_id,
            amount: row.amount,
            percentage: row.percentage,
            allocation_date: row.allocation_date,
            description: row.description,
            created_at: row.created_at,
        }
    }
}

// ===========================================================================
// PostgresCostCenterRepository
// ===========================================================================

/// PostgreSQL cost center repository
pub struct PostgresCostCenterRepository {
    pool: Arc<PgPool>,
}

impl PostgresCostCenterRepository {
    /// Create a new PostgreSQL cost center repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxCostCenterRepository {
        Arc::new(self) as BoxCostCenterRepository
    }
}

/// Common column list for cost_centers SELECT queries
const COST_CENTER_COLUMNS: &str = r#"
    id, tenant_id, code, name, description, center_type, parent_id,
    is_active, created_at, updated_at, deleted_at, deleted_by
"#;

#[async_trait]
impl CostCenterRepository for PostgresCostCenterRepository {
    async fn create(
        &self,
        create: CreateCostCenter,
        tenant_id: i64,
    ) -> Result<CostCenter, ApiError> {
        let center_type_str = create.center_type.to_string();

        let row: CostCenterRow = sqlx::query_as(&format!(
            r#"
            INSERT INTO cost_centers (tenant_id, code, name, description, center_type, parent_id, is_active)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING {COST_CENTER_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(tenant_id)
        .bind(&create.code)
        .bind(&create.name)
        .bind(&create.description)
        .bind(&center_type_str)
        .bind(create.parent_id)
        .bind(create.is_active)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "CostCenter"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<CostCenter>, ApiError> {
        let result: Option<CostCenterRow> = sqlx::query_as(&format!(
            r#"
            SELECT {COST_CENTER_COLUMNS}, 0 as total_count
            FROM cost_centers
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find cost center: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        center_type: Option<CostCenterType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<CostCenter>, ApiError> {
        let offset = params.offset() as i64;
        let per_page = params.per_page as i64;

        match center_type {
            Some(ct) => {
                let type_str = ct.to_string();
                let rows: Vec<CostCenterRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {COST_CENTER_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM cost_centers
                    WHERE tenant_id = $1 AND center_type = $2 AND deleted_at IS NULL
                    ORDER BY code ASC
                    LIMIT $3 OFFSET $4
                    "#,
                ))
                .bind(tenant_id)
                .bind(&type_str)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "CostCenter"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<CostCenter> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
            None => {
                let rows: Vec<CostCenterRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {COST_CENTER_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM cost_centers
                    WHERE tenant_id = $1 AND deleted_at IS NULL
                    ORDER BY code ASC
                    LIMIT $2 OFFSET $3
                    "#,
                ))
                .bind(tenant_id)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "CostCenter"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<CostCenter> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
        }
    }

    async fn find_by_type(
        &self,
        tenant_id: i64,
        center_type: CostCenterType,
    ) -> Result<Vec<CostCenter>, ApiError> {
        let type_str = center_type.to_string();
        let rows: Vec<CostCenterRow> = sqlx::query_as(&format!(
            r#"
            SELECT {COST_CENTER_COLUMNS}, 0 as total_count
            FROM cost_centers
            WHERE tenant_id = $1 AND center_type = $2 AND deleted_at IS NULL
            ORDER BY code ASC
            "#,
        ))
        .bind(tenant_id)
        .bind(&type_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "CostCenter"))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCostCenter,
    ) -> Result<CostCenter, ApiError> {
        let row: CostCenterRow = sqlx::query_as(&format!(
            r#"
            UPDATE cost_centers
            SET
                code = COALESCE($1, code),
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                center_type = COALESCE($4, center_type),
                parent_id = COALESCE($5, parent_id),
                is_active = COALESCE($6, is_active),
                updated_at = NOW()
            WHERE id = $7 AND tenant_id = $8 AND deleted_at IS NULL
            RETURNING {COST_CENTER_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(&update.code)
        .bind(&update.name)
        .bind(&update.description)
        .bind(update.center_type.as_ref().map(|t| t.to_string()))
        .bind(update.parent_id)
        .bind(update.is_active)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "CostCenter"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE cost_centers
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete cost center: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Cost center not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<CostCenter, ApiError> {
        let row: CostCenterRow = sqlx::query_as(&format!(
            r#"
            UPDATE cost_centers
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING {COST_CENTER_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "CostCenter"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<CostCenter>, ApiError> {
        let rows: Vec<CostCenterRow> = sqlx::query_as(&format!(
            r#"
            SELECT {COST_CENTER_COLUMNS}, 0 as total_count
            FROM cost_centers
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        ))
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted cost centers: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM cost_centers
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy cost center: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Deleted cost center not found".to_string(),
            ));
        }

        Ok(())
    }

    async fn create_allocation(
        &self,
        allocation: CreateAllocation,
        tenant_id: i64,
    ) -> Result<CostCenterAllocation, ApiError> {
        let row: AllocationRow = sqlx::query_as(
            r#"
            INSERT INTO cost_center_allocations
                (tenant_id, source_type, source_id, cost_center_id, amount, percentage, allocation_date, description)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, tenant_id, source_type, source_id, cost_center_id,
                      amount, percentage, allocation_date, description, created_at
            "#,
        )
        .bind(tenant_id)
        .bind(&allocation.source_type)
        .bind(allocation.source_id)
        .bind(allocation.cost_center_id)
        .bind(allocation.amount)
        .bind(allocation.percentage)
        .bind(allocation.allocation_date)
        .bind(&allocation.description)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "CostCenterAllocation"))?;

        Ok(row.into())
    }

    async fn get_allocations(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<CostCenterAllocation>, ApiError> {
        let rows: Vec<AllocationRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, source_type, source_id, cost_center_id,
                   amount, percentage, allocation_date, description, created_at
            FROM cost_center_allocations
            WHERE cost_center_id = $1 AND tenant_id = $2
            ORDER BY allocation_date DESC, id DESC
            "#,
        )
        .bind(cost_center_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get allocations: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_profitability_report(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
        period_start: Option<DateTime<Utc>>,
        period_end: Option<DateTime<Utc>>,
    ) -> Result<ProfitabilityReport, ApiError> {
        let center: Option<CostCenterRow> = sqlx::query_as(&format!(
            r#"
            SELECT {COST_CENTER_COLUMNS}, 0 as total_count
            FROM cost_centers
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        ))
        .bind(cost_center_id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find cost center: {}", e)))?;

        let center = center.ok_or_else(|| {
            ApiError::NotFound(format!("Cost center {} not found", cost_center_id))
        })?;

        let center_type = center.center_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid center_type '{}' in database: {}, defaulting to Cost",
                center.center_type,
                e
            );
            CostCenterType::Cost
        });

        let mut query = r#"
            SELECT
                COALESCE(SUM(CASE WHEN source_type IN ('invoice', 'sales') THEN amount ELSE 0 END), 0) as total_income,
                COALESCE(SUM(CASE WHEN source_type NOT IN ('invoice', 'sales') THEN amount ELSE 0 END), 0) as total_expense,
                COUNT(*) as allocation_count
            FROM cost_center_allocations
            WHERE cost_center_id = $1 AND tenant_id = $2
        "#
        .to_string();

        if period_start.is_some() {
            query.push_str(" AND allocation_date >= $3");
        }
        if period_end.is_some() {
            if period_start.is_some() {
                query.push_str(" AND allocation_date <= $4");
            } else {
                query.push_str(" AND allocation_date <= $3");
            }
        }

        let mut q = sqlx::query_as::<_, ProfitabilityRow>(&query)
            .bind(cost_center_id)
            .bind(tenant_id);

        if let Some(start) = period_start {
            q = q.bind(start);
        }
        if let Some(end) = period_end {
            q = q.bind(end);
        }

        let row: ProfitabilityRow = q.fetch_one(&*self.pool).await.map_err(|e| {
            ApiError::Database(format!("Failed to get profitability report: {}", e))
        })?;

        let net_profit = row.total_income - row.total_expense;

        Ok(ProfitabilityReport {
            cost_center_id: center.id,
            cost_center_code: center.code,
            cost_center_name: center.name,
            center_type,
            total_income: row.total_income,
            total_expense: row.total_expense,
            net_profit,
            allocation_count: row.allocation_count,
            period_start,
            period_end,
        })
    }
}

/// Database row for profitability report aggregation
#[derive(Debug, FromRow)]
struct ProfitabilityRow {
    total_income: Decimal,
    total_expense: Decimal,
    allocation_count: i64,
}
