//! PostgreSQL tax rate and tax period repository implementations

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::db::error::map_sqlx_error;
use crate::domain::tax::model::{
    CreateTaxRate, TaxPeriod, TaxPeriodDetail, TaxPeriodStatus, TaxRate, TaxType, UpdateTaxRate,
};
use crate::domain::tax::repository::{
    BoxTaxPeriodRepository, BoxTaxRateRepository, TaxPeriodRepository, TaxRateRepository,
};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// TaxRateRow / TaxRate conversion
// ---------------------------------------------------------------------------

/// Database row representation for TaxRate
#[derive(Debug, FromRow)]
struct TaxRateRow {
    id: i64,
    tenant_id: i64,
    tax_type: String,
    rate: Decimal,
    effective_from: NaiveDate,
    effective_to: Option<NaiveDate>,
    category: Option<String>,
    description: String,
    is_default: bool,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
    total_count: Option<i64>,
}

impl From<TaxRateRow> for TaxRate {
    fn from(row: TaxRateRow) -> Self {
        let tax_type = row.tax_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid tax_type '{}' in database: {}, defaulting to KDV",
                row.tax_type,
                e
            );
            TaxType::KDV
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            tax_type,
            rate: row.rate,
            effective_from: row.effective_from,
            effective_to: row.effective_to,
            category: row.category,
            description: row.description,
            is_default: row.is_default,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

// ---------------------------------------------------------------------------
// TaxPeriodRow / TaxPeriod conversion
// ---------------------------------------------------------------------------

/// Database row representation for TaxPeriod
#[derive(Debug, FromRow)]
struct TaxPeriodRow {
    id: i64,
    tenant_id: i64,
    tax_type: String,
    period_year: i32,
    period_month: i32,
    total_base: Decimal,
    total_tax: Decimal,
    total_deduction: Decimal,
    net_tax: Decimal,
    status: String,
    filed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
    total_count: Option<i64>,
}

impl From<TaxPeriodRow> for TaxPeriod {
    fn from(row: TaxPeriodRow) -> Self {
        let tax_type = row.tax_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid tax_type '{}' in database: {}, defaulting to KDV",
                row.tax_type,
                e
            );
            TaxType::KDV
        });

        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid TaxPeriodStatus '{}' in database: {}, defaulting to Open",
                row.status,
                e
            );
            TaxPeriodStatus::Open
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            tax_type,
            period_year: row.period_year,
            period_month: row.period_month as u32,
            total_base: row.total_base,
            total_tax: row.total_tax,
            total_deduction: row.total_deduction,
            net_tax: row.net_tax,
            status,
            filed_at: row.filed_at,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

// ---------------------------------------------------------------------------
// TaxPeriodDetailRow / TaxPeriodDetail conversion
// ---------------------------------------------------------------------------

/// Database row representation for TaxPeriodDetail
#[derive(Debug, FromRow)]
struct TaxPeriodDetailRow {
    id: i64,
    period_id: i64,
    transaction_date: NaiveDate,
    transaction_type: String,
    base_amount: Decimal,
    tax_rate: Decimal,
    tax_amount: Decimal,
    deduction_amount: Decimal,
    reference_id: Option<i64>,
}

impl From<TaxPeriodDetailRow> for TaxPeriodDetail {
    fn from(row: TaxPeriodDetailRow) -> Self {
        Self {
            id: row.id,
            period_id: row.period_id,
            transaction_date: row.transaction_date,
            transaction_type: row.transaction_type,
            base_amount: row.base_amount,
            tax_rate: row.tax_rate,
            tax_amount: row.tax_amount,
            deduction_amount: row.deduction_amount,
            reference_id: row.reference_id,
        }
    }
}

// ===========================================================================
// PostgresTaxRateRepository
// ===========================================================================

/// PostgreSQL tax rate repository
pub struct PostgresTaxRateRepository {
    pool: Arc<PgPool>,
}

impl PostgresTaxRateRepository {
    /// Create a new PostgreSQL tax rate repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxTaxRateRepository {
        Arc::new(self) as BoxTaxRateRepository
    }
}

/// Common column list for tax_rates SELECT queries
const TAX_RATE_COLUMNS: &str = r#"
    id, tenant_id, tax_type, rate, effective_from, effective_to,
    category, description, is_default, created_at, deleted_at, deleted_by
"#;

