//! PostgreSQL purchase repository implementation

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::purchase::model::{
    CreateGoodsReceipt, CreateGoodsReceiptLine, CreatePurchaseOrder, CreatePurchaseOrderLine,
    CreatePurchaseRequest, CreatePurchaseRequestLine, GoodsReceipt, GoodsReceiptLine,
    GoodsReceiptStatus, PurchaseOrder, PurchaseOrderLine, PurchaseOrderStatus, PurchaseRequest,
    PurchaseRequestLine, PurchaseRequestStatus, UpdatePurchaseRequest,
};
use crate::domain::purchase::repository::{
    BoxGoodsReceiptLineRepository, BoxGoodsReceiptRepository, BoxPurchaseOrderLineRepository,
    BoxPurchaseOrderRepository, BoxPurchaseRequestLineRepository, BoxPurchaseRequestRepository,
    GoodsReceiptLineRepository, GoodsReceiptRepository, PurchaseOrderLineRepository,
    PurchaseOrderRepository, PurchaseRequestLineRepository, PurchaseRequestRepository,
};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

/// Parse a string into PurchaseOrderStatus, defaulting to Draft on failure
fn parse_purchase_order_status(s: &str) -> PurchaseOrderStatus {
    match s {
        "Draft" => PurchaseOrderStatus::Draft,
        "PendingApproval" => PurchaseOrderStatus::PendingApproval,
        "Approved" => PurchaseOrderStatus::Approved,
        "SentToVendor" => PurchaseOrderStatus::SentToVendor,
        "PartialReceived" => PurchaseOrderStatus::PartialReceived,
        "Received" => PurchaseOrderStatus::Received,
        "Cancelled" => PurchaseOrderStatus::Cancelled,
        "OnHold" => PurchaseOrderStatus::OnHold,
        _ => {
            tracing::warn!(
                "Invalid PurchaseOrderStatus '{}' in database, defaulting to Draft",
                s
            );
            PurchaseOrderStatus::Draft
        }
    }
}

/// Parse a string into PurchaseRequestStatus, defaulting to Draft on failure
fn parse_purchase_request_status(s: &str) -> PurchaseRequestStatus {
    match s {
        "Draft" => PurchaseRequestStatus::Draft,
        "PendingApproval" => PurchaseRequestStatus::PendingApproval,
        "Approved" => PurchaseRequestStatus::Approved,
        "Rejected" => PurchaseRequestStatus::Rejected,
        "ConvertedToOrder" => PurchaseRequestStatus::ConvertedToOrder,
        _ => {
            tracing::warn!(
                "Invalid PurchaseRequestStatus '{}' in database, defaulting to Draft",
                s
            );
            PurchaseRequestStatus::Draft
        }
    }
}

/// Parse a string into GoodsReceiptStatus, defaulting to Pending on failure
fn parse_goods_receipt_status(s: &str) -> GoodsReceiptStatus {
    match s {
        "Pending" => GoodsReceiptStatus::Pending,
        "Partial" => GoodsReceiptStatus::Partial,
        "Completed" => GoodsReceiptStatus::Completed,
        "Cancelled" => GoodsReceiptStatus::Cancelled,
        _ => {
            tracing::warn!(
                "Invalid GoodsReceiptStatus '{}' in database, defaulting to Pending",
                s
            );
            GoodsReceiptStatus::Pending
        }
    }
}

// ============================================================================
// PurchaseOrder Row and Repository
// ============================================================================

