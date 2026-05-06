//! PostgreSQL invoice repository implementation

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::invoice::model::{
    CreateInvoice, CreateInvoiceLine, CreatePayment, Invoice, InvoiceLine, InvoiceStatus,
    InvoiceType, Payment,
};
use crate::domain::invoice::repository::{
    BoxInvoiceLineRepository, BoxInvoiceRepository, BoxPaymentRepository, InvoiceLineRepository,
    InvoiceRepository, PaymentRepository,
};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

/// Database row representation for Invoice
#[derive(Debug, FromRow)]
struct InvoiceRow {
    id: i64,
    tenant_id: i64,
    invoice_number: String,
    invoice_type: String,
    status: String,
    cari_id: i64,
    issue_date: chrono::DateTime<chrono::Utc>,
    due_date: chrono::DateTime<chrono::Utc>,
    subtotal: Decimal,
    tax_amount: Decimal,
    discount_amount: Decimal,
    total_amount: Decimal,
    paid_amount: Decimal,
    currency: String,
    exchange_rate: Decimal,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<InvoiceRow> for Invoice {
    fn from(row: InvoiceRow) -> Self {
        let invoice_type = row.invoice_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid invoice_type '{}' in database: {}, defaulting to SalesInvoice",
                row.invoice_type,
                e
            );
            InvoiceType::SalesInvoice
        });

        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            InvoiceStatus::Draft
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            invoice_number: row.invoice_number,
            invoice_type,
            status,
            cari_id: row.cari_id,
            issue_date: row.issue_date,
            due_date: row.due_date,
            subtotal: row.subtotal,
            tax_amount: row.tax_amount,
            discount_amount: row.discount_amount,
            total_amount: row.total_amount,
            paid_amount: row.paid_amount,
            currency: row.currency,
            exchange_rate: row.exchange_rate,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at.unwrap_or(row.created_at),
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// Database row representation for paginated invoice queries with total count
#[derive(Debug, FromRow)]
struct InvoiceRowWithTotal {
    id: i64,
    tenant_id: i64,
    invoice_number: String,
    invoice_type: String,
    status: String,
    cari_id: i64,
    issue_date: chrono::DateTime<chrono::Utc>,
    due_date: chrono::DateTime<chrono::Utc>,
    subtotal: Decimal,
    tax_amount: Decimal,
    discount_amount: Decimal,
    total_amount: Decimal,
    paid_amount: Decimal,
    currency: String,
    exchange_rate: Decimal,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
    total_count: i64,
}

impl From<InvoiceRowWithTotal> for (Invoice, i64) {
    fn from(row: InvoiceRowWithTotal) -> (Invoice, i64) {
        let invoice_type = row.invoice_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid invoice_type '{}' in database: {}, defaulting to SalesInvoice",
                row.invoice_type,
                e
            );
            InvoiceType::SalesInvoice
        });

        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            InvoiceStatus::Draft
        });

        let invoice = Invoice {
            id: row.id,
            tenant_id: row.tenant_id,
            invoice_number: row.invoice_number,
            invoice_type,
            status,
            cari_id: row.cari_id,
            issue_date: row.issue_date,
            due_date: row.due_date,
            subtotal: row.subtotal,
            tax_amount: row.tax_amount,
            discount_amount: row.discount_amount,
            total_amount: row.total_amount,
            paid_amount: row.paid_amount,
            currency: row.currency,
            exchange_rate: row.exchange_rate,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at.unwrap_or(row.created_at),
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        };
        (invoice, row.total_count)
    }
}