#[async_trait]
impl TaxRateRepository for PostgresTaxRateRepository {
    async fn create(&self, create: CreateTaxRate, tenant_id: i64) -> Result<TaxRate, ApiError> {
        let tax_type = create.tax_type.to_string();

        let row: TaxRateRow = sqlx::query_as(&format!(
            r#"
            INSERT INTO tax_rates (tenant_id, tax_type, rate, effective_from, effective_to,
                                   category, description, is_default)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING {TAX_RATE_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(tenant_id)
        .bind(&tax_type)
        .bind(create.rate)
        .bind(create.effective_from)
        .bind(create.effective_to)
        .bind(&create.category)
        .bind(&create.description)
        .bind(create.is_default)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "TaxRate"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<TaxRate>, ApiError> {
        let result: Option<TaxRateRow> = sqlx::query_as(&format!(
            r#"
            SELECT {TAX_RATE_COLUMNS}, 0 as total_count
            FROM tax_rates
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find tax rate: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        tax_type: Option<TaxType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<TaxRate>, ApiError> {
        let offset = params.offset() as i64;
        let per_page = params.per_page as i64;

        match tax_type {
            Some(tt) => {
                let tax_type_str = tt.to_string();
                let rows: Vec<TaxRateRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {TAX_RATE_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM tax_rates
                    WHERE tenant_id = $1 AND tax_type = $2 AND deleted_at IS NULL
                    ORDER BY effective_from DESC
                    LIMIT $3 OFFSET $4
                    "#,
                ))
                .bind(tenant_id)
                .bind(&tax_type_str)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "TaxRate"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<TaxRate> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
            None => {
                let rows: Vec<TaxRateRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {TAX_RATE_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM tax_rates
                    WHERE tenant_id = $1 AND deleted_at IS NULL
                    ORDER BY effective_from DESC
                    LIMIT $2 OFFSET $3
                    "#,
                ))
                .bind(tenant_id)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "TaxRate"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<TaxRate> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
        }
    }

    async fn find_effective(
        &self,
        tax_type: TaxType,
        date: NaiveDate,
        tenant_id: i64,
    ) -> Result<Option<TaxRate>, ApiError> {
        let tax_type_str = tax_type.to_string();

        let result: Option<TaxRateRow> = sqlx::query_as(&format!(
            r#"
            SELECT {TAX_RATE_COLUMNS}, 0 as total_count
            FROM tax_rates
            WHERE tenant_id = $1
              AND tax_type = $2
              AND effective_from <= $3
              AND (effective_to IS NULL OR effective_to >= $3)
              AND deleted_at IS NULL
            ORDER BY effective_from DESC
            LIMIT 1
            "#,
        ))
        .bind(tenant_id)
        .bind(&tax_type_str)
        .bind(date)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find effective tax rate: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateTaxRate,
    ) -> Result<TaxRate, ApiError> {
        let row: TaxRateRow = sqlx::query_as(&format!(
            r#"
            UPDATE tax_rates
            SET
                rate = COALESCE($1, rate),
                effective_to = COALESCE($2, effective_to),
                category = COALESCE($3, category),
                description = COALESCE($4, description),
                is_default = COALESCE($5, is_default)
            WHERE id = $6 AND tenant_id = $7 AND deleted_at IS NULL
            RETURNING {TAX_RATE_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(update.rate)
        .bind(update.effective_to)
        .bind(&update.category)
        .bind(&update.description)
        .bind(update.is_default)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "TaxRate"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE tax_rates
            SET deleted_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete tax rate: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Tax rate not found".to_string()));
        }

        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE tax_rates
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete tax rate: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Tax rate not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<TaxRate, ApiError> {
        let row: TaxRateRow = sqlx::query_as(&format!(
            r#"
            UPDATE tax_rates
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING {TAX_RATE_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "TaxRate"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<TaxRate>, ApiError> {
        let rows: Vec<TaxRateRow> = sqlx::query_as(&format!(
            r#"
            SELECT {TAX_RATE_COLUMNS}, 0 as total_count
            FROM tax_rates
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        ))
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted tax rates: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM tax_rates
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy tax rate: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Deleted tax rate not found".to_string()));
        }

        Ok(())
    }
}

// ===========================================================================
// PostgresTaxPeriodRepository
// ===========================================================================

/// PostgreSQL tax period repository
pub struct PostgresTaxPeriodRepository {
    pool: Arc<PgPool>,
}

impl PostgresTaxPeriodRepository {
    /// Create a new PostgreSQL tax period repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxTaxPeriodRepository {
        Arc::new(self) as BoxTaxPeriodRepository
    }
}

/// Common column list for tax_periods SELECT queries
const TAX_PERIOD_COLUMNS: &str = r#"
    id, tenant_id, tax_type, period_year, period_month,
    total_base, total_tax, total_deduction, net_tax,
    status, filed_at, created_at, deleted_at, deleted_by
"#;

#[async_trait]
impl TaxPeriodRepository for PostgresTaxPeriodRepository {
    async fn create(
        &self,
        tax_type: TaxType,
        year: i32,
        month: u32,
        tenant_id: i64,
    ) -> Result<TaxPeriod, ApiError> {
        let tax_type_str = tax_type.to_string();

        let row: TaxPeriodRow = sqlx::query_as(&format!(
            r#"
            INSERT INTO tax_periods (tenant_id, tax_type, period_year, period_month)
            VALUES ($1, $2, $3, $4)
            RETURNING {TAX_PERIOD_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(tenant_id)
        .bind(&tax_type_str)
        .bind(year)
        .bind(month as i32)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "TaxPeriod"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<TaxPeriod>, ApiError> {
        let result: Option<TaxPeriodRow> = sqlx::query_as(&format!(
            r#"
            SELECT {TAX_PERIOD_COLUMNS}, 0 as total_count
            FROM tax_periods
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find tax period: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        tax_type: Option<TaxType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<TaxPeriod>, ApiError> {
        let offset = params.offset() as i64;
        let per_page = params.per_page as i64;

        match tax_type {
            Some(tt) => {
                let tax_type_str = tt.to_string();
                let rows: Vec<TaxPeriodRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {TAX_PERIOD_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM tax_periods
                    WHERE tenant_id = $1 AND tax_type = $2 AND deleted_at IS NULL
                    ORDER BY period_year DESC, period_month DESC
                    LIMIT $3 OFFSET $4
                    "#,
                ))
                .bind(tenant_id)
                .bind(&tax_type_str)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "TaxPeriod"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<TaxPeriod> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
            None => {
                let rows: Vec<TaxPeriodRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {TAX_PERIOD_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM tax_periods
                    WHERE tenant_id = $1 AND deleted_at IS NULL
                    ORDER BY period_year DESC, period_month DESC
                    LIMIT $2 OFFSET $3
                    "#,
                ))
                .bind(tenant_id)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "TaxPeriod"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<TaxPeriod> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
        }
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        period: TaxPeriod,
    ) -> Result<TaxPeriod, ApiError> {
        let tax_type_str = period.tax_type.to_string();
        let status_str = period.status.to_string();

        let row: TaxPeriodRow = sqlx::query_as(&format!(
            r#"
            UPDATE tax_periods
            SET tax_type = $1,
                period_year = $2,
                period_month = $3,
                total_base = $4,
                total_tax = $5,
                total_deduction = $6,
                net_tax = $7,
                status = $8,
                filed_at = $9
            WHERE id = $10 AND tenant_id = $11 AND deleted_at IS NULL
            RETURNING {TAX_PERIOD_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(&tax_type_str)
        .bind(period.period_year)
        .bind(period.period_month as i32)
        .bind(period.total_base)
        .bind(period.total_tax)
        .bind(period.total_deduction)
        .bind(period.net_tax)
        .bind(&status_str)
        .bind(period.filed_at)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "TaxPeriod"))?;

        Ok(row.into())
    }

    async fn add_detail(&self, detail: TaxPeriodDetail) -> Result<TaxPeriodDetail, ApiError> {
        let row: TaxPeriodDetailRow = sqlx::query_as(
            r#"
            INSERT INTO tax_period_details (period_id, transaction_date, transaction_type,
                                            base_amount, tax_rate, tax_amount, deduction_amount, reference_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, period_id, transaction_date, transaction_type,
                      base_amount, tax_rate, tax_amount, deduction_amount, reference_id
            "#,
        )
        .bind(detail.period_id)
        .bind(detail.transaction_date)
        .bind(&detail.transaction_type)
        .bind(detail.base_amount)
        .bind(detail.tax_rate)
        .bind(detail.tax_amount)
        .bind(detail.deduction_amount)
        .bind(detail.reference_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "TaxPeriodDetail"))?;

        Ok(row.into())
    }

    async fn get_details(&self, period_id: i64) -> Result<Vec<TaxPeriodDetail>, ApiError> {
        let rows: Vec<TaxPeriodDetailRow> = sqlx::query_as(
            r#"
            SELECT id, period_id, transaction_date, transaction_type,
                   base_amount, tax_rate, tax_amount, deduction_amount, reference_id
            FROM tax_period_details
            WHERE period_id = $1
            ORDER BY transaction_date ASC, id ASC
            "#,
        )
        .bind(period_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get tax period details: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE tax_periods
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete tax period: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Tax period not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<TaxPeriod, ApiError> {
        let row: TaxPeriodRow = sqlx::query_as(&format!(
            r#"
            UPDATE tax_periods
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING {TAX_PERIOD_COLUMNS}, 0 as total_count
            "#,
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "TaxPeriod"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<TaxPeriod>, ApiError> {
        let rows: Vec<TaxPeriodRow> = sqlx::query_as(&format!(
            r#"
            SELECT {TAX_PERIOD_COLUMNS}, 0 as total_count
            FROM tax_periods
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        ))
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted tax periods: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM tax_periods
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy tax period: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Deleted tax period not found".to_string(),
            ));
        }

        Ok(())
    }
}