/// Database row representation for PurchaseOrder
#[derive(Debug, FromRow)]
struct PurchaseOrderRow {
    id: i64,
    tenant_id: i64,
    order_number: String,
    cari_id: i64,
    status: String,
    order_date: chrono::DateTime<chrono::Utc>,
    expected_delivery_date: Option<chrono::DateTime<chrono::Utc>>,
    subtotal: Decimal,
    tax_amount: Decimal,
    discount_amount: Decimal,
    total_amount: Decimal,
    currency: String,
    exchange_rate: Decimal,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<PurchaseOrderRow> for PurchaseOrder {
    fn from(row: PurchaseOrderRow) -> Self {
        let status = parse_purchase_order_status(&row.status);

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            order_number: row.order_number,
            cari_id: row.cari_id,
            status,
            order_date: row.order_date,
            expected_delivery_date: row.expected_delivery_date,
            subtotal: row.subtotal,
            tax_amount: row.tax_amount,
            discount_amount: row.discount_amount,
            total_amount: row.total_amount,
            currency: row.currency,
            exchange_rate: row.exchange_rate,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL purchase order repository
pub struct PostgresPurchaseOrderRepository {
    pool: Arc<PgPool>,
}

impl PostgresPurchaseOrderRepository {
    /// Create a new PostgreSQL purchase order repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxPurchaseOrderRepository {
        Arc::new(self) as BoxPurchaseOrderRepository
    }
}

#[async_trait]
impl PurchaseOrderRepository for PostgresPurchaseOrderRepository {
    async fn create(&self, create: CreatePurchaseOrder) -> Result<PurchaseOrder, ApiError> {
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

        let order_number = format!("PO-{}", chrono::Utc::now().timestamp());
        let status_str = PurchaseOrderStatus::Draft.to_string();

        let row: PurchaseOrderRow = sqlx::query_as(
            r#"
            INSERT INTO purchase_orders (tenant_id, order_number, cari_id, status, order_date,
                                          expected_delivery_date, subtotal, tax_amount, discount_amount,
                                          total_amount, notes, created_at, updated_at, deleted_at, deleted_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW(), NULL, NULL)
            RETURNING id, tenant_id, order_number, cari_id, status, order_date,
                      expected_delivery_date, subtotal, tax_amount, discount_amount,
                      total_amount, notes, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(&order_number)
        .bind(create.cari_id)
        .bind(&status_str)
        .bind(create.order_date)
        .bind(create.expected_delivery_date)
        .bind(subtotal)
        .bind(tax_amount)
        .bind(discount_amount)
        .bind(total_amount)
        .bind(&create.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "PurchaseOrder"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<PurchaseOrder>, ApiError> {
        let result: Option<PurchaseOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   expected_delivery_date, subtotal, tax_amount, discount_amount,
                   total_amount, notes, created_at, updated_at, deleted_at, deleted_by
            FROM purchase_orders
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find purchase order by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<PurchaseOrder>, ApiError> {
        let rows: Vec<PurchaseOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   expected_delivery_date, subtotal, tax_amount, discount_amount,
                   total_amount, notes, created_at, updated_at, deleted_at, deleted_by
            FROM purchase_orders
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find purchase orders by tenant: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<PurchaseOrder>, ApiError> {
        let rows: Vec<PurchaseOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   expected_delivery_date, subtotal, tax_amount, discount_amount,
                   total_amount, notes, created_at, updated_at, deleted_at, deleted_by
            FROM purchase_orders
            WHERE cari_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(cari_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find purchase orders by cari: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseOrderStatus,
    ) -> Result<Vec<PurchaseOrder>, ApiError> {
        let status_str = status.to_string();

        let rows: Vec<PurchaseOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   expected_delivery_date, subtotal, tax_amount, discount_amount,
                   total_amount, notes, created_at, updated_at, deleted_at, deleted_by
            FROM purchase_orders
            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find purchase orders by status: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(
        &self,
        id: i64,
        status: PurchaseOrderStatus,
    ) -> Result<PurchaseOrder, ApiError> {
        let status_str = status.to_string();

        let row: PurchaseOrderRow = sqlx::query_as(
            r#"
            UPDATE purchase_orders
            SET status = $1, updated_at = NOW()
            WHERE id = $2 AND deleted_at IS NULL
            RETURNING id, tenant_id, order_number, cari_id, status, order_date,
                      expected_delivery_date, subtotal, tax_amount, discount_amount,
                      total_amount, notes, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "PurchaseOrder"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE purchase_orders
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete purchase order: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "PurchaseOrder not found or already deleted".to_string(),
            ));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<PurchaseOrder, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE purchase_orders
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore purchase order: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "PurchaseOrder not found or not deleted".to_string(),
            ));
        }

        self.find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("PurchaseOrder not found".to_string()))
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<PurchaseOrder>, ApiError> {
        let rows: Vec<PurchaseOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, order_number, cari_id, status, order_date,
                   expected_delivery_date, subtotal, tax_amount, discount_amount,
                   total_amount, notes, created_at, updated_at, deleted_at, deleted_by
            FROM purchase_orders
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find deleted purchase orders: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM purchase_orders
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to permanently delete purchase order: {}",
                e
            ))
        })?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "PurchaseOrder not found or not soft-deleted".to_string(),
            ));
        }

        Ok(())
    }

    async fn update_line_received_quantity(
        &self,
        line_id: i64,
        received_qty: Decimal,
    ) -> Result<PurchaseOrderLine, ApiError> {
        let row: PurchaseOrderLineRow = sqlx::query_as(
            r#"
            UPDATE purchase_order_lines
            SET received_quantity = $1
            WHERE id = $2
            RETURNING id, order_id, product_id, description, quantity, received_quantity,
                      unit_price, tax_rate, discount_rate, line_total, sort_order
            "#,
        )
        .bind(received_qty)
        .bind(line_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "PurchaseOrderLine"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM purchase_orders
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete purchase order: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("PurchaseOrder not found".to_string()));
        }

        Ok(())
    }
}

