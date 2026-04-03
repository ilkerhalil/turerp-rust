//! Purchase repository

use async_trait::async_trait;
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::domain::purchase::model::{
    CreateGoodsReceipt, CreateGoodsReceiptLine, CreatePurchaseOrder, CreatePurchaseOrderLine,
    CreatePurchaseRequest, CreatePurchaseRequestLine, GoodsReceipt, GoodsReceiptLine,
    GoodsReceiptStatus, PurchaseOrder, PurchaseOrderLine, PurchaseOrderStatus, PurchaseRequest,
    PurchaseRequestLine, PurchaseRequestStatus, UpdatePurchaseRequest,
};
use crate::error::ApiError;

/// Repository trait for PurchaseOrder operations
#[async_trait]
pub trait PurchaseOrderRepository: Send + Sync {
    async fn create(&self, order: CreatePurchaseOrder) -> Result<PurchaseOrder, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseOrder>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<PurchaseOrder>, ApiError>;
    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<PurchaseOrder>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseOrderStatus,
    ) -> Result<Vec<PurchaseOrder>, ApiError>;
    async fn update_status(
        &self,
        id: i64,
        status: PurchaseOrderStatus,
    ) -> Result<PurchaseOrder, ApiError>;
    async fn update_line_received_quantity(
        &self,
        line_id: i64,
        received_qty: Decimal,
    ) -> Result<PurchaseOrderLine, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for PurchaseOrderLine operations
