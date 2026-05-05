//! PostgreSQL sales repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::sales::model::{
    CreateQuotation, CreateQuotationLine, CreateSalesOrder, CreateSalesOrderLine, Quotation,
    QuotationLine, QuotationStatus, SalesOrder, SalesOrderLine, SalesOrderStatus,
};
use crate::domain::sales::repository::{
    BoxQuotationLineRepository, BoxQuotationRepository, BoxSalesOrderLineRepository,
    BoxSalesOrderRepository, QuotationLineRepository, QuotationRepository,
    SalesOrderLineRepository, SalesOrderRepository,
};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

// ---------------------------------------------------------------------------
// Row structs
// ---------------------------------------------------------------------------

/// Database row representation for SalesOrder
#[derive(Debug, FromRow)]
struct SalesOrderRow {
    id: i64,
    tenant_id: i64,
    order_number: String,
    cari_id: i64,
    status: String,
    order_date: DateTime<Utc>,
    delivery_date: Option<DateTime<Utc>>,
    subtotal: Decimal,
    tax_amount: Decimal,
    discount_amount: Decimal,
    total_amount: Decimal,
    notes: Option<String>,
    shipping_address: Option<String>,
    billing_address: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
}

impl From<SalesOrderRow> for SalesOrder {
    fn from(row: SalesOrderRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid sales order status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            SalesOrderStatus::Draft
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            order_number: row.order_number,
            cari_id: row.cari_id,
            status,
            order_date: row.order_date,
            delivery_date: row.delivery_date,
            subtotal: row.subtotal,
            tax_amount: row.tax_amount,
            discount_amount: row.discount_amount,
            total_amount: row.total_amount,
            notes: row.notes,
            shipping_address: row.shipping_address,
            billing_address: row.billing_address,
            created_at: row.created_at,
            updated_at: row.updated_at.unwrap_or(row.created_at),
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// Database row representation for SalesOrder with total count (for pagination)
#[derive(Debug, FromRow)]
struct SalesOrderRowWithTotal {
    id: i64,
    tenant_id: i64,
    order_number: String,
    cari_id: i64,
    status: String,
    order_date: DateTime<Utc>,
    delivery_date: Option<DateTime<Utc>>,
    subtotal: Decimal,
    tax_amount: Decimal,
    discount_amount: Decimal,
    total_amount: Decimal,
    notes: Option<String>,
    shipping_address: Option<String>,
    billing_address: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
    total_count: i64,
}

impl From<SalesOrderRowWithTotal> for (SalesOrder, i64) {
    fn from(row: SalesOrderRowWithTotal) -> (SalesOrder, i64) {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid sales order status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            SalesOrderStatus::Draft
        });

        let order = SalesOrder {
            id: row.id,
            tenant_id: row.tenant_id,
            order_number: row.order_number,
            cari_id: row.cari_id,
            status,
            order_date: row.order_date,
            delivery_date: row.delivery_date,
            subtotal: row.subtotal,
            tax_amount: row.tax_amount,
            discount_amount: row.discount_amount,
            total_amount: row.total_amount,
            notes: row.notes,
            shipping_address: row.shipping_address,
            billing_address: row.billing_address,
            created_at: row.created_at,
            updated_at: row.updated_at.unwrap_or(row.created_at),
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        };
        (order, row.total_count)
    }
}

/// Database row representation for SalesOrderLine
#[derive(Debug, FromRow)]
struct SalesOrderLineRow {
    id: i64,
    order_id: i64,
    product_id: Option<i64>,
    description: String,
    quantity: Decimal,
    unit_price: Decimal,
    tax_rate: Decimal,
    discount_rate: Decimal,
    line_total: Decimal,
    sort_order: i32,
}

impl From<SalesOrderLineRow> for SalesOrderLine {
    fn from(row: SalesOrderLineRow) -> Self {
        Self {
            id: row.id,
            order_id: row.order_id,
            product_id: row.product_id,
            description: row.description,
            quantity: row.quantity,
            unit_price: row.unit_price,
            tax_rate: row.tax_rate,
            discount_rate: row.discount_rate,
            line_total: row.line_total,
            sort_order: row.sort_order,
        }
    }
}

