//! PostgreSQL cari repository implementation

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::cari::model::{Cari, CariStatus, CariType, CreateCari, UpdateCari};
use crate::domain::cari::repository::{BoxCariRepository, CariRepository};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

/// Database row representation for Cari
#[derive(Debug, FromRow)]
struct CariRow {
    id: i64,
    code: String,
    name: String,
    cari_type: String,
    tax_number: Option<String>,
    tax_office: Option<String>,
    identity_number: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    city: Option<String>,
    country: Option<String>,
    postal_code: Option<String>,
    credit_limit: Decimal,
    current_balance: Decimal,
    status: String,
    tenant_id: i64,
    created_by: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<CariRow> for Cari {
    fn from(row: CariRow) -> Self {
        // Parse cari_type with warning for invalid values
        let cari_type = row.cari_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid cari_type '{}' in database: {}, defaulting to Customer",
                row.cari_type,
                e
            );
            CariType::default()
        });

        // Parse status with warning for invalid values
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid status '{}' in database: {}, defaulting to Active",
                row.status,
                e
            );
            CariStatus::default()
        });

        Self {
            id: row.id,
            code: row.code,
            name: row.name,
            cari_type,
            tax_number: row.tax_number,
            tax_office: row.tax_office,
            identity_number: row.identity_number,
            email: row.email,
            phone: row.phone,
            address: row.address,
            city: row.city,
            country: row.country,
            postal_code: row.postal_code,
            credit_limit: row.credit_limit,
            current_balance: row.current_balance,
            status,
            tenant_id: row.tenant_id,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Database row representation for paginated cari queries with total count
#[derive(Debug, FromRow)]
struct CariRowWithTotal {
    id: i64,
    code: String,
    name: String,
    cari_type: String,
    tax_number: Option<String>,
    tax_office: Option<String>,
    identity_number: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    city: Option<String>,
    country: Option<String>,
    postal_code: Option<String>,
    credit_limit: Decimal,
    current_balance: Decimal,
    status: String,
    tenant_id: i64,
    created_by: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    total_count: i64,
}

impl From<CariRowWithTotal> for (Cari, i64) {
    fn from(row: CariRowWithTotal) -> (Cari, i64) {
        let cari_type = row.cari_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid cari_type '{}' in database: {}, defaulting to Customer",
                row.cari_type,
                e
            );
            CariType::default()
        });

        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid status '{}' in database: {}, defaulting to Active",
                row.status,
                e
            );
            CariStatus::default()
        });

        let cari = Cari {
            id: row.id,
            code: row.code,
            name: row.name,
            cari_type,
            tax_number: row.tax_number,
            tax_office: row.tax_office,
            identity_number: row.identity_number,
            email: row.email,
            phone: row.phone,
            address: row.address,
            city: row.city,
            country: row.country,
            postal_code: row.postal_code,
            credit_limit: row.credit_limit,
            current_balance: row.current_balance,
            status,
            tenant_id: row.tenant_id,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };
        (cari, row.total_count)
    }
}

/// PostgreSQL cari repository
pub struct PostgresCariRepository {
    pool: Arc<PgPool>,
}

impl PostgresCariRepository {
    /// Create a new PostgreSQL cari repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxCariRepository {
        Arc::new(self) as BoxCariRepository
    }
}

