//! PostgreSQL currency and exchange rate repository implementations

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::db::error::map_sqlx_error;
use crate::domain::currency::model::{
    CreateCurrency, CreateExchangeRate, Currency, ExchangeRate, UpdateCurrency, UpdateExchangeRate,
};
use crate::domain::currency::repository::{
    BoxCurrencyRepository, BoxExchangeRateRepository, CurrencyRepository, ExchangeRateRepository,
};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// CurrencyRow / Currency conversion
// ---------------------------------------------------------------------------

/// Database row representation for Currency
#[derive(Debug, FromRow)]
struct CurrencyRow {
    id: i64,
    tenant_id: i64,
    code: String,
    name: String,
    symbol: String,
    decimal_places: i32,
    is_active: bool,
    is_base: bool,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
    total_count: Option<i64>,
}

impl From<CurrencyRow> for Currency {
    fn from(row: CurrencyRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            code: row.code,
            name: row.name,
            symbol: row.symbol,
            decimal_places: row.decimal_places,
            is_active: row.is_active,
            is_base: row.is_base,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

// ---------------------------------------------------------------------------
// ExchangeRateRow / ExchangeRate conversion
// ---------------------------------------------------------------------------

/// Database row representation for ExchangeRate
#[derive(Debug, FromRow)]
struct ExchangeRateRow {
    id: i64,
    tenant_id: i64,
    from_currency: String,
    to_currency: String,
    rate: Decimal,
    effective_date: NaiveDate,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
    total_count: Option<i64>,
}

impl From<ExchangeRateRow> for ExchangeRate {
    fn from(row: ExchangeRateRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            from_currency: row.from_currency,
            to_currency: row.to_currency,
            rate: row.rate,
            effective_date: row.effective_date,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

// ---------------------------------------------------------------------------
// PostgresCurrencyRepository
// ---------------------------------------------------------------------------

/// PostgreSQL implementation of CurrencyRepository
#[derive(Clone)]
pub struct PostgresCurrencyRepository {
    pool: Arc<PgPool>,
}

impl PostgresCurrencyRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxCurrencyRepository {
        Arc::new(self) as BoxCurrencyRepository
    }
}

#[async_trait]
impl CurrencyRepository for PostgresCurrencyRepository {
    async fn create(&self, create: CreateCurrency, tenant_id: i64) -> Result<Currency, ApiError> {
        let code = create.code.trim().to_uppercase();
        let row = sqlx::query_as::<_, CurrencyRow>(
            r#"
            INSERT INTO currencies (tenant_id, code, name, symbol, decimal_places, is_active, is_base)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, tenant_id, code, name, symbol, decimal_places, is_active, is_base,
                      created_at, updated_at, deleted_at, deleted_by, NULL::bigint as total_count
            "#,
        )
        .bind(tenant_id)
        .bind(&code)
        .bind(&create.name)
        .bind(&create.symbol)
        .bind(create.decimal_places)
        .bind(create.is_active)
        .bind(create.is_base)
        .fetch_one(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Currency>, ApiError> {
        let row = sqlx::query_as::<_, CurrencyRow>(
            r#"
            SELECT id, tenant_id, code, name, symbol, decimal_places, is_active, is_base,
                   created_at, updated_at, deleted_at, deleted_by, NULL::bigint as total_count
            FROM currencies
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        Ok(row.map(Into::into))
    }

    async fn find_by_code(&self, code: &str, tenant_id: i64) -> Result<Option<Currency>, ApiError> {
        let row = sqlx::query_as::<_, CurrencyRow>(
            r#"
            SELECT id, tenant_id, code, name, symbol, decimal_places, is_active, is_base,
                   created_at, updated_at, deleted_at, deleted_by, NULL::bigint as total_count
            FROM currencies
            WHERE code = UPPER($1) AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(code)
        .bind(tenant_id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        Ok(row.map(Into::into))
    }

    async fn find_base(&self, tenant_id: i64) -> Result<Option<Currency>, ApiError> {
        let row = sqlx::query_as::<_, CurrencyRow>(
            r#"
            SELECT id, tenant_id, code, name, symbol, decimal_places, is_active, is_base,
                   created_at, updated_at, deleted_at, deleted_by, NULL::bigint as total_count
            FROM currencies
            WHERE tenant_id = $1 AND is_base = TRUE AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        Ok(row.map(Into::into))
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        active_only: Option<bool>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<Currency>, ApiError> {
        let offset = (params.page.saturating_sub(1)) * params.per_page;

        let rows = sqlx::query_as::<_, CurrencyRow>(
            r#"
            SELECT id, tenant_id, code, name, symbol, decimal_places, is_active, is_base,
                   created_at, updated_at, deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM currencies
            WHERE tenant_id = $1
              AND deleted_at IS NULL
              AND ($2::bool IS NULL OR is_active = $2)
            ORDER BY code
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(active_only)
        .bind(params.per_page as i64)
        .bind(offset as i64)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<Currency> = rows.into_iter().map(Into::into).collect();

        Ok(PaginatedResult::new(
            items,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCurrency,
    ) -> Result<Currency, ApiError> {
        let row = sqlx::query_as::<_, CurrencyRow>(
            r#"
            UPDATE currencies
            SET name = COALESCE($1, name),
                symbol = COALESCE($2, symbol),
                decimal_places = COALESCE($3, decimal_places),
                is_active = COALESCE($4, is_active),
                is_base = COALESCE($5, is_base),
                updated_at = NOW()
            WHERE id = $6 AND tenant_id = $7 AND deleted_at IS NULL
            RETURNING id, tenant_id, code, name, symbol, decimal_places, is_active, is_base,
                      created_at, updated_at, deleted_at, deleted_by, NULL::bigint as total_count
            "#,
        )
        .bind(update.name)
        .bind(update.symbol)
        .bind(update.decimal_places)
        .bind(update.is_active)
        .bind(update.is_base)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query("DELETE FROM currencies WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| map_sqlx_error(e, "currency"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Currency {} not found", id)));
        }
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE currencies
            SET deleted_at = NOW(), deleted_by = $3, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Currency {} not found", id)));
        }
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE currencies
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted currency {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Currency>, ApiError> {
        let rows = sqlx::query_as::<_, CurrencyRow>(
            r#"
            SELECT id, tenant_id, code, name, symbol, decimal_places, is_active, is_base,
                   created_at, updated_at, deleted_at, deleted_by, NULL::bigint as total_count
            FROM currencies
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM currencies
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted currency {} not found",
                id
            )));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PostgresExchangeRateRepository
// ---------------------------------------------------------------------------

/// PostgreSQL implementation of ExchangeRateRepository
#[derive(Clone)]
pub struct PostgresExchangeRateRepository {
    pool: Arc<PgPool>,
}

impl PostgresExchangeRateRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxExchangeRateRepository {
        Arc::new(self) as BoxExchangeRateRepository
    }
}

#[async_trait]
impl ExchangeRateRepository for PostgresExchangeRateRepository {
    async fn create(
        &self,
        create: CreateExchangeRate,
        tenant_id: i64,
    ) -> Result<ExchangeRate, ApiError> {
        let from_currency = create.from_currency.trim().to_uppercase();
        let to_currency = create.to_currency.trim().to_uppercase();

        let row = sqlx::query_as::<_, ExchangeRateRow>(
            r#"
            INSERT INTO exchange_rates (tenant_id, from_currency, to_currency, rate, effective_date)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, tenant_id, from_currency, to_currency, rate, effective_date,
                      created_at, deleted_at, deleted_by, NULL::bigint as total_count
            "#,
        )
        .bind(tenant_id)
        .bind(&from_currency)
        .bind(&to_currency)
        .bind(create.rate)
        .bind(create.effective_date)
        .fetch_one(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ExchangeRate>, ApiError> {
        let row = sqlx::query_as::<_, ExchangeRateRow>(
            r#"
            SELECT id, tenant_id, from_currency, to_currency, rate, effective_date,
                   created_at, deleted_at, deleted_by, NULL::bigint as total_count
            FROM exchange_rates
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        Ok(row.map(Into::into))
    }

    async fn find_effective_rate(
        &self,
        from: &str,
        to: &str,
        date: NaiveDate,
        tenant_id: i64,
    ) -> Result<Option<ExchangeRate>, ApiError> {
        let row = sqlx::query_as::<_, ExchangeRateRow>(
            r#"
            SELECT id, tenant_id, from_currency, to_currency, rate, effective_date,
                   created_at, deleted_at, deleted_by, NULL::bigint as total_count
            FROM exchange_rates
            WHERE tenant_id = $1
              AND from_currency = UPPER($2)
              AND to_currency = UPPER($3)
              AND effective_date <= $4
              AND deleted_at IS NULL
            ORDER BY effective_date DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(from)
        .bind(to)
        .bind(date)
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        Ok(row.map(Into::into))
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        currency: Option<String>,
        date: Option<NaiveDate>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ExchangeRate>, ApiError> {
        let offset = (params.page.saturating_sub(1)) * params.per_page;

        let rows = sqlx::query_as::<_, ExchangeRateRow>(
            r#"
            SELECT id, tenant_id, from_currency, to_currency, rate, effective_date,
                   created_at, deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM exchange_rates
            WHERE tenant_id = $1
              AND deleted_at IS NULL
              AND ($2::text IS NULL OR from_currency = UPPER($2) OR to_currency = UPPER($2))
              AND ($3::date IS NULL OR effective_date = $3)
            ORDER BY from_currency, to_currency, effective_date DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(tenant_id)
        .bind(currency)
        .bind(date)
        .bind(params.per_page as i64)
        .bind(offset as i64)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<ExchangeRate> = rows.into_iter().map(Into::into).collect();

        Ok(PaginatedResult::new(
            items,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn list_effective_on(
        &self,
        tenant_id: i64,
        date: NaiveDate,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ExchangeRate>, ApiError> {
        let offset = (params.page.saturating_sub(1)) * params.per_page;

        let rows = sqlx::query_as::<_, ExchangeRateRow>(
            r#"
            SELECT DISTINCT ON (from_currency, to_currency)
                   id, tenant_id, from_currency, to_currency, rate, effective_date,
                   created_at, deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM exchange_rates
            WHERE tenant_id = $1
              AND effective_date <= $2
              AND deleted_at IS NULL
            ORDER BY from_currency, to_currency, effective_date DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(date)
        .bind(params.per_page as i64)
        .bind(offset as i64)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<ExchangeRate> = rows.into_iter().map(Into::into).collect();

        Ok(PaginatedResult::new(
            items,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateExchangeRate,
    ) -> Result<ExchangeRate, ApiError> {
        let row = sqlx::query_as::<_, ExchangeRateRow>(
            r#"
            UPDATE exchange_rates
            SET rate = COALESCE($1, rate),
                effective_date = COALESCE($2, effective_date)
            WHERE id = $3 AND tenant_id = $4 AND deleted_at IS NULL
            RETURNING id, tenant_id, from_currency, to_currency, rate, effective_date,
                      created_at, deleted_at, deleted_by, NULL::bigint as total_count
            "#,
        )
        .bind(update.rate)
        .bind(update.effective_date)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query("DELETE FROM exchange_rates WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(self.pool.as_ref())
            .await
            .map_err(|e| map_sqlx_error(e, "currency"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Exchange rate {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE exchange_rates
            SET deleted_at = NOW(), deleted_by = $3, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Exchange rate {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE exchange_rates
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted exchange rate {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<ExchangeRate>, ApiError> {
        let rows = sqlx::query_as::<_, ExchangeRateRow>(
            r#"
            SELECT id, tenant_id, from_currency, to_currency, rate, effective_date,
                   created_at, deleted_at, deleted_by, NULL::bigint as total_count
            FROM exchange_rates
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM exchange_rates
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| map_sqlx_error(e, "currency"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!(
                "Deleted exchange rate {} not found",
                id
            )));
        }
        Ok(())
    }
}