// ============================================================================
// PurchaseOrderLine Row and Repository
// ============================================================================

/// Database row representation for PurchaseOrderLine
#[derive(Debug, FromRow)]
struct PurchaseOrderLineRow {
    id: i64,
    order_id: i64,
    product_id: Option<i64>,
    description: String,
    quantity: Decimal,
    received_quantity: Decimal,
    unit_price: Decimal,
    tax_rate: Decimal,
    discount_rate: Decimal,
    line_total: Decimal,
    sort_order: i32,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<PurchaseOrderLineRow> for PurchaseOrderLine {
    fn from(row: PurchaseOrderLineRow) -> Self {
        Self {
            id: row.id,
            order_id: row.order_id,
            product_id: row.product_id,
            description: row.description,
            quantity: row.quantity,
            received_quantity: row.received_quantity,
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

/// PostgreSQL purchase order line repository
pub struct PostgresPurchaseOrderLineRepository {
    pool: Arc<PgPool>,
}

impl PostgresPurchaseOrderLineRepository {
    /// Create a new PostgreSQL purchase order line repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxPurchaseOrderLineRepository {
        Arc::new(self) as BoxPurchaseOrderLineRepository
    }
}

#[async_trait]
impl PurchaseOrderLineRepository for PostgresPurchaseOrderLineRepository {
    async fn create_many(
        &self,
        order_id: i64,
        lines: Vec<CreatePurchaseOrderLine>,
    ) -> Result<Vec<PurchaseOrderLine>, ApiError> {
        let mut result = Vec::with_capacity(lines.len());

        for (i, line) in lines.into_iter().enumerate() {
            // line_total = quantity * unit_price * (1 + tax_rate/100) * (1 - discount_rate/100)
            let line_total = line.calculate_line_total();

            let row: PurchaseOrderLineRow = sqlx::query_as(
                r#"
                INSERT INTO purchase_order_lines (order_id, product_id, description, quantity,
                                                  received_quantity, unit_price, tax_rate, discount_rate,
                                                  line_total, sort_order)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING id, order_id, product_id, description, quantity, received_quantity,
                          unit_price, tax_rate, discount_rate, line_total, sort_order,
                          deleted_at, deleted_by
                "#,
            )
            .bind(order_id)
            .bind(line.product_id)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(Decimal::ZERO)
            .bind(line.unit_price)
            .bind(line.tax_rate)
            .bind(line.discount_rate)
            .bind(line_total)
            .bind(i as i32)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "PurchaseOrderLine"))?;

            result.push(row.into());
        }

        Ok(result)
    }

    async fn find_by_order(&self, order_id: i64) -> Result<Vec<PurchaseOrderLine>, ApiError> {
        let rows: Vec<PurchaseOrderLineRow> = sqlx::query_as(
            r#"
            SELECT id, order_id, product_id, description, quantity, received_quantity,
                   unit_price, tax_rate, discount_rate, line_total, sort_order,
                   deleted_at, deleted_by
            FROM purchase_order_lines
            WHERE order_id = $1 AND deleted_at IS NULL
            ORDER BY sort_order
            "#,
        )
        .bind(order_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find purchase order lines by order: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseOrderLine>, ApiError> {
        let result: Option<PurchaseOrderLineRow> = sqlx::query_as(
            r#"
            SELECT id, order_id, product_id, description, quantity, received_quantity,
                   unit_price, tax_rate, discount_rate, line_total, sort_order,
                   deleted_at, deleted_by
            FROM purchase_order_lines
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find purchase order line by id: {}", e))
        })?;

        Ok(result.map(|r| r.into()))
    }

    async fn delete_by_order(&self, order_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            DELETE FROM purchase_order_lines
            WHERE order_id = $1
            "#,
        )
        .bind(order_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete purchase order lines: {}", e)))?;

        Ok(())
    }

    async fn soft_delete_by_order(&self, order_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            UPDATE purchase_order_lines
            SET deleted_at = NOW(), deleted_by = $2
            WHERE order_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(order_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to soft delete purchase order lines: {}", e))
        })?;

        Ok(())
    }

    async fn restore_by_order(&self, order_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            UPDATE purchase_order_lines
            SET deleted_at = NULL, deleted_by = NULL
            WHERE order_id = $1 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(order_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to restore purchase order lines: {}", e))
        })?;

        Ok(())
    }
}

