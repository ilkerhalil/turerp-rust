//! PostgreSQL inter-company repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::inter_company::model::{
    CreateInterCompanyInvoice, CreateInterCompanyStockTransfer, InterCompanyInvoice,
    InterCompanyInvoiceLine, InterCompanyStockTransfer,
};
use crate::domain::inter_company::repository::{BoxInterCompanyRepository, InterCompanyRepository};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// InvoiceRow / Invoice conversion
// ---------------------------------------------------------------------------

#[derive(Debug, FromRow)]
struct InterCompanyInvoiceRow {
    id: i64,
    tenant_id: i64,
    seller_company_id: i64,
    buyer_company_id: i64,
    sales_invoice_id: i64,
    purchase_invoice_id: i64,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct InterCompanyInvoiceLineRow {
    product_id: i64,
    description: String,
    quantity: Decimal,
    unit_price: Decimal,
    vat_rate: Decimal,
}

// ---------------------------------------------------------------------------
// StockTransferRow / StockTransfer conversion
// ---------------------------------------------------------------------------

#[derive(Debug, FromRow)]
struct InterCompanyStockTransferRow {
    id: i64,
    tenant_id: i64,
    from_company_id: i64,
    to_company_id: i64,
    product_id: i64,
    warehouse_id: i64,
    quantity: Decimal,
    out_movement_id: i64,
    in_movement_id: i64,
    created_by: i64,
    created_at: DateTime<Utc>,
}

// ===========================================================================
// PostgresInterCompanyRepository
// ===========================================================================

/// PostgreSQL inter-company repository.
pub struct PostgresInterCompanyRepository {
    pool: Arc<PgPool>,
}

impl PostgresInterCompanyRepository {
    /// Create a new PostgreSQL inter-company repository.
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object.
    pub fn into_boxed(self) -> BoxInterCompanyRepository {
        Arc::new(self) as BoxInterCompanyRepository
    }

    /// Fetch lines for a given invoice, scoped to the tenant of the parent invoice.
    ///
    /// Joins to `inter_company_invoices` so that line items can only be read for
    /// invoices that belong to the calling tenant. Without the JOIN, a tenant
    /// that knows another tenant's `invoice_id` could read its line items.
    async fn fetch_invoice_lines(
        &self,
        invoice_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<InterCompanyInvoiceLine>, ApiError> {
        let rows: Vec<InterCompanyInvoiceLineRow> = sqlx::query_as(concat!(
            "SELECT l.invoice_id, l.product_id, l.description, l.quantity, l.unit_price, l.vat_rate ",
            "FROM inter_company_invoice_lines l ",
            "INNER JOIN inter_company_invoices i ON i.id = l.invoice_id ",
            "WHERE l.invoice_id = $1 AND i.tenant_id = $2"
        ))
        .bind(invoice_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InterCompanyInvoiceLine"))?;

        Ok(rows
            .into_iter()
            .map(|r| InterCompanyInvoiceLine {
                product_id: r.product_id,
                description: r.description,
                quantity: r.quantity,
                unit_price: r.unit_price,
                vat_rate: r.vat_rate,
            })
            .collect())
    }
}

#[async_trait]
impl InterCompanyRepository for PostgresInterCompanyRepository {
    async fn create_invoice(
        &self,
        invoice: CreateInterCompanyInvoice,
    ) -> Result<InterCompanyInvoice, ApiError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| map_sqlx_error(e, "InterCompanyInvoice"))?;

        let row: InterCompanyInvoiceRow = sqlx::query_as(concat!(
            "INSERT INTO inter_company_invoices ",
            "(tenant_id, seller_company_id, buyer_company_id, sales_invoice_id, purchase_invoice_id) ",
            "VALUES ($1, $2, $3, $4, $5) ",
            "RETURNING id, tenant_id, seller_company_id, buyer_company_id, sales_invoice_id, purchase_invoice_id, created_at"
        ))
        .bind(invoice.tenant_id)
        .bind(invoice.seller_company_id)
        .bind(invoice.buyer_company_id)
        .bind(invoice.sales_invoice_id)
        .bind(invoice.purchase_invoice_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| map_sqlx_error(e, "InterCompanyInvoice"))?;

        for line in &invoice.lines {
            sqlx::query(concat!(
                "INSERT INTO inter_company_invoice_lines ",
                "(invoice_id, product_id, description, quantity, unit_price, vat_rate) ",
                "VALUES ($1, $2, $3, $4, $5, $6)"
            ))
            .bind(row.id)
            .bind(line.product_id)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(line.unit_price)
            .bind(line.vat_rate)
            .execute(&mut *tx)
            .await
            .map_err(|e| map_sqlx_error(e, "InterCompanyInvoiceLine"))?;
        }

        tx.commit()
            .await
            .map_err(|e| map_sqlx_error(e, "InterCompanyInvoice"))?;

        Ok(InterCompanyInvoice {
            id: row.id,
            tenant_id: row.tenant_id,
            seller_company_id: row.seller_company_id,
            buyer_company_id: row.buyer_company_id,
            lines: invoice.lines,
            sales_invoice_id: row.sales_invoice_id,
            purchase_invoice_id: row.purchase_invoice_id,
            created_at: row.created_at,
        })
    }