/// Database row representation for Quotation
#[derive(Debug, FromRow)]
struct QuotationRow {
    id: i64,
    tenant_id: i64,
    quotation_number: String,
    cari_id: i64,
    status: String,
    valid_until: DateTime<Utc>,
    subtotal: Decimal,
    tax_amount: Decimal,
    discount_amount: Decimal,
    total_amount: Decimal,
    notes: Option<String>,
    terms: Option<String>,
    sales_order_id: Option<i64>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
}

impl From<QuotationRow> for Quotation {
    fn from(row: QuotationRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid quotation status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            QuotationStatus::Draft
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            quotation_number: row.quotation_number,
            cari_id: row.cari_id,
            status,
            valid_until: row.valid_until,
            subtotal: row.subtotal,
            tax_amount: row.tax_amount,
            discount_amount: row.discount_amount,
            total_amount: row.total_amount,
            notes: row.notes,
            terms: row.terms,
            sales_order_id: row.sales_order_id,
            created_at: row.created_at,
            updated_at: row.updated_at.unwrap_or(row.created_at),
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// Database row representation for Quotation with total count (for pagination)
#[derive(Debug, FromRow)]
struct QuotationRowWithTotal {
    id: i64,
    tenant_id: i64,
    quotation_number: String,
    cari_id: i64,
    status: String,
    valid_until: DateTime<Utc>,
    subtotal: Decimal,
    tax_amount: Decimal,
    discount_amount: Decimal,
    total_amount: Decimal,
    notes: Option<String>,
    terms: Option<String>,
    sales_order_id: Option<i64>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
    total_count: i64,
}

impl From<QuotationRowWithTotal> for (Quotation, i64) {
    fn from(row: QuotationRowWithTotal) -> (Quotation, i64) {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid quotation status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            QuotationStatus::Draft
        });

        let quotation = Quotation {
            id: row.id,
            tenant_id: row.tenant_id,
            quotation_number: row.quotation_number,
            cari_id: row.cari_id,
            status,
            valid_until: row.valid_until,
            subtotal: row.subtotal,
            tax_amount: row.tax_amount,
            discount_amount: row.discount_amount,
            total_amount: row.total_amount,
            notes: row.notes,
            terms: row.terms,
            sales_order_id: row.sales_order_id,
            created_at: row.created_at,
            updated_at: row.updated_at.unwrap_or(row.created_at),
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        };
        (quotation, row.total_count)
    }
}

/// Database row representation for QuotationLine
#[derive(Debug, FromRow)]
struct QuotationLineRow {
    id: i64,
    quotation_id: i64,
    product_id: Option<i64>,
    description: String,
    quantity: Decimal,
    unit_price: Decimal,
    tax_rate: Decimal,
    discount_rate: Decimal,
    line_total: Decimal,
    sort_order: i32,
}