#[async_trait]
impl CariRepository for PostgresCariRepository {
    async fn create(&self, create: CreateCari) -> Result<Cari, ApiError> {
        let cari_type = create.cari_type.to_string();
        let status = CariStatus::default().to_string();

        let row: CariRow = sqlx::query_as(
            r#"
            INSERT INTO cari (code, name, cari_type, tax_number, tax_office, identity_number,
                              email, phone, address, city, country, postal_code,
                              credit_limit, current_balance, status, tenant_id, created_by, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, NOW())
            RETURNING id, code, name, cari_type, tax_number, tax_office, identity_number,
                      email, phone, address, city, country, postal_code,
                      credit_limit, current_balance, status, tenant_id, created_by, created_at, updated_at
            "#,
        )
        .bind(&create.code)
        .bind(&create.name)
        .bind(&cari_type)
        .bind(&create.tax_number)
        .bind(&create.tax_office)
        .bind(&create.identity_number)
        .bind(&create.email)
        .bind(&create.phone)
        .bind(&create.address)
        .bind(&create.city)
        .bind(&create.country)
        .bind(&create.postal_code)
        .bind(create.credit_limit)
        .bind(Decimal::ZERO)
        .bind(&status)
        .bind(create.tenant_id)
        .bind(create.created_by)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Cari"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Cari>, ApiError> {
        let result: Option<CariRow> = sqlx::query_as(
            r#"
            SELECT id, code, name, cari_type, tax_number, tax_office, identity_number,
                   email, phone, address, city, country, postal_code,
                   credit_limit, current_balance, status, tenant_id, created_by, created_at, updated_at
            FROM cari
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find cari by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_code(&self, code: &str, tenant_id: i64) -> Result<Option<Cari>, ApiError> {
        let result: Option<CariRow> = sqlx::query_as(
            r#"
            SELECT id, code, name, cari_type, tax_number, tax_office, identity_number,
                   email, phone, address, city, country, postal_code,
                   credit_limit, current_balance, status, tenant_id, created_by, created_at, updated_at
            FROM cari
            WHERE code = $1 AND tenant_id = $2
            "#,
        )
        .bind(code)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find cari by code: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_all(&self, tenant_id: i64) -> Result<Vec<Cari>, ApiError> {
        let rows: Vec<CariRow> = sqlx::query_as(
            r#"
            SELECT id, code, name, cari_type, tax_number, tax_office, identity_number,
                   email, phone, address, city, country, postal_code,
                   credit_limit, current_balance, status, tenant_id, created_by, created_at, updated_at
            FROM cari
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find all cari: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_type(
        &self,
        cari_type: CariType,
        tenant_id: i64,
    ) -> Result<Vec<Cari>, ApiError> {
        let cari_type_str = cari_type.to_string();

        let rows: Vec<CariRow> = sqlx::query_as(
            r#"
            SELECT id, code, name, cari_type, tax_number, tax_office, identity_number,
                   email, phone, address, city, country, postal_code,
                   credit_limit, current_balance, status, tenant_id, created_by, created_at, updated_at
            FROM cari
            WHERE tenant_id = $1 AND cari_type = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&cari_type_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find cari by type: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn search(&self, query: &str, tenant_id: i64) -> Result<Vec<Cari>, ApiError> {
        let pattern = format!("%{}%", query);

        let rows: Vec<CariRow> = sqlx::query_as(
            r#"
            SELECT id, code, name, cari_type, tax_number, tax_office, identity_number,
                   email, phone, address, city, country, postal_code,
                   credit_limit, current_balance, status, tenant_id, created_by, created_at, updated_at
            FROM cari
            WHERE tenant_id = $1
              AND (LOWER(code) LIKE LOWER($2) OR LOWER(name) LIKE LOWER($2))
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&pattern)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to search cari: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Cari>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;

        let rows: Vec<CariRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, code, name, cari_type, tax_number, tax_office, identity_number,
                   email, phone, address, city, country, postal_code,
                   credit_limit, current_balance, status, tenant_id, created_by, created_at, updated_at,
                   COUNT(*) OVER() as total_count
            FROM cari
            WHERE tenant_id = $1
            ORDER BY id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Cari"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<Cari> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(cari, _)| cari)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_type_paginated(
        &self,
        cari_type: CariType,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Cari>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;
        let cari_type_str = cari_type.to_string();

        let rows: Vec<CariRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, code, name, cari_type, tax_number, tax_office, identity_number,
                   email, phone, address, city, country, postal_code,
                   credit_limit, current_balance, status, tenant_id, created_by, created_at, updated_at,
                   COUNT(*) OVER() as total_count
            FROM cari
            WHERE tenant_id = $1 AND cari_type = $2
            ORDER BY id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(&cari_type_str)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Cari"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<Cari> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(cari, _)| cari)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn search_paginated(
        &self,
        query: &str,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Cari>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;
        let pattern = format!("%{}%", query);

        let rows: Vec<CariRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, code, name, cari_type, tax_number, tax_office, identity_number,
                   email, phone, address, city, country, postal_code,
                   credit_limit, current_balance, status, tenant_id, created_by, created_at, updated_at,
                   COUNT(*) OVER() as total_count
            FROM cari
            WHERE tenant_id = $1
              AND (LOWER(code) LIKE LOWER($2) OR LOWER(name) LIKE LOWER($2))
            ORDER BY id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(&pattern)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Cari"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<Cari> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(cari, _)| cari)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(&self, id: i64, tenant_id: i64, update: UpdateCari) -> Result<Cari, ApiError> {
        let cari_type_str = update.cari_type.map(|t| t.to_string());
        let status_str = update.status.map(|s| s.to_string());

        let row: CariRow = sqlx::query_as(
            r#"
            UPDATE cari
            SET
                code = COALESCE($1, code),
                name = COALESCE($2, name),
                cari_type = COALESCE($3, cari_type),
                tax_number = COALESCE($4, tax_number),
                tax_office = COALESCE($5, tax_office),
                identity_number = COALESCE($6, identity_number),
                email = COALESCE($7, email),
                phone = COALESCE($8, phone),
                address = COALESCE($9, address),
                city = COALESCE($10, city),
                country = COALESCE($11, country),
                postal_code = COALESCE($12, postal_code),
                credit_limit = COALESCE($13, credit_limit),
                status = COALESCE($14, status),
                updated_at = NOW()
            WHERE id = $15 AND tenant_id = $16
            RETURNING id, code, name, cari_type, tax_number, tax_office, identity_number,
                      email, phone, address, city, country, postal_code,
                      credit_limit, current_balance, status, tenant_id, created_by, created_at, updated_at
            "#,
        )
        .bind(&update.code)
        .bind(&update.name)
        .bind(&cari_type_str)
        .bind(&update.tax_number)
        .bind(&update.tax_office)
        .bind(&update.identity_number)
        .bind(&update.email)
        .bind(&update.phone)
        .bind(&update.address)
        .bind(&update.city)
        .bind(&update.country)
        .bind(&update.postal_code)
        .bind(update.credit_limit)
        .bind(&status_str)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Cari"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM cari
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete cari: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Cari not found".to_string()));
        }

        Ok(())
    }

    async fn code_exists(&self, code: &str, tenant_id: i64) -> Result<bool, ApiError> {
        let result: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(SELECT 1 FROM cari WHERE code = $1 AND tenant_id = $2)
            "#,
        )
        .bind(code)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to check cari code: {}", e)))?;

        Ok(result.0)
    }

    async fn update_balance(
        &self,
        id: i64,
        tenant_id: i64,
        amount: Decimal,
    ) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE cari
            SET current_balance = current_balance + $1,
                updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            "#,
        )
        .bind(amount)
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to update cari balance: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Cari not found".to_string()));
        }

        Ok(())
    }
}