// ============================================================================
// GoodsReceipt Row and Repository
// ============================================================================

/// Database row representation for GoodsReceipt
#[derive(Debug, FromRow)]
struct GoodsReceiptRow {
    id: i64,
    tenant_id: i64,
    receipt_number: String,
    purchase_order_id: i64,
    status: String,
    receipt_date: chrono::DateTime<chrono::Utc>,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<GoodsReceiptRow> for GoodsReceipt {
    fn from(row: GoodsReceiptRow) -> Self {
        let status = parse_goods_receipt_status(&row.status);

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            receipt_number: row.receipt_number,
            purchase_order_id: row.purchase_order_id,
            status,
            receipt_date: row.receipt_date,
            notes: row.notes,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL goods receipt repository
pub struct PostgresGoodsReceiptRepository {
    pool: Arc<PgPool>,
}

impl PostgresGoodsReceiptRepository {
    /// Create a new PostgreSQL goods receipt repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxGoodsReceiptRepository {
        Arc::new(self) as BoxGoodsReceiptRepository
    }
}

#[async_trait]
impl GoodsReceiptRepository for PostgresGoodsReceiptRepository {
    async fn create(&self, create: CreateGoodsReceipt) -> Result<GoodsReceipt, ApiError> {
        let receipt_number = format!("GR-{}", chrono::Utc::now().timestamp());
        let status_str = "Pending";

        let row: GoodsReceiptRow = sqlx::query_as(
            r#"
            INSERT INTO goods_receipts (tenant_id, receipt_number, purchase_order_id, status,
                                          receipt_date, notes, created_at, deleted_at, deleted_by)
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NULL, NULL)
            RETURNING id, tenant_id, receipt_number, purchase_order_id, status,
                      receipt_date, notes, created_at, deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(&receipt_number)
        .bind(create.purchase_order_id)
        .bind(status_str)
        .bind(create.receipt_date)
        .bind(&create.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "GoodsReceipt"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<GoodsReceipt>, ApiError> {
        let result: Option<GoodsReceiptRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, receipt_number, purchase_order_id, status,
                   receipt_date, notes, created_at, deleted_at, deleted_by
            FROM goods_receipts
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find goods receipt by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_order(&self, order_id: i64) -> Result<Vec<GoodsReceipt>, ApiError> {
        let rows: Vec<GoodsReceiptRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, receipt_number, purchase_order_id, status,
                   receipt_date, notes, created_at, deleted_at, deleted_by
            FROM goods_receipts
            WHERE purchase_order_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(order_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find goods receipts by order: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(
        &self,
        id: i64,
        status: GoodsReceiptStatus,
    ) -> Result<GoodsReceipt, ApiError> {
        let status_str = match status {
            GoodsReceiptStatus::Pending => "Pending",
            GoodsReceiptStatus::Partial => "Partial",
            GoodsReceiptStatus::Completed => "Completed",
            GoodsReceiptStatus::Cancelled => "Cancelled",
        };

        let row: GoodsReceiptRow = sqlx::query_as(
            r#"
            UPDATE goods_receipts
            SET status = $1
            WHERE id = $2 AND deleted_at IS NULL
            RETURNING id, tenant_id, receipt_number, purchase_order_id, status,
                      receipt_date, notes, created_at, deleted_at, deleted_by
            "#,
        )
        .bind(status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "GoodsReceipt"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE goods_receipts
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete goods receipt: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "GoodsReceipt not found or already deleted".to_string(),
            ));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<GoodsReceipt, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE goods_receipts
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore goods receipt: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "GoodsReceipt not found or not deleted".to_string(),
            ));
        }

        self.find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("GoodsReceipt not found".to_string()))
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<GoodsReceipt>, ApiError> {
        let rows: Vec<GoodsReceiptRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, receipt_number, purchase_order_id, status,
                   receipt_date, notes, created_at, deleted_at, deleted_by
            FROM goods_receipts
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted goods receipts: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM goods_receipts
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to permanently delete goods receipt: {}", e))
        })?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "GoodsReceipt not found or not soft-deleted".to_string(),
            ));
        }

        Ok(())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM goods_receipts
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete goods receipt: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("GoodsReceipt not found".to_string()));
        }

        Ok(())
    }
}