impl From<QuotationLineRow> for QuotationLine {
    fn from(row: QuotationLineRow) -> Self {
        Self {
            id: row.id,
            quotation_id: row.quotation_id,
            product_id: row.product_id,
            description: row.description,
            quantity: row.quantity,
            unit_price: row.unit_price,
            tax_rate: row.tax_rate,
            discount_rate: row.discount_rate,
            line_total: row.line_total,
            sort_order: row.sort_order,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: calculate totals from line items
// ---------------------------------------------------------------------------

fn calculate_totals(lines: &[CreateSalesOrderLine]) -> (Decimal, Decimal, Decimal, Decimal) {
    let mut subtotal = Decimal::ZERO;
    let mut tax_amount = Decimal::ZERO;
    let mut discount_amount = Decimal::ZERO;

    for line in lines {
        let line_subtotal = line.quantity * line.unit_price;
        let line_discount = line_subtotal * (line.discount_rate / Decimal::ONE_HUNDRED);
        let after_discount = line_subtotal - line_discount;
        let line_tax = after_discount * (line.tax_rate / Decimal::ONE_HUNDRED);

        subtotal += line_subtotal;
        discount_amount += line_discount;
        tax_amount += line_tax;
    }

    let total_amount = subtotal - discount_amount + tax_amount;
    (subtotal, tax_amount, discount_amount, total_amount)
}

fn calculate_quotation_totals(
    lines: &[CreateQuotationLine],
) -> (Decimal, Decimal, Decimal, Decimal) {
    let mut subtotal = Decimal::ZERO;
    let mut tax_amount = Decimal::ZERO;
    let mut discount_amount = Decimal::ZERO;

    for line in lines {
        let line_subtotal = line.quantity * line.unit_price;
        let line_discount = line_subtotal * (line.discount_rate / Decimal::ONE_HUNDRED);
        let after_discount = line_subtotal - line_discount;
        let line_tax = after_discount * (line.tax_rate / Decimal::ONE_HUNDRED);

        subtotal += line_subtotal;
        discount_amount += line_discount;
        tax_amount += line_tax;
    }

    let total_amount = subtotal - discount_amount + tax_amount;
    (subtotal, tax_amount, discount_amount, total_amount)
}

// ---------------------------------------------------------------------------
// PostgreSQL SalesOrderRepository
// ---------------------------------------------------------------------------

/// PostgreSQL sales order repository
pub struct PostgresSalesOrderRepository {
    pool: Arc<PgPool>,
}

impl PostgresSalesOrderRepository {
    /// Create a new PostgreSQL sales order repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxSalesOrderRepository {
        Arc::new(self) as BoxSalesOrderRepository
    }
}

#[async_trait]
impl SalesOrderRepository for PostgresSalesOrderRepository {
    async fn create(&self, create: CreateSalesOrder) -> Result<SalesOrder, ApiError> {
        let status_str = SalesOrderStatus::Draft.to_string();
        let order_number = format!("SO-{}", chrono::Utc::now().timestamp_millis());

        let (subtotal, tax_amount, discount_amount, total_amount) = calculate_totals(&create.lines);

        let row: SalesOrderRow = sqlx::query_as(
            r#"
            INSERT INTO sales_orders (tenant_id, order_number, cari_id, status, order_date,
                                      delivery_date, subtotal, tax_amount, discount_amount,
                                      total_amount, notes, shipping_address, billing_address, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())
            RETURNING id, tenant_id, order_number, cari_id, status, order_date,
                      delivery_date, subtotal, tax_amount, discount_amount, total_amount,
                      notes, shipping_address, billing_address, created_at, updated_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(&order_number)
        .bind(create.cari_id)
        .bind(&status_str)
        .bind(create.order_date)
        .bind(create.delivery_date)
        .bind(subtotal)
        .bind(tax_amount)
        .bind(discount_amount)
        .bind(total_amount)
        .bind(&create.notes)
        .bind(&create.shipping_address)
        .bind(&create.billing_address)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SalesOrder"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<SalesOrder>, ApiError> {
        let result: Option<SalesOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   delivery_date, subtotal, tax_amount, discount_amount, total_amount,
                   notes, shipping_address, billing_address, created_at, updated_at,
                   deleted_at, deleted_by
            FROM sales_orders
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find sales order by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<SalesOrder>, ApiError> {
        let rows: Vec<SalesOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   delivery_date, subtotal, tax_amount, discount_amount, total_amount,
                   notes, shipping_address, billing_address, created_at, updated_at,
                   deleted_at, deleted_by
            FROM sales_orders
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find sales orders by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<SalesOrder>, ApiError> {
        let rows: Vec<SalesOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   delivery_date, subtotal, tax_amount, discount_amount, total_amount,
                   notes, shipping_address, billing_address, created_at, updated_at,
                   deleted_at, deleted_by
            FROM sales_orders
            WHERE cari_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(cari_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find sales orders by cari: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: SalesOrderStatus,
    ) -> Result<Vec<SalesOrder>, ApiError> {
        let status_str = status.to_string();

        let rows: Vec<SalesOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   delivery_date, subtotal, tax_amount, discount_amount, total_amount,
                   notes, shipping_address, billing_address, created_at, updated_at,
                   deleted_at, deleted_by
            FROM sales_orders
            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find sales orders by status: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<SalesOrder>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;

        let rows: Vec<SalesOrderRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   delivery_date, subtotal, tax_amount, discount_amount, total_amount,
                   notes, shipping_address, billing_address, created_at, updated_at,
                   deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM sales_orders
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
        .map_err(|e| map_sqlx_error(e, "SalesOrder"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<SalesOrder> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(order, _)| order)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: SalesOrderStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<SalesOrder>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;
        let status_str = status.to_string();

        let rows: Vec<SalesOrderRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   delivery_date, subtotal, tax_amount, discount_amount, total_amount,
                   notes, shipping_address, billing_address, created_at, updated_at,
                   deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM sales_orders
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
        .map_err(|e| map_sqlx_error(e, "SalesOrder"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<SalesOrder> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(order, _)| order)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update_status(
        &self,
        id: i64,
        status: SalesOrderStatus,
    ) -> Result<SalesOrder, ApiError> {
        let status_str = status.to_string();

        let row: SalesOrderRow = sqlx::query_as(
            r#"
            UPDATE sales_orders
            SET status = $1, updated_at = NOW()
            WHERE id = $2 AND deleted_at IS NULL
            RETURNING id, tenant_id, order_number, cari_id, status, order_date,
                      delivery_date, subtotal, tax_amount, discount_amount, total_amount,
                      notes, shipping_address, billing_address, created_at, updated_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SalesOrder"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE sales_orders
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete sales order: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Sales order not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<SalesOrder, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE sales_orders
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore sales order: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Sales order not found or not deleted".to_string(),
            ));
        }

        self.find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Sales order not found".to_string()))
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<SalesOrder>, ApiError> {
        let rows: Vec<SalesOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   delivery_date, subtotal, tax_amount, discount_amount, total_amount,
                   notes, shipping_address, billing_address, created_at, updated_at,
                   deleted_at, deleted_by
            FROM sales_orders
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted sales orders: {}", e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM sales_orders
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy sales order: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Sales order not found".to_string()));
        }

        Ok(())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM sales_orders
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete sales order: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Sales order not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PostgreSQL SalesOrderLineRepository
// ---------------------------------------------------------------------------

/// PostgreSQL sales order line repository
pub struct PostgresSalesOrderLineRepository {
    pool: Arc<PgPool>,
}

impl PostgresSalesOrderLineRepository {
    /// Create a new PostgreSQL sales order line repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxSalesOrderLineRepository {
        Arc::new(self) as BoxSalesOrderLineRepository
    }
}

#[async_trait]
impl SalesOrderLineRepository for PostgresSalesOrderLineRepository {
    async fn create_many(
        &self,
        order_id: i64,
        lines: Vec<CreateSalesOrderLine>,
    ) -> Result<Vec<SalesOrderLine>, ApiError> {
        let mut result_lines = Vec::with_capacity(lines.len());

        for (i, create) in lines.into_iter().enumerate() {
            let line_total = create.calculate_line_total();

            let row: SalesOrderLineRow = sqlx::query_as(
                r#"
                INSERT INTO sales_order_lines (order_id, product_id, description, quantity,
                                               unit_price, tax_rate, discount_rate, line_total, sort_order)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING id, order_id, product_id, description, quantity,
                          unit_price, tax_rate, discount_rate, line_total, sort_order
                "#,
            )
            .bind(order_id)
            .bind(create.product_id)
            .bind(&create.description)
            .bind(create.quantity)
            .bind(create.unit_price)
            .bind(create.tax_rate)
            .bind(create.discount_rate)
            .bind(line_total)
            .bind(i as i32)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "SalesOrderLine"))?;

            result_lines.push(row.into());
        }

        Ok(result_lines)
    }

    async fn find_by_order(&self, order_id: i64) -> Result<Vec<SalesOrderLine>, ApiError> {
        let rows: Vec<SalesOrderLineRow> = sqlx::query_as(
            r#"
            SELECT id, order_id, product_id, description, quantity,
                   unit_price, tax_rate, discount_rate, line_total, sort_order
            FROM sales_order_lines
            WHERE order_id = $1
            ORDER BY sort_order
            "#,
        )
        .bind(order_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find sales order lines by order: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete_by_order(&self, order_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            DELETE FROM sales_order_lines
            WHERE order_id = $1
            "#,
        )
        .bind(order_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to delete sales order lines by order: {}",
                e
            ))
        })?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PostgreSQL QuotationRepository
// ---------------------------------------------------------------------------

/// PostgreSQL quotation repository
pub struct PostgresQuotationRepository {
    pool: Arc<PgPool>,
}

impl PostgresQuotationRepository {
    /// Create a new PostgreSQL quotation repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxQuotationRepository {
        Arc::new(self) as BoxQuotationRepository
    }
}

#[async_trait]
impl QuotationRepository for PostgresQuotationRepository {
    async fn create(&self, create: CreateQuotation) -> Result<Quotation, ApiError> {
        let status_str = QuotationStatus::Draft.to_string();
        let quotation_number = format!("QUO-{}", chrono::Utc::now().timestamp_millis());

        let (subtotal, tax_amount, discount_amount, total_amount) =
            calculate_quotation_totals(&create.lines);

        let row: QuotationRow = sqlx::query_as(
            r#"
            INSERT INTO quotations (tenant_id, quotation_number, cari_id, status, valid_until,
                                    subtotal, tax_amount, discount_amount, total_amount,
                                    notes, terms, sales_order_id, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NULL, NOW())
            RETURNING id, tenant_id, quotation_number, cari_id, status, valid_until,
                      subtotal, tax_amount, discount_amount, total_amount,
                      notes, terms, sales_order_id, created_at, updated_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(&quotation_number)
        .bind(create.cari_id)
        .bind(&status_str)
        .bind(create.valid_until)
        .bind(subtotal)
        .bind(tax_amount)
        .bind(discount_amount)
        .bind(total_amount)
        .bind(&create.notes)
        .bind(&create.terms)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Quotation"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Quotation>, ApiError> {
        let result: Option<QuotationRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, quotation_number, cari_id, status, valid_until,
                   subtotal, tax_amount, discount_amount, total_amount,
                   notes, terms, sales_order_id, created_at, updated_at,
                   deleted_at, deleted_by
            FROM quotations
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find quotation by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Quotation>, ApiError> {
        let rows: Vec<QuotationRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, quotation_number, cari_id, status, valid_until,
                   subtotal, tax_amount, discount_amount, total_amount,
                   notes, terms, sales_order_id, created_at, updated_at,
                   deleted_at, deleted_by
            FROM quotations
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find quotations by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<Quotation>, ApiError> {
        let rows: Vec<QuotationRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, quotation_number, cari_id, status, valid_until,
                   subtotal, tax_amount, discount_amount, total_amount,
                   notes, terms, sales_order_id, created_at, updated_at,
                   deleted_at, deleted_by
            FROM quotations
            WHERE cari_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(cari_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find quotations by cari: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: QuotationStatus,
    ) -> Result<Vec<Quotation>, ApiError> {
        let status_str = status.to_string();

        let rows: Vec<QuotationRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, quotation_number, cari_id, status, valid_until,
                   subtotal, tax_amount, discount_amount, total_amount,
                   notes, terms, sales_order_id, created_at, updated_at,
                   deleted_at, deleted_by
            FROM quotations
            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find quotations by status: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Quotation>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;

        let rows: Vec<QuotationRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, quotation_number, cari_id, status, valid_until,
                   subtotal, tax_amount, discount_amount, total_amount,
                   notes, terms, sales_order_id, created_at, updated_at,
                   deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM quotations
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
        .map_err(|e| map_sqlx_error(e, "Quotation"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<Quotation> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(quotation, _)| quotation)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: QuotationStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Quotation>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;
        let status_str = status.to_string();

        let rows: Vec<QuotationRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, quotation_number, cari_id, status, valid_until,
                   subtotal, tax_amount, discount_amount, total_amount,
                   notes, terms, sales_order_id, created_at, updated_at,
                   deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM quotations
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
        .map_err(|e| map_sqlx_error(e, "Quotation"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<Quotation> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(quotation, _)| quotation)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update_status(&self, id: i64, status: QuotationStatus) -> Result<Quotation, ApiError> {
        let status_str = status.to_string();

        let row: QuotationRow = sqlx::query_as(
            r#"
            UPDATE quotations
            SET status = $1, updated_at = NOW()
            WHERE id = $2 AND deleted_at IS NULL
            RETURNING id, tenant_id, quotation_number, cari_id, status, valid_until,
                      subtotal, tax_amount, discount_amount, total_amount,
                      notes, terms, sales_order_id, created_at, updated_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Quotation"))?;

        Ok(row.into())
    }

    async fn link_to_order(&self, id: i64, order_id: i64) -> Result<Quotation, ApiError> {
        let status_str = QuotationStatus::ConvertedToOrder.to_string();

        let row: QuotationRow = sqlx::query_as(
            r#"
            UPDATE quotations
            SET sales_order_id = $1, status = $2, updated_at = NOW()
            WHERE id = $3 AND deleted_at IS NULL
            RETURNING id, tenant_id, quotation_number, cari_id, status, valid_until,
                      subtotal, tax_amount, discount_amount, total_amount,
                      notes, terms, sales_order_id, created_at, updated_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(order_id)
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Quotation"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE quotations
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete quotation: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Quotation not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Quotation, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE quotations
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore quotation: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Quotation not found or not deleted".to_string(),
            ));
        }

        self.find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Quotation not found".to_string()))
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Quotation>, ApiError> {
        let rows: Vec<QuotationRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, quotation_number, cari_id, status, valid_until,
                   subtotal, tax_amount, discount_amount, total_amount,
                   notes, terms, sales_order_id, created_at, updated_at,
                   deleted_at, deleted_by
            FROM quotations
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted quotations: {}", e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM quotations
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy quotation: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Quotation not found".to_string()));
        }

        Ok(())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM quotations
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete quotation: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Quotation not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PostgreSQL QuotationLineRepository
// ---------------------------------------------------------------------------

/// PostgreSQL quotation line repository
pub struct PostgresQuotationLineRepository {
    pool: Arc<PgPool>,
}

impl PostgresQuotationLineRepository {
    /// Create a new PostgreSQL quotation line repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxQuotationLineRepository {
        Arc::new(self) as BoxQuotationLineRepository
    }
}

#[async_trait]
impl QuotationLineRepository for PostgresQuotationLineRepository {
    async fn create_many(
        &self,
        quotation_id: i64,
        lines: Vec<CreateQuotationLine>,
    ) -> Result<Vec<QuotationLine>, ApiError> {
        let mut result_lines = Vec::with_capacity(lines.len());

        for (i, create) in lines.into_iter().enumerate() {
            let line_subtotal = create.quantity * create.unit_price;
            let line_discount = line_subtotal * (create.discount_rate / Decimal::ONE_HUNDRED);
            let after_discount = line_subtotal - line_discount;
            let line_tax = after_discount * (create.tax_rate / Decimal::ONE_HUNDRED);
            let line_total = after_discount + line_tax;

            let row: QuotationLineRow = sqlx::query_as(
                r#"
                INSERT INTO quotation_lines (quotation_id, product_id, description, quantity,
                                             unit_price, tax_rate, discount_rate, line_total, sort_order)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING id, quotation_id, product_id, description, quantity,
                          unit_price, tax_rate, discount_rate, line_total, sort_order
                "#,
            )
            .bind(quotation_id)
            .bind(create.product_id)
            .bind(&create.description)
            .bind(create.quantity)
            .bind(create.unit_price)
            .bind(create.tax_rate)
            .bind(create.discount_rate)
            .bind(line_total)
            .bind(i as i32)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "QuotationLine"))?;

            result_lines.push(row.into());
        }

        Ok(result_lines)
    }

    async fn find_by_quotation(&self, quotation_id: i64) -> Result<Vec<QuotationLine>, ApiError> {
        let rows: Vec<QuotationLineRow> = sqlx::query_as(
            r#"
            SELECT id, quotation_id, product_id, description, quantity,
                   unit_price, tax_rate, discount_rate, line_total, sort_order
            FROM quotation_lines
            WHERE quotation_id = $1
            ORDER BY sort_order
            "#,
        )
        .bind(quotation_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find quotation lines by quotation: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete_by_quotation(&self, quotation_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            DELETE FROM quotation_lines
            WHERE quotation_id = $1
            "#,
        )
        .bind(quotation_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to delete quotation lines by quotation: {}",
                e
            ))
        })?;

        Ok(())
    }
}