#[async_trait]
pub trait PurchaseOrderLineRepository: Send + Sync {
    async fn create_many(
        &self,
        order_id: i64,
        lines: Vec<CreatePurchaseOrderLine>,
    ) -> Result<Vec<PurchaseOrderLine>, ApiError>;
    async fn find_by_order(&self, order_id: i64) -> Result<Vec<PurchaseOrderLine>, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseOrderLine>, ApiError>;
    async fn delete_by_order(&self, order_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for GoodsReceipt operations
#[async_trait]
pub trait GoodsReceiptRepository: Send + Sync {
    async fn create(&self, receipt: CreateGoodsReceipt) -> Result<GoodsReceipt, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<GoodsReceipt>, ApiError>;
    async fn find_by_order(&self, order_id: i64) -> Result<Vec<GoodsReceipt>, ApiError>;
    async fn update_status(
        &self,
        id: i64,
        status: GoodsReceiptStatus,
    ) -> Result<GoodsReceipt, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for GoodsReceiptLine operations
#[async_trait]
pub trait GoodsReceiptLineRepository: Send + Sync {
    async fn create_many(
        &self,
        receipt_id: i64,
        lines: Vec<CreateGoodsReceiptLine>,
    ) -> Result<Vec<GoodsReceiptLine>, ApiError>;
    async fn find_by_receipt(&self, receipt_id: i64) -> Result<Vec<GoodsReceiptLine>, ApiError>;
    async fn delete_by_receipt(&self, receipt_id: i64) -> Result<(), ApiError>;
}

/// Type aliases
pub type BoxPurchaseOrderRepository = Arc<dyn PurchaseOrderRepository>;
pub type BoxPurchaseOrderLineRepository = Arc<dyn PurchaseOrderLineRepository>;
pub type BoxGoodsReceiptRepository = Arc<dyn GoodsReceiptRepository>;
pub type BoxGoodsReceiptLineRepository = Arc<dyn GoodsReceiptLineRepository>;
pub type BoxPurchaseRequestRepository = Arc<dyn PurchaseRequestRepository>;
pub type BoxPurchaseRequestLineRepository = Arc<dyn PurchaseRequestLineRepository>;

/// Repository trait for PurchaseRequest operations
#[async_trait]
pub trait PurchaseRequestRepository: Send + Sync {
    async fn create(&self, request: CreatePurchaseRequest) -> Result<PurchaseRequest, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseRequest>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<PurchaseRequest>, ApiError>;
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<PurchaseRequest>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<Vec<PurchaseRequest>, ApiError>;
    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<PurchaseRequest>, ApiError>;
    async fn find_by_requester(&self, requested_by: i64) -> Result<Vec<PurchaseRequest>, ApiError>;
    async fn count_by_tenant(&self, tenant_id: i64) -> Result<u64, ApiError>;
    async fn count_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<u64, ApiError>;
    async fn update(
        &self,
        id: i64,
        update: UpdatePurchaseRequest,
    ) -> Result<PurchaseRequest, ApiError>;
    async fn update_status(
        &self,
        id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<PurchaseRequest, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for PurchaseRequestLine operations
#[async_trait]
pub trait PurchaseRequestLineRepository: Send + Sync {
    async fn create_many(
        &self,
        request_id: i64,
        lines: Vec<CreatePurchaseRequestLine>,
    ) -> Result<Vec<PurchaseRequestLine>, ApiError>;
    async fn find_by_request(&self, request_id: i64) -> Result<Vec<PurchaseRequestLine>, ApiError>;
    async fn delete_by_request(&self, request_id: i64) -> Result<(), ApiError>;
}

fn calculate_totals(lines: &[CreatePurchaseOrderLine]) -> (Decimal, Decimal, Decimal, Decimal) {
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

fn generate_order_number(count: i64) -> String {
    format!("PO-{:06}", count)
}

fn generate_receipt_number(count: i64) -> String {
    format!("GR-{:06}", count)
}

fn generate_request_number(count: i64) -> String {
    format!("PR-{:06}", count)
}

/// Inner state for InMemoryPurchaseOrderRepository
struct InMemoryPurchaseOrderInner {
    orders: std::collections::HashMap<i64, PurchaseOrder>,
    next_id: i64,
}

/// In-memory purchase order repository
pub struct InMemoryPurchaseOrderRepository {
    inner: Mutex<InMemoryPurchaseOrderInner>,
}

impl InMemoryPurchaseOrderRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryPurchaseOrderInner {
                orders: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryPurchaseOrderRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PurchaseOrderRepository for InMemoryPurchaseOrderRepository {
    async fn create(&self, create: CreatePurchaseOrder) -> Result<PurchaseOrder, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let order_number = generate_order_number(id);
        let (subtotal, tax_amount, discount_amount, total_amount) = calculate_totals(&create.lines);
        let now = chrono::Utc::now();

        let order = PurchaseOrder {
            id,
            tenant_id: create.tenant_id,
            order_number,
            cari_id: create.cari_id,
            status: PurchaseOrderStatus::Draft,
            order_date: create.order_date,
            expected_delivery_date: create.expected_delivery_date,
            subtotal,
            tax_amount,
            discount_amount,
            total_amount,
            notes: create.notes,
            created_at: now,
            updated_at: now,
        };

        inner.orders.insert(id, order.clone());
        Ok(order)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseOrder>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.orders.get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<PurchaseOrder>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .orders
            .values()
            .filter(|o| o.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<PurchaseOrder>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .orders
            .values()
            .filter(|o| o.cari_id == cari_id)
            .cloned()
            .collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseOrderStatus,
    ) -> Result<Vec<PurchaseOrder>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .orders
            .values()
            .filter(|o| o.tenant_id == tenant_id && o.status == status)
            .cloned()
            .collect())
    }

    async fn update_status(
        &self,
        id: i64,
        status: PurchaseOrderStatus,
    ) -> Result<PurchaseOrder, ApiError> {
        let mut inner = self.inner.lock();
        let order = inner
            .orders
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Purchase order {} not found", id)))?;
        order.status = status;
        order.updated_at = chrono::Utc::now();
        Ok(order.clone())
    }

    async fn update_line_received_quantity(
        &self,
        _line_id: i64,
        _received_qty: Decimal,
    ) -> Result<PurchaseOrderLine, ApiError> {
        // This would need access to lines - simplified for now
        Err(ApiError::NotFound("Line not found".to_string()))
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.orders.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryPurchaseOrderLineRepository
struct InMemoryPurchaseOrderLineInner {
    lines: std::collections::HashMap<i64, Vec<PurchaseOrderLine>>,
    next_id: i64,
}

/// In-memory purchase order line repository
pub struct InMemoryPurchaseOrderLineRepository {
    inner: Mutex<InMemoryPurchaseOrderLineInner>,
}

impl InMemoryPurchaseOrderLineRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryPurchaseOrderLineInner {
                lines: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryPurchaseOrderLineRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PurchaseOrderLineRepository for InMemoryPurchaseOrderLineRepository {
    async fn create_many(
        &self,
        order_id: i64,
        create_lines: Vec<CreatePurchaseOrderLine>,
    ) -> Result<Vec<PurchaseOrderLine>, ApiError> {
        let mut inner = self.inner.lock();
        let mut lines = Vec::new();

        for (i, create) in create_lines.into_iter().enumerate() {
            let id = inner.next_id;
            inner.next_id += 1;

            let line_total = create.calculate_line_total();

            lines.push(PurchaseOrderLine {
                id,
                order_id,
                product_id: create.product_id,
                description: create.description,
                quantity: create.quantity,
                received_quantity: Decimal::ZERO,
                unit_price: create.unit_price,
                tax_rate: create.tax_rate,
                discount_rate: create.discount_rate,
                line_total,
                sort_order: i as i32,
            });
        }

        inner.lines.insert(order_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_order(&self, order_id: i64) -> Result<Vec<PurchaseOrderLine>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.lines.get(&order_id).cloned().unwrap_or_default())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseOrderLine>, ApiError> {
        let inner = self.inner.lock();
        for line_set in inner.lines.values() {
            if let Some(line) = line_set.iter().find(|l| l.id == id) {
                return Ok(Some(line.clone()));
            }
        }
        Ok(None)
    }

    async fn delete_by_order(&self, order_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.lines.remove(&order_id);
        Ok(())
    }
}

/// Inner state for InMemoryGoodsReceiptRepository
struct InMemoryGoodsReceiptInner {
    receipts: std::collections::HashMap<i64, GoodsReceipt>,
    next_id: i64,
}

/// In-memory goods receipt repository
pub struct InMemoryGoodsReceiptRepository {
    inner: Mutex<InMemoryGoodsReceiptInner>,
}

impl InMemoryGoodsReceiptRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryGoodsReceiptInner {
                receipts: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryGoodsReceiptRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GoodsReceiptRepository for InMemoryGoodsReceiptRepository {
    async fn create(&self, create: CreateGoodsReceipt) -> Result<GoodsReceipt, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let receipt_number = generate_receipt_number(id);
        let now = chrono::Utc::now();

        let receipt = GoodsReceipt {
            id,
            tenant_id: create.tenant_id,
            receipt_number,
            purchase_order_id: create.purchase_order_id,
            status: GoodsReceiptStatus::Pending,
            receipt_date: create.receipt_date,
            notes: create.notes,
            created_at: now,
        };

        inner.receipts.insert(id, receipt.clone());
        Ok(receipt)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<GoodsReceipt>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.receipts.get(&id).cloned())
    }

    async fn find_by_order(&self, order_id: i64) -> Result<Vec<GoodsReceipt>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .receipts
            .values()
            .filter(|r| r.purchase_order_id == order_id)
            .cloned()
            .collect())
    }

    async fn update_status(
        &self,
        id: i64,
        status: GoodsReceiptStatus,
    ) -> Result<GoodsReceipt, ApiError> {
        let mut inner = self.inner.lock();
        let receipt = inner
            .receipts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Goods receipt {} not found", id)))?;
        receipt.status = status;
        Ok(receipt.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.receipts.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryGoodsReceiptLineRepository
struct InMemoryGoodsReceiptLineInner {
    lines: std::collections::HashMap<i64, Vec<GoodsReceiptLine>>,
    next_id: i64,
}

/// In-memory goods receipt line repository
pub struct InMemoryGoodsReceiptLineRepository {
    inner: Mutex<InMemoryGoodsReceiptLineInner>,
}

impl InMemoryGoodsReceiptLineRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryGoodsReceiptLineInner {
                lines: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryGoodsReceiptLineRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GoodsReceiptLineRepository for InMemoryGoodsReceiptLineRepository {
    async fn create_many(
        &self,
        receipt_id: i64,
        create_lines: Vec<CreateGoodsReceiptLine>,
    ) -> Result<Vec<GoodsReceiptLine>, ApiError> {
        let mut inner = self.inner.lock();
        let mut lines = Vec::new();

        for create in create_lines {
            let id = inner.next_id;
            inner.next_id += 1;

            lines.push(GoodsReceiptLine {
                id,
                receipt_id,
                order_line_id: create.order_line_id,
                product_id: create.product_id,
                quantity: create.quantity,
                condition: create.condition,
                notes: create.notes,
            });
        }

        inner.lines.insert(receipt_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_receipt(&self, receipt_id: i64) -> Result<Vec<GoodsReceiptLine>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.lines.get(&receipt_id).cloned().unwrap_or_default())
    }

    async fn delete_by_receipt(&self, receipt_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.lines.remove(&receipt_id);
        Ok(())
    }
}

/// Inner state for InMemoryPurchaseRequestRepository
struct InMemoryPurchaseRequestInner {
    requests: std::collections::HashMap<i64, PurchaseRequest>,
    next_id: i64,
}

/// In-memory purchase request repository
pub struct InMemoryPurchaseRequestRepository {
    inner: Mutex<InMemoryPurchaseRequestInner>,
}

impl InMemoryPurchaseRequestRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryPurchaseRequestInner {
                requests: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryPurchaseRequestRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PurchaseRequestRepository for InMemoryPurchaseRequestRepository {
    async fn create(&self, create: CreatePurchaseRequest) -> Result<PurchaseRequest, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let request_number = generate_request_number(id);
        let now = chrono::Utc::now();

        let request = PurchaseRequest {
            id,
            tenant_id: create.tenant_id,
            request_number,
            status: PurchaseRequestStatus::Draft,
            requested_by: create.requested_by,
            department: create.department,
            priority: create.priority,
            reason: create.reason,
            created_at: now,
            updated_at: now,
        };

        inner.requests.insert(id, request.clone());
        Ok(request)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseRequest>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.requests.get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<PurchaseRequest>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .requests
            .values()
            .filter(|r| r.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<PurchaseRequest>, ApiError> {
        let inner = self.inner.lock();
        let total = inner
            .requests
            .values()
            .filter(|r| r.tenant_id == tenant_id)
            .count() as u64;

        let items: Vec<PurchaseRequest> = inner
            .requests
            .values()
            .filter(|r| r.tenant_id == tenant_id)
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .cloned()
            .collect();

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<Vec<PurchaseRequest>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .requests
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.status == status)
            .cloned()
            .collect())
    }

    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<PurchaseRequest>, ApiError> {
        let inner = self.inner.lock();
        let total = inner
            .requests
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.status == status)
            .count() as u64;

        let items: Vec<PurchaseRequest> = inner
            .requests
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.status == status)
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .cloned()
            .collect();

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_requester(&self, requested_by: i64) -> Result<Vec<PurchaseRequest>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .requests
            .values()
            .filter(|r| r.requested_by == requested_by)
            .cloned()
            .collect())
    }

    async fn count_by_tenant(&self, tenant_id: i64) -> Result<u64, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .requests
            .values()
            .filter(|r| r.tenant_id == tenant_id)
            .count() as u64)
    }

    async fn count_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<u64, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .requests
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.status == status)
            .count() as u64)
    }

    async fn update(
        &self,
        id: i64,
        update: UpdatePurchaseRequest,
    ) -> Result<PurchaseRequest, ApiError> {
        let mut inner = self.inner.lock();
        let request = inner
            .requests
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Purchase request {} not found", id)))?;

        if let Some(department) = update.department {
            request.department = Some(department);
        }
        if let Some(priority) = update.priority {
            request.priority = priority;
        }
        if let Some(reason) = update.reason {
            request.reason = Some(reason);
        }
        if let Some(status) = update.status {
            request.status = status;
        }
        request.updated_at = chrono::Utc::now();

        Ok(request.clone())
    }

    async fn update_status(
        &self,
        id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<PurchaseRequest, ApiError> {
        let mut inner = self.inner.lock();
        let request = inner
            .requests
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Purchase request {} not found", id)))?;
        request.status = status;
        request.updated_at = chrono::Utc::now();
        Ok(request.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.requests.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryPurchaseRequestLineRepository
struct InMemoryPurchaseRequestLineInner {
    lines: std::collections::HashMap<i64, Vec<PurchaseRequestLine>>,
    next_id: i64,
}

/// In-memory purchase request line repository
pub struct InMemoryPurchaseRequestLineRepository {
    inner: Mutex<InMemoryPurchaseRequestLineInner>,
}

impl InMemoryPurchaseRequestLineRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryPurchaseRequestLineInner {
                lines: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryPurchaseRequestLineRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PurchaseRequestLineRepository for InMemoryPurchaseRequestLineRepository {
    async fn create_many(
        &self,
        request_id: i64,
        create_lines: Vec<CreatePurchaseRequestLine>,
    ) -> Result<Vec<PurchaseRequestLine>, ApiError> {
        let mut inner = self.inner.lock();
        let mut lines = Vec::new();

        for (i, create) in create_lines.into_iter().enumerate() {
            let id = inner.next_id;
            inner.next_id += 1;

            lines.push(PurchaseRequestLine {
                id,
                request_id,
                product_id: create.product_id,
                description: create.description,
                quantity: create.quantity,
                notes: create.notes,
                sort_order: i as i32,
            });
        }

        inner.lines.insert(request_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_request(&self, request_id: i64) -> Result<Vec<PurchaseRequestLine>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.lines.get(&request_id).cloned().unwrap_or_default())
    }

    async fn delete_by_request(&self, request_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.lines.remove(&request_id);
        Ok(())
    }
}