// ============================================================================
// GoodsReceiptLine Row and Repository
// ============================================================================

/// Database row representation for GoodsReceiptLine
#[derive(Debug, FromRow)]
struct GoodsReceiptLineRow {
    id: i64,
    receipt_id: i64,
    order_line_id: i64,
    product_id: Option<i64>,
    quantity: Decimal,
    condition: String,
    notes: Option<String>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<GoodsReceiptLineRow> for GoodsReceiptLine {
    fn from(row: GoodsReceiptLineRow) -> Self {
        Self {
            id: row.id,
            receipt_id: row.receipt_id,
            order_line_id: row.order_line_id,
            product_id: row.product_id,
            quantity: row.quantity,
            condition: row.condition,
            notes: row.notes,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL goods receipt line repository
pub struct PostgresGoodsReceiptLineRepository {
    pool: Arc<PgPool>,
}

impl PostgresGoodsReceiptLineRepository {
    /// Create a new PostgreSQL goods receipt line repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxGoodsReceiptLineRepository {
        Arc::new(self) as BoxGoodsReceiptLineRepository
    }
}

#[async_trait]
impl GoodsReceiptLineRepository for PostgresGoodsReceiptLineRepository {
    async fn create_many(
        &self,
        receipt_id: i64,
        lines: Vec<CreateGoodsReceiptLine>,
    ) -> Result<Vec<GoodsReceiptLine>, ApiError> {
        let mut result = Vec::with_capacity(lines.len());

        for line in lines {
            let row: GoodsReceiptLineRow = sqlx::query_as(
                r#"
                INSERT INTO goods_receipt_lines (receipt_id, order_line_id, product_id, quantity,
                                                  condition, notes)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id, receipt_id, order_line_id, product_id, quantity, condition, notes,
                          deleted_at, deleted_by
                "#,
            )
            .bind(receipt_id)
            .bind(line.order_line_id)
            .bind(line.product_id)
            .bind(line.quantity)
            .bind(&line.condition)
            .bind(&line.notes)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "GoodsReceiptLine"))?;

            result.push(row.into());
        }

        Ok(result)
    }

    async fn find_by_receipt(&self, receipt_id: i64) -> Result<Vec<GoodsReceiptLine>, ApiError> {
        let rows: Vec<GoodsReceiptLineRow> = sqlx::query_as(
            r#"
            SELECT id, receipt_id, order_line_id, product_id, quantity, condition, notes,
                   deleted_at, deleted_by
            FROM goods_receipt_lines
            WHERE receipt_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(receipt_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find goods receipt lines by receipt: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete_by_receipt(&self, receipt_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            DELETE FROM goods_receipt_lines
            WHERE receipt_id = $1
            "#,
        )
        .bind(receipt_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete goods receipt lines: {}", e)))?;

        Ok(())
    }

    async fn soft_delete_by_receipt(
        &self,
        receipt_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            UPDATE goods_receipt_lines
            SET deleted_at = NOW(), deleted_by = $2
            WHERE receipt_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(receipt_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to soft delete goods receipt lines: {}", e))
        })?;

        Ok(())
    }

    async fn restore_by_receipt(&self, receipt_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            UPDATE goods_receipt_lines
            SET deleted_at = NULL, deleted_by = NULL
            WHERE receipt_id = $1 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(receipt_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore goods receipt lines: {}", e)))?;

        Ok(())
    }
}

// ============================================================================
// PurchaseRequest Row and Repository
// ============================================================================

/// Database row representation for PurchaseRequest
#[derive(Debug, FromRow)]
struct PurchaseRequestRow {
    id: i64,
    tenant_id: i64,
    request_number: String,
    status: String,
    requested_by: i64,
    department: Option<String>,
    priority: String,
    reason: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<PurchaseRequestRow> for PurchaseRequest {
    fn from(row: PurchaseRequestRow) -> Self {
        let status = parse_purchase_request_status(&row.status);

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            request_number: row.request_number,
            status,
            requested_by: row.requested_by,
            department: row.department,
            priority: row.priority,
            reason: row.reason,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// Row for paginated queries that includes a total count
#[derive(Debug, FromRow)]
struct PurchaseRequestRowWithCount {
    id: i64,
    tenant_id: i64,
    request_number: String,
    status: String,
    requested_by: i64,
    department: Option<String>,
    priority: String,
    reason: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
    total_count: i64,
}

/// PostgreSQL purchase request repository
pub struct PostgresPurchaseRequestRepository {
    pool: Arc<PgPool>,
}

impl PostgresPurchaseRequestRepository {
    /// Create a new PostgreSQL purchase request repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxPurchaseRequestRepository {
        Arc::new(self) as BoxPurchaseRequestRepository
    }
}

#[async_trait]
impl PurchaseRequestRepository for PostgresPurchaseRequestRepository {
    async fn create(&self, create: CreatePurchaseRequest) -> Result<PurchaseRequest, ApiError> {
        let request_number = format!("PR-{}", chrono::Utc::now().timestamp());
        let status_str = PurchaseRequestStatus::Draft.to_string();

        let row: PurchaseRequestRow = sqlx::query_as(
            r#"
            INSERT INTO purchase_requests (tenant_id, request_number, status, requested_by,
                                            department, priority, reason, created_at, updated_at,
                                            deleted_at, deleted_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW(), NULL, NULL)
            RETURNING id, tenant_id, request_number, status, requested_by,
                      department, priority, reason, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(&request_number)
        .bind(&status_str)
        .bind(create.requested_by)
        .bind(&create.department)
        .bind(&create.priority)
        .bind(&create.reason)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "PurchaseRequest"))?;

        Ok(row.into())
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<PurchaseRequest>, ApiError> {
        let result: Option<PurchaseRequestRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, request_number, status, requested_by,
                   department, priority, reason, created_at, updated_at, deleted_at, deleted_by
            FROM purchase_requests
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find purchase request by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<PurchaseRequest>, ApiError> {
        let rows: Vec<PurchaseRequestRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, request_number, status, requested_by,
                   department, priority, reason, created_at, updated_at, deleted_at, deleted_by
            FROM purchase_requests
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find purchase requests by tenant: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<PurchaseRequest>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;

        let rows: Vec<PurchaseRequestRowWithCount> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, request_number, status, requested_by,
                   department, priority, reason, created_at, updated_at, deleted_at, deleted_by,
                   COUNT(*) OVER() AS total_count
            FROM purchase_requests
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find purchase requests by tenant paginated: {}",
                e
            ))
        })?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<PurchaseRequest> = rows
            .into_iter()
            .map(|r| {
                PurchaseRequestRow {
                    id: r.id,
                    tenant_id: r.tenant_id,
                    request_number: r.request_number,
                    status: r.status,
                    requested_by: r.requested_by,
                    department: r.department,
                    priority: r.priority,
                    reason: r.reason,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                    deleted_at: r.deleted_at,
                    deleted_by: r.deleted_by,
                }
                .into()
            })
            .collect();

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<Vec<PurchaseRequest>, ApiError> {
        let status_str = status.to_string();

        let rows: Vec<PurchaseRequestRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, request_number, status, requested_by,
                   department, priority, reason, created_at, updated_at, deleted_at, deleted_by
            FROM purchase_requests
            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find purchase requests by status: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<PurchaseRequest>, ApiError> {
        let status_str = status.to_string();
        let offset = page.saturating_sub(1) * per_page;

        let rows: Vec<PurchaseRequestRowWithCount> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, request_number, status, requested_by,
                   department, priority, reason, created_at, updated_at, deleted_at, deleted_by,
                   COUNT(*) OVER() AS total_count
            FROM purchase_requests
            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find purchase requests by status paginated: {}",
                e
            ))
        })?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<PurchaseRequest> = rows
            .into_iter()
            .map(|r| {
                PurchaseRequestRow {
                    id: r.id,
                    tenant_id: r.tenant_id,
                    request_number: r.request_number,
                    status: r.status,
                    requested_by: r.requested_by,
                    department: r.department,
                    priority: r.priority,
                    reason: r.reason,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                    deleted_at: r.deleted_at,
                    deleted_by: r.deleted_by,
                }
                .into()
            })
            .collect();

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_requester(&self, requested_by: i64) -> Result<Vec<PurchaseRequest>, ApiError> {
        let rows: Vec<PurchaseRequestRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, request_number, status, requested_by,
                   department, priority, reason, created_at, updated_at, deleted_at, deleted_by
            FROM purchase_requests
            WHERE requested_by = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(requested_by)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find purchase requests by requester: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn count_by_tenant(&self, tenant_id: i64) -> Result<u64, ApiError> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) as count
            FROM purchase_requests
            WHERE tenant_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to count purchase requests by tenant: {}",
                e
            ))
        })?;

        Ok(result.0 as u64)
    }

    async fn count_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<u64, ApiError> {
        let status_str = status.to_string();

        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) as count
            FROM purchase_requests
            WHERE tenant_id = $1 AND status = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(&status_str)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to count purchase requests by status: {}",
                e
            ))
        })?;

        Ok(result.0 as u64)
    }

    async fn update(
        &self,
        id: i64,
        update: UpdatePurchaseRequest,
    ) -> Result<PurchaseRequest, ApiError> {
        let status_str = update.status.map(|s| s.to_string());

        let row: PurchaseRequestRow = sqlx::query_as(
            r#"
            UPDATE purchase_requests
            SET department = COALESCE($1, department),
                priority = COALESCE($2, priority),
                reason = COALESCE($3, reason),
                status = COALESCE($4, status),
                updated_at = NOW()
            WHERE id = $5 AND deleted_at IS NULL
            RETURNING id, tenant_id, request_number, status, requested_by,
                      department, priority, reason, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(&update.department)
        .bind(&update.priority)
        .bind(&update.reason)
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "PurchaseRequest"))?;

        Ok(row.into())
    }

    async fn update_status(
        &self,
        id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<PurchaseRequest, ApiError> {
        let status_str = status.to_string();

        let row: PurchaseRequestRow = sqlx::query_as(
            r#"
            UPDATE purchase_requests
            SET status = $1, updated_at = NOW()
            WHERE id = $2 AND deleted_at IS NULL
            RETURNING id, tenant_id, request_number, status, requested_by,
                      department, priority, reason, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "PurchaseRequest"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE purchase_requests
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to soft delete purchase request: {}", e))
        })?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "PurchaseRequest not found or already deleted".to_string(),
            ));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<PurchaseRequest, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE purchase_requests
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore purchase request: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "PurchaseRequest not found or not deleted".to_string(),
            ));
        }

        self.find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("PurchaseRequest not found".to_string()))
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<PurchaseRequest>, ApiError> {
        let rows: Vec<PurchaseRequestRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, request_number, status, requested_by,
                   department, priority, reason, created_at, updated_at, deleted_at, deleted_by
            FROM purchase_requests
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find deleted purchase requests: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM purchase_requests
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to permanently delete purchase request: {}",
                e
            ))
        })?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "PurchaseRequest not found or not soft-deleted".to_string(),
            ));
        }

        Ok(())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM purchase_requests
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete purchase request: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("PurchaseRequest not found".to_string()));
        }

        Ok(())
    }
}