    async fn get_invoice(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<InterCompanyInvoice>, ApiError> {
        let row: Option<InterCompanyInvoiceRow> = sqlx::query_as(concat!(
            "SELECT id, tenant_id, seller_company_id, buyer_company_id, sales_invoice_id, purchase_invoice_id, created_at ",
            "FROM inter_company_invoices WHERE id = $1 AND tenant_id = $2"
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InterCompanyInvoice"))?;

        match row {
            Some(r) => {
                let lines = self.fetch_invoice_lines(r.id, tenant_id).await?;
                Ok(Some(InterCompanyInvoice {
                    id: r.id,
                    tenant_id: r.tenant_id,
                    seller_company_id: r.seller_company_id,
                    buyer_company_id: r.buyer_company_id,
                    lines,
                    sales_invoice_id: r.sales_invoice_id,
                    purchase_invoice_id: r.purchase_invoice_id,
                    created_at: r.created_at,
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_invoices(&self, tenant_id: i64) -> Result<Vec<InterCompanyInvoice>, ApiError> {
        let rows: Vec<InterCompanyInvoiceRow> = sqlx::query_as(concat!(
            "SELECT id, tenant_id, seller_company_id, buyer_company_id, sales_invoice_id, purchase_invoice_id, created_at ",
            "FROM inter_company_invoices WHERE tenant_id = $1 ORDER BY created_at DESC"
        ))
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InterCompanyInvoice"))?;

        let mut invoices = Vec::with_capacity(rows.len());
        for row in rows {
            // Each row is already filtered to the caller's tenant by the WHERE
            // clause, so row.tenant_id == tenant_id here.
            let lines = self.fetch_invoice_lines(row.id, row.tenant_id).await?;
            invoices.push(InterCompanyInvoice {
                id: row.id,
                tenant_id: row.tenant_id,
                seller_company_id: row.seller_company_id,
                buyer_company_id: row.buyer_company_id,
                lines,
                sales_invoice_id: row.sales_invoice_id,
                purchase_invoice_id: row.purchase_invoice_id,
                created_at: row.created_at,
            });
        }

        Ok(invoices)
    }

    async fn create_stock_transfer(
        &self,
        transfer: CreateInterCompanyStockTransfer,
    ) -> Result<InterCompanyStockTransfer, ApiError> {
        let row: InterCompanyStockTransferRow = sqlx::query_as(concat!(
            "INSERT INTO inter_company_stock_transfers ",
            "(tenant_id, from_company_id, to_company_id, product_id, warehouse_id, quantity, out_movement_id, in_movement_id, created_by) ",
            "VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) ",
            "RETURNING id, tenant_id, from_company_id, to_company_id, product_id, warehouse_id, quantity, out_movement_id, in_movement_id, created_by, created_at"
        ))
        .bind(transfer.tenant_id)
        .bind(transfer.from_company_id)
        .bind(transfer.to_company_id)
        .bind(transfer.product_id)
        .bind(transfer.warehouse_id)
        .bind(transfer.quantity)
        .bind(transfer.out_movement_id)
        .bind(transfer.in_movement_id)
        .bind(transfer.created_by)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InterCompanyStockTransfer"))?;

        Ok(InterCompanyStockTransfer {
            id: row.id,
            tenant_id: row.tenant_id,
            from_company_id: row.from_company_id,
            to_company_id: row.to_company_id,
            product_id: row.product_id,
            warehouse_id: row.warehouse_id,
            quantity: row.quantity,
            out_movement_id: row.out_movement_id,
            in_movement_id: row.in_movement_id,
            created_by: row.created_by,
            created_at: row.created_at,
        })
    }

    async fn get_stock_transfer(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<InterCompanyStockTransfer>, ApiError> {
        let row: Option<InterCompanyStockTransferRow> = sqlx::query_as(concat!(
            "SELECT id, tenant_id, from_company_id, to_company_id, product_id, warehouse_id, quantity, out_movement_id, in_movement_id, created_by, created_at ",
            "FROM inter_company_stock_transfers WHERE id = $1 AND tenant_id = $2"
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InterCompanyStockTransfer"))?;

        match row {
            Some(r) => Ok(Some(InterCompanyStockTransfer {
                id: r.id,
                tenant_id: r.tenant_id,
                from_company_id: r.from_company_id,
                to_company_id: r.to_company_id,
                product_id: r.product_id,
                warehouse_id: r.warehouse_id,
                quantity: r.quantity,
                out_movement_id: r.out_movement_id,
                in_movement_id: r.in_movement_id,
                created_by: r.created_by,
                created_at: r.created_at,
            })),
            None => Ok(None),
        }
    }

    async fn list_stock_transfers(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<InterCompanyStockTransfer>, ApiError> {
        let rows: Vec<InterCompanyStockTransferRow> = sqlx::query_as(concat!(
            "SELECT id, tenant_id, from_company_id, to_company_id, product_id, warehouse_id, quantity, out_movement_id, in_movement_id, created_by, created_at ",
            "FROM inter_company_stock_transfers WHERE tenant_id = $1 ORDER BY created_at DESC"
        ))
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "InterCompanyStockTransfer"))?;

        Ok(rows
            .into_iter()
            .map(|r| InterCompanyStockTransfer {
                id: r.id,
                tenant_id: r.tenant_id,
                from_company_id: r.from_company_id,
                to_company_id: r.to_company_id,
                product_id: r.product_id,
                warehouse_id: r.warehouse_id,
                quantity: r.quantity,
                out_movement_id: r.out_movement_id,
                in_movement_id: r.in_movement_id,
                created_by: r.created_by,
                created_at: r.created_at,
            })
            .collect())
    }
}
