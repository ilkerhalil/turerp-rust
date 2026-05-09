//! PostgreSQL company repository implementation

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::company::model::{Company, CreateCompany, UpdateCompany};
use crate::domain::company::repository::{BoxCompanyRepository, CompanyRepository};
use crate::error::ApiError;

#[derive(Debug, FromRow)]
struct CompanyRow {
    id: i64,
    tenant_id: i64,
    code: String,
    name: String,
    tax_number: Option<String>,
    address: Option<String>,
    city: Option<String>,
    country: Option<String>,
    currency: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<CompanyRow> for Company {
    fn from(row: CompanyRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            code: row.code,
            name: row.name,
            tax_number: row.tax_number,
            address: row.address,
            city: row.city,
            country: row.country,
            currency: row.currency,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

#[derive(Debug, FromRow)]
struct CompanyRowWithTotal {
    id: i64,
    tenant_id: i64,
    code: String,
    name: String,
    tax_number: Option<String>,
    address: Option<String>,
    city: Option<String>,
    country: Option<String>,
    currency: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
    total_count: i64,
}

impl From<CompanyRowWithTotal> for (Company, i64) {
    fn from(row: CompanyRowWithTotal) -> (Company, i64) {
        let company = Company {
            id: row.id,
            tenant_id: row.tenant_id,
            code: row.code,
            name: row.name,
            tax_number: row.tax_number,
            address: row.address,
            city: row.city,
            country: row.country,
            currency: row.currency,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        };
        (company, row.total_count)
    }
}

pub struct PostgresCompanyRepository {
    pool: Arc<PgPool>,
}

impl PostgresCompanyRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
    pub fn into_boxed(self) -> BoxCompanyRepository {
        Arc::new(self) as BoxCompanyRepository
    }
}

#[async_trait]
impl CompanyRepository for PostgresCompanyRepository {
    async fn create(&self, create: CreateCompany) -> Result<Company, ApiError> {
        let row: CompanyRow = sqlx::query_as(
            r#"
            INSERT INTO companies (tenant_id, code, name, tax_number, address, city, country, currency, is_active, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true, NOW())
            RETURNING id, tenant_id, code, name, tax_number, address, city, country, currency, is_active, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.code)
        .bind(&create.name)
        .bind(&create.tax_number)
        .bind(&create.address)
        .bind(&create.city)
        .bind(&create.country)
        .bind(&create.currency)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Company"))?;
        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Company>, ApiError> {
        let row: Option<CompanyRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, tax_number, address, city, country, currency, is_active, created_at, updated_at, deleted_at, deleted_by
            FROM companies
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find company by id: {}", e)))?;
        Ok(row.map(|r| r.into()))
    }

    async fn find_by_code(&self, code: &str, tenant_id: i64) -> Result<Option<Company>, ApiError> {
        let row: Option<CompanyRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, tax_number, address, city, country, currency, is_active, created_at, updated_at, deleted_at, deleted_by
            FROM companies
            WHERE code = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(code)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find company by code: {}", e)))?;
        Ok(row.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Company>, ApiError> {
        let rows: Vec<CompanyRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, tax_number, address, city, country, currency, is_active, created_at, updated_at, deleted_at, deleted_by
            FROM companies
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find companies: {}", e)))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Company>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;
        let rows: Vec<CompanyRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, tax_number, address, city, country, currency, is_active, created_at, updated_at, deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM companies
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
        .map_err(|e| map_sqlx_error(e, "Company"))?;
        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items = rows.into_iter().map(|r| r.into()).map(|(c, _)| c).collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCompany,
    ) -> Result<Company, ApiError> {
        let row: CompanyRow = sqlx::query_as(
            r#"
            UPDATE companies
            SET code = COALESCE($1, code),
                name = COALESCE($2, name),
                tax_number = COALESCE($3, tax_number),
                address = COALESCE($4, address),
                city = COALESCE($5, city),
                country = COALESCE($6, country),
                currency = COALESCE($7, currency),
                is_active = COALESCE($8, is_active),
                updated_at = NOW()
            WHERE id = $9 AND tenant_id = $10 AND deleted_at IS NULL
            RETURNING id, tenant_id, code, name, tax_number, address, city, country, currency, is_active, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(&update.code)
        .bind(&update.name)
        .bind(&update.tax_number)
        .bind(&update.address)
        .bind(&update.city)
        .bind(&update.country)
        .bind(&update.currency)
        .bind(update.is_active)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Company"))?;
        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE companies
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete company: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Company not found".to_string()));
        }
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Company, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE companies
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore company: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Company not found or not deleted".to_string(),
            ));
        }
        self.find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Company not found".to_string()))
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Company>, ApiError> {
        let rows: Vec<CompanyRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, tax_number, address, city, country, currency, is_active, created_at, updated_at, deleted_at, deleted_by
            FROM companies
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted companies: {}", e)))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM companies
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy company: {}", e)))?;
        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Company not found".to_string()));
        }
        Ok(())
    }

    async fn code_exists(&self, code: &str, tenant_id: i64) -> Result<bool, ApiError> {
        let result: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(SELECT 1 FROM companies WHERE code = $1 AND tenant_id = $2 AND deleted_at IS NULL)
            "#,
        )
        .bind(code)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to check company code: {}", e)))?;
        Ok(result.0)
    }
}