// ============================================================================
// PurchaseRequestLine Row and Repository
// ============================================================================

/// Database row representation for PurchaseRequestLine
#[derive(Debug, FromRow)]
struct PurchaseRequestLineRow {
    id: i64,
    request_id: i64,
    product_id: Option<i64>,
    description: String,
    quantity: Decimal,
    notes: Option<String>,
    sort_order: i32,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<PurchaseRequestLineRow> for PurchaseRequestLine {
    fn from(row: PurchaseRequestLineRow) -> Self {
        Self {
            id: row.id,
            request_id: row.request_id,
            product_id: row.product_id,
            description: row.description,
            quantity: row.quantity,
            notes: row.notes,
            sort_order: row.sort_order,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL purchase request line repository
pub struct PostgresPurchaseRequestLineRepository {
    pool: Arc<PgPool>,
}

impl PostgresPurchaseRequestLineRepository {
    /// Create a new PostgreSQL purchase request line repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxPurchaseRequestLineRepository {
        Arc::new(self) as BoxPurchaseRequestLineRepository
    }
}

#[async_trait]
impl PurchaseRequestLineRepository for PostgresPurchaseRequestLineRepository {
    async fn create_many(
        &self,
        request_id: i64,
        lines: Vec<CreatePurchaseRequestLine>,
    ) -> Result<Vec<PurchaseRequestLine>, ApiError> {
        let mut result = Vec::with_capacity(lines.len());

        for (i, line) in lines.into_iter().enumerate() {
            let row: PurchaseRequestLineRow = sqlx::query_as(
                r#"
                INSERT INTO purchase_request_lines (request_id, product_id, description, quantity,
                                                     notes, sort_order)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id, request_id, product_id, description, quantity, notes, sort_order,
                          deleted_at, deleted_by
                "#,
            )
            .bind(request_id)
            .bind(line.product_id)
            .bind(&line.description)
            .bind(line.quantity)
            .bind(&line.notes)
            .bind(i as i32)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "PurchaseRequestLine"))?;

            result.push(row.into());
        }

        Ok(result)
    }

    async fn find_by_request(&self, request_id: i64) -> Result<Vec<PurchaseRequestLine>, ApiError> {
        let rows: Vec<PurchaseRequestLineRow> = sqlx::query_as(
            r#"
            SELECT id, request_id, product_id, description, quantity, notes, sort_order,
                   deleted_at, deleted_by
            FROM purchase_request_lines
            WHERE request_id = $1 AND deleted_at IS NULL
            ORDER BY sort_order
            "#,
        )
        .bind(request_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find purchase request lines by request: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete_by_request(&self, request_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            DELETE FROM purchase_request_lines
            WHERE request_id = $1
            "#,
        )
        .bind(request_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to delete purchase request lines: {}", e))
        })?;

        Ok(())
    }

    async fn soft_delete_by_request(
        &self,
        request_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            UPDATE purchase_request_lines
            SET deleted_at = NOW(), deleted_by = $2
            WHERE request_id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(request_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to soft delete purchase request lines by request: {}",
                e
            ))
        })?;
        Ok(())
    }

    async fn restore_by_request(&self, request_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            UPDATE purchase_request_lines
            SET deleted_at = NULL, deleted_by = NULL
            WHERE request_id = $1
            "#,
        )
        .bind(request_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to restore purchase request lines by request: {}",
                e
            ))
        })?;
        Ok(())
    }
}