/// Database row representation for InvoiceLine
#[derive(Debug, FromRow)]
struct InvoiceLineRow {
    id: i64,
    invoice_id: i64,
    product_id: Option<i64>,
    description: String,
    quantity: Decimal,
    unit_price: Decimal,
    tax_rate: Decimal,
    discount_rate: Decimal,
    line_total: Decimal,
    sort_order: i32,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<InvoiceLineRow> for InvoiceLine {
    fn from(row: InvoiceLineRow) -> Self {
        Self {
            id: row.id,
            invoice_id: row.invoice_id,
            product_id: row.product_id,
            description: row.description,
            quantity: row.quantity,
            unit_price: row.unit_price,
            tax_rate: row.tax_rate,
            discount_rate: row.discount_rate,
            line_total: row.line_total,
            sort_order: row.sort_order,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// Database row representation for Payment
#[derive(Debug, FromRow)]
struct PaymentRow {
    id: i64,
    tenant_id: i64,
    invoice_id: i64,
    amount: Decimal,
    currency: String,
    payment_date: chrono::DateTime<chrono::Utc>,
    payment_method: String,
    reference_number: Option<String>,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<PaymentRow> for Payment {
    fn from(row: PaymentRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            invoice_id: row.invoice_id,
            amount: row.amount,
            currency: row.currency,
            payment_date: row.payment_date,
            payment_method: row.payment_method,
            reference_number: row.reference_number,
            notes: row.notes,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL invoice repository
pub struct PostgresInvoiceRepository {
    pool: Arc<PgPool>,
}

impl PostgresInvoiceRepository {
    /// Create a new PostgreSQL invoice repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxInvoiceRepository {
        Arc::new(self) as BoxInvoiceRepository
    }
}

#[async_trait]
impl InvoiceRepository for PostgresInvoiceRepository {
    async fn create(&self, create: CreateInvoice) -> Result<Invoice, ApiError> {
        // Calculate totals from lines
        let mut subtotal = Decimal::ZERO;
        let mut tax_amount = Decimal::ZERO;
        let mut discount_amount = Decimal::ZERO;

        for line in &create.lines {
            let line_subtotal = line.quantity * line.unit_price;
            let line_discount = line_subtotal * (line.discount_rate / Decimal::ONE_HUNDRED);
            let after_discount = line_subtotal - line_discount;
            let line_tax = after_discount * (line.tax_rate / Decimal::ONE_HUNDRED);

            subtotal += line_subtotal;
            discount_amount += line_discount;
            tax_amount += line_tax;
        }

        let total_amount = subtotal - discount_amount + tax_amount;

        let invoice_number = format!("INV-{}", chrono::Utc::now().timestamp());
        let invoice_type_str = create.invoice_type.to_string();
        let status_str = InvoiceStatus::Draft.to_string();

        let row: InvoiceRow = sqlx::query_as(
            r#"
            INSERT INTO invoices (tenant_id, invoice_number, invoice_type, status, cari_id,
                                  issue_date, due_date, subtotal, tax_amount, discount_amount,
                                  total_amount, paid_amount, currency, notes, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW(), NOW())
            RETURNING id, tenant_id, invoice_number, invoice_type, status, cari_id,
                      issue_date, due_date, subtotal, tax_amount, discount_amount,
                      total_amount, paid_amount, currency, notes, created_at, updated_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(&invoice_number)
        .bind(&invoice_type_str)
        .bind(&status_str)
        .bind(create.cari_id)
        .bind(create.issue_date)
        .bind(create.due_date)
        .bind(subtotal)
        .bind(tax_amount)
        .bind(discount_amount)
        .bind(total_amount)
        .bind(Decimal::ZERO)
        .bind(&create.currency)
        .bind(&create.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Invoice"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Invoice>, ApiError> {
        let result: Option<InvoiceRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_number, invoice_type, status, cari_id,
                   issue_date, due_date, subtotal, tax_amount, discount_amount,
                   total_amount, paid_amount, currency, notes, created_at, updated_at,
                   deleted_at, deleted_by
            FROM invoices
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find invoice by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError> {
        let rows: Vec<InvoiceRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_number, invoice_type, status, cari_id,
                   issue_date, due_date, subtotal, tax_amount, discount_amount,
                   total_amount, paid_amount, currency, notes, created_at, updated_at,
                   deleted_at, deleted_by
            FROM invoices
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find invoices by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_number(
        &self,
        tenant_id: i64,
        number: &str,
    ) -> Result<Option<Invoice>, ApiError> {
        let result: Option<InvoiceRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_number, invoice_type, status, cari_id,
                   issue_date, due_date, subtotal, tax_amount, discount_amount,
                   total_amount, paid_amount, currency, notes, created_at, updated_at,
                   deleted_at, deleted_by
            FROM invoices
            WHERE tenant_id = $1 AND invoice_number = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(number)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find invoice by number: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<Invoice>, ApiError> {
        let rows: Vec<InvoiceRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_number, invoice_type, status, cari_id,
                   issue_date, due_date, subtotal, tax_amount, discount_amount,
                   total_amount, paid_amount, currency, notes, created_at, updated_at,
                   deleted_at, deleted_by
            FROM invoices
            WHERE cari_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(cari_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find invoices by cari: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: InvoiceStatus,
    ) -> Result<Vec<Invoice>, ApiError> {
        let status_str = status.to_string();

        let rows: Vec<InvoiceRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_number, invoice_type, status, cari_id,
                   issue_date, due_date, subtotal, tax_amount, discount_amount,
                   total_amount, paid_amount, currency, notes, created_at, updated_at,
                   deleted_at, deleted_by
            FROM invoices
            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find invoices by status: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Invoice>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;

        let rows: Vec<InvoiceRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_number, invoice_type, status, cari_id,
                   issue_date, due_date, subtotal, tax_amount, discount_amount,
                   total_amount, paid_amount, currency, notes, created_at, updated_at,
                   deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM invoices
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
        .map_err(|e| map_sqlx_error(e, "Invoice"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<Invoice> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(invoice, _)| invoice)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: InvoiceStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Invoice>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;
        let status_str = status.to_string();

        let rows: Vec<InvoiceRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_number, invoice_type, status, cari_id,
                   issue_date, due_date, subtotal, tax_amount, discount_amount,
                   total_amount, paid_amount, currency, notes, created_at, updated_at,
                   deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM invoices
            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL
            ORDER BY id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Invoice"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<Invoice> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(invoice, _)| invoice)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: InvoiceStatus,
    ) -> Result<Invoice, ApiError> {
        let status_str = status.to_string();

        let row: InvoiceRow = sqlx::query_as(
            r#"
            UPDATE invoices
            SET status = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            RETURNING id, tenant_id, invoice_number, invoice_type, status, cari_id,
                      issue_date, due_date, subtotal, tax_amount, discount_amount,
                      total_amount, paid_amount, currency, notes, created_at, updated_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Invoice"))?;

        Ok(row.into())
    }

    async fn update_paid_amount(
        &self,
        id: i64,
        tenant_id: i64,
        paid_amount: Decimal,
    ) -> Result<Invoice, ApiError> {
        let row: InvoiceRow = sqlx::query_as(
            r#"
            UPDATE invoices
            SET paid_amount = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            RETURNING id, tenant_id, invoice_number, invoice_type, status, cari_id,
                      issue_date, due_date, subtotal, tax_amount, discount_amount,
                      total_amount, paid_amount, currency, notes, created_at, updated_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(paid_amount)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Invoice"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM invoices
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete invoice: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Invoice not found".to_string()));
        }

        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE invoices
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete invoice: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Invoice not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Invoice, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE invoices
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore invoice: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Invoice not found or not deleted".to_string(),
            ));
        }

        // After restore, find_by_id will work because deleted_at is now NULL
        self.find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Invoice not found".to_string()))
    }

    async fn search(&self, tenant_id: i64, query: &str) -> Result<Vec<Invoice>, ApiError> {
        let rows: Vec<InvoiceRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_number, invoice_type, status, cari_id,
                   issue_date, due_date, subtotal, tax_amount, discount_amount,
                   total_amount, paid_amount, currency, notes, created_at, updated_at,
                   deleted_at, deleted_by
            FROM invoices
            WHERE tenant_id = $1 AND deleted_at IS NULL
              AND (
                  unaccent(invoice_number) % unaccent($2)
                  OR unaccent(COALESCE(notes, '')) % unaccent($2)
                  OR search_vector @@ plainto_tsquery('turkish', $2)
              )
            ORDER BY GREATEST(
                similarity(unaccent(invoice_number), unaccent($2)),
                similarity(unaccent(COALESCE(notes, '')), unaccent($2)),
                COALESCE(ts_rank_cd(search_vector, plainto_tsquery('turkish', $2), 32), 0.0)
            ) DESC
            "#,
        )
        .bind(tenant_id)
        .bind(query)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to search invoices: {}", e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn search_paginated(
        &self,
        tenant_id: i64,
        query: &str,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Invoice>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;

        let rows: Vec<InvoiceRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_number, invoice_type, status, cari_id,
                   issue_date, due_date, subtotal, tax_amount, discount_amount,
                   total_amount, paid_amount, currency, notes, created_at, updated_at,
                   deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM invoices
            WHERE tenant_id = $1 AND deleted_at IS NULL
              AND (
                  unaccent(invoice_number) % unaccent($2)
                  OR unaccent(COALESCE(notes, '')) % unaccent($2)
                  OR search_vector @@ plainto_tsquery('turkish', $2)
              )
            ORDER BY GREATEST(
                similarity(unaccent(invoice_number), unaccent($2)),
                similarity(unaccent(COALESCE(notes, '')), unaccent($2)),
                COALESCE(ts_rank_cd(search_vector, plainto_tsquery('turkish', $2), 32), 0.0)
            ) DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(query)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to search invoices: {}", e)))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<Invoice> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(invoice, _)| invoice)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError> {
        let rows: Vec<InvoiceRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_number, invoice_type, status, cari_id,
                   issue_date, due_date, subtotal, tax_amount, discount_amount,
                   total_amount, paid_amount, currency, notes, created_at, updated_at,
                   deleted_at, deleted_by
            FROM invoices
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted invoices: {}", e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM invoices
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy invoice: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Invoice not found".to_string()));
        }

        Ok(())
    }
}

/// PostgreSQL invoice line repository
pub struct PostgresInvoiceLineRepository {
    pool: Arc<PgPool>,
}

impl PostgresInvoiceLineRepository {
    /// Create a new PostgreSQL invoice line repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxInvoiceLineRepository {
        Arc::new(self) as BoxInvoiceLineRepository
    }
}

#[async_trait]
impl InvoiceLineRepository for PostgresInvoiceLineRepository {
    async fn create_many(
        &self,
        invoice_id: i64,
        lines: Vec<CreateInvoiceLine>,
    ) -> Result<Vec<InvoiceLine>, ApiError> {
        let mut result = Vec::with_capacity(lines.len());

        for (i, line) in lines.into_iter().enumerate() {
            let line_subtotal = line.quantity * line.unit_price;
            let line_discount = line_subtotal * (line.discount_rate / Decimal::ONE_HUNDRED);
            let after_discount = line_subtotal - line_discount;
            let line_tax = after_discount * (line.tax_rate / Decimal::ONE_HUNDRED);
            let line_total = after_discount + line_tax;

            let row: InvoiceLineRow = sqlx::query_as(
                r#"
                INSERT INTO invoice_lines (invoice_id, product_id, description, quantity,
                                           unit_price, tax_rate, discount_rate, line_total, sort_order)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING id, invoice_id, product_id, description, quantity,
                          unit_price, tax_rate, discount_rate, line_total, sort_order,
                          deleted_at, deleted_by
                "#,
            )
            .bind(invoice_id)
            .bind(line.product_id)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(line.tax_rate)
            .bind(line.discount_rate)
            .bind(line_total)
            .bind(i as i32)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "InvoiceLine"))?;

            result.push(row.into());
        }

        Ok(result)
    }

    async fn find_by_invoice(&self, invoice_id: i64) -> Result<Vec<InvoiceLine>, ApiError> {
        let rows: Vec<InvoiceLineRow> = sqlx::query_as(
            r#"
            SELECT id, invoice_id, product_id, description, quantity,
                   unit_price, tax_rate, discount_rate, line_total, sort_order,
                   deleted_at, deleted_by
            FROM invoice_lines
            WHERE invoice_id = $1 AND deleted_at IS NULL
            ORDER BY sort_order
            "#,
        )
        .bind(invoice_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find invoice lines by invoice: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete_by_invoice(&self, invoice_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            DELETE FROM invoice_lines
            WHERE invoice_id = $1
            "#,
        )
        .bind(invoice_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete invoice lines: {}", e)))?;

        Ok(())
    }

    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE invoice_lines
            SET deleted_at = NOW(), deleted_by = $2
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete invoice line: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("InvoiceLine not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64) -> Result<InvoiceLine, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE invoice_lines
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore invoice line: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "InvoiceLine not found or not deleted".to_string(),
            ));
        }

        let row: InvoiceLineRow = sqlx::query_as(
            r#"
            SELECT id, invoice_id, product_id, description, quantity,
                   unit_price, tax_rate, discount_rate, line_total, sort_order,
                   deleted_at, deleted_by
            FROM invoice_lines
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InvoiceLine"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self) -> Result<Vec<InvoiceLine>, ApiError> {
        let rows: Vec<InvoiceLineRow> = sqlx::query_as(
            r#"
            SELECT id, invoice_id, product_id, description, quantity,
                   unit_price, tax_rate, discount_rate, line_total, sort_order,
                   deleted_at, deleted_by
            FROM invoice_lines
            WHERE deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted invoice lines: {}", e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM invoice_lines
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy invoice line: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("InvoiceLine not found".to_string()));
        }

        Ok(())
    }
}

/// PostgreSQL payment repository
pub struct PostgresPaymentRepository {
    pool: Arc<PgPool>,
}

impl PostgresPaymentRepository {
    /// Create a new PostgreSQL payment repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxPaymentRepository {
        Arc::new(self) as BoxPaymentRepository
    }
}

#[async_trait]
impl PaymentRepository for PostgresPaymentRepository {
    async fn create(&self, create: CreatePayment) -> Result<Payment, ApiError> {
        let row: PaymentRow = sqlx::query_as(
            r#"
            INSERT INTO payments (tenant_id, invoice_id, amount, payment_date,
                                  payment_method, reference_number, notes, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING id, tenant_id, invoice_id, amount, payment_date,
                      payment_method, reference_number, notes, created_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(create.invoice_id)
        .bind(create.amount)
        .bind(create.payment_date)
        .bind(&create.payment_method)
        .bind(&create.reference_number)
        .bind(&create.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Payment"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Payment>, ApiError> {
        let result: Option<PaymentRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_id, amount, payment_date,
                   payment_method, reference_number, notes, created_at,
                   deleted_at, deleted_by
            FROM payments
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find payment by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_invoice(&self, invoice_id: i64) -> Result<Vec<Payment>, ApiError> {
        let rows: Vec<PaymentRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_id, amount, payment_date,
                   payment_method, reference_number, notes, created_at,
                   deleted_at, deleted_by
            FROM payments
            WHERE invoice_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(invoice_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find payments by invoice: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM payments
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete payment: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Payment not found".to_string()));
        }

        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE payments
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete payment: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Payment not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Payment, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE payments
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore payment: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Payment not found or not deleted".to_string(),
            ));
        }

        let row: PaymentRow = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_id, amount, payment_date,
                   payment_method, reference_number, notes, created_at,
                   deleted_at, deleted_by
            FROM payments
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Payment"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Payment>, ApiError> {
        let rows: Vec<PaymentRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, invoice_id, amount, payment_date,
                   payment_method, reference_number, notes, created_at,
                   deleted_at, deleted_by
            FROM payments
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted payments: {}", e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM payments
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy payment: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Payment not found".to_string()));
        }

        Ok(())
    }
}
