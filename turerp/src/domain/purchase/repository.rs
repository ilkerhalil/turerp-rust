//! Purchase repository

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

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
        received_qty: f64,
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
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<Vec<PurchaseRequest>, ApiError>;
    async fn find_by_requester(&self, requested_by: i64) -> Result<Vec<PurchaseRequest>, ApiError>;
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

fn calculate_totals(lines: &[CreatePurchaseOrderLine]) -> (f64, f64, f64, f64) {
    let mut subtotal = 0.0;
    let mut tax_amount = 0.0;
    let mut discount_amount = 0.0;

    for line in lines {
        let line_subtotal = line.quantity * line.unit_price;
        let line_discount = line_subtotal * (line.discount_rate / 100.0);
        let after_discount = line_subtotal - line_discount;
        let line_tax = after_discount * (line.tax_rate / 100.0);

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

/// In-memory purchase order repository
pub struct InMemoryPurchaseOrderRepository {
    orders: Mutex<std::collections::HashMap<i64, PurchaseOrder>>,
    next_id: Mutex<i64>,
}

impl InMemoryPurchaseOrderRepository {
    pub fn new() -> Self {
        Self {
            orders: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
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
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;

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

        self.orders.lock().insert(id, order.clone());
        Ok(order)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseOrder>, ApiError> {
        Ok(self.orders.lock().get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<PurchaseOrder>, ApiError> {
        let orders = self.orders.lock();
        Ok(orders
            .values()
            .filter(|o| o.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<PurchaseOrder>, ApiError> {
        let orders = self.orders.lock();
        Ok(orders
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
        let orders = self.orders.lock();
        Ok(orders
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
        let mut orders = self.orders.lock();
        let order = orders
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Purchase order {} not found", id)))?;
        order.status = status;
        order.updated_at = chrono::Utc::now();
        Ok(order.clone())
    }

    async fn update_line_received_quantity(
        &self,
        _line_id: i64,
        _received_qty: f64,
    ) -> Result<PurchaseOrderLine, ApiError> {
        // This would need access to lines - simplified for now
        Err(ApiError::NotFound("Line not found".to_string()))
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.orders.lock().remove(&id);
        Ok(())
    }
}

/// In-memory purchase order line repository
pub struct InMemoryPurchaseOrderLineRepository {
    lines: Mutex<std::collections::HashMap<i64, Vec<PurchaseOrderLine>>>,
    next_id: Mutex<i64>,
}

impl InMemoryPurchaseOrderLineRepository {
    pub fn new() -> Self {
        Self {
            lines: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
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
        let mut next_id = self.next_id.lock();
        let mut lines = Vec::new();

        for (i, create) in create_lines.into_iter().enumerate() {
            let id = *next_id;
            *next_id += 1;

            let line_total = create.calculate_line_total();

            lines.push(PurchaseOrderLine {
                id,
                order_id,
                product_id: create.product_id,
                description: create.description,
                quantity: create.quantity,
                received_quantity: 0.0,
                unit_price: create.unit_price,
                tax_rate: create.tax_rate,
                discount_rate: create.discount_rate,
                line_total,
                sort_order: i as i32,
            });
        }

        self.lines.lock().insert(order_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_order(&self, order_id: i64) -> Result<Vec<PurchaseOrderLine>, ApiError> {
        Ok(self
            .lines
            .lock()
            .get(&order_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseOrderLine>, ApiError> {
        let lines = self.lines.lock();
        for line_set in lines.values() {
            if let Some(line) = line_set.iter().find(|l| l.id == id) {
                return Ok(Some(line.clone()));
            }
        }
        Ok(None)
    }

    async fn delete_by_order(&self, order_id: i64) -> Result<(), ApiError> {
        self.lines.lock().remove(&order_id);
        Ok(())
    }
}

/// In-memory goods receipt repository
pub struct InMemoryGoodsReceiptRepository {
    receipts: Mutex<std::collections::HashMap<i64, GoodsReceipt>>,
    next_id: Mutex<i64>,
}

impl InMemoryGoodsReceiptRepository {
    pub fn new() -> Self {
        Self {
            receipts: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
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
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;

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

        self.receipts.lock().insert(id, receipt.clone());
        Ok(receipt)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<GoodsReceipt>, ApiError> {
        Ok(self.receipts.lock().get(&id).cloned())
    }

    async fn find_by_order(&self, order_id: i64) -> Result<Vec<GoodsReceipt>, ApiError> {
        let receipts = self.receipts.lock();
        Ok(receipts
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
        let mut receipts = self.receipts.lock();
        let receipt = receipts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Goods receipt {} not found", id)))?;
        receipt.status = status;
        Ok(receipt.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.receipts.lock().remove(&id);
        Ok(())
    }
}

/// In-memory goods receipt line repository
pub struct InMemoryGoodsReceiptLineRepository {
    lines: Mutex<std::collections::HashMap<i64, Vec<GoodsReceiptLine>>>,
    next_id: Mutex<i64>,
}

impl InMemoryGoodsReceiptLineRepository {
    pub fn new() -> Self {
        Self {
            lines: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
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
        let mut next_id = self.next_id.lock();
        let mut lines = Vec::new();

        for create in create_lines {
            let id = *next_id;
            *next_id += 1;

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

        self.lines.lock().insert(receipt_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_receipt(&self, receipt_id: i64) -> Result<Vec<GoodsReceiptLine>, ApiError> {
        Ok(self
            .lines
            .lock()
            .get(&receipt_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn delete_by_receipt(&self, receipt_id: i64) -> Result<(), ApiError> {
        self.lines.lock().remove(&receipt_id);
        Ok(())
    }
}

/// In-memory purchase request repository
pub struct InMemoryPurchaseRequestRepository {
    requests: Mutex<std::collections::HashMap<i64, PurchaseRequest>>,
    next_id: Mutex<i64>,
}

impl InMemoryPurchaseRequestRepository {
    pub fn new() -> Self {
        Self {
            requests: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
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
        let mut next_id = self.next_id.lock();
        let id = *next_id;
        *next_id += 1;

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

        self.requests.lock().insert(id, request.clone());
        Ok(request)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseRequest>, ApiError> {
        Ok(self.requests.lock().get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<PurchaseRequest>, ApiError> {
        let requests = self.requests.lock();
        Ok(requests
            .values()
            .filter(|r| r.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: PurchaseRequestStatus,
    ) -> Result<Vec<PurchaseRequest>, ApiError> {
        let requests = self.requests.lock();
        Ok(requests
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.status == status)
            .cloned()
            .collect())
    }

    async fn find_by_requester(&self, requested_by: i64) -> Result<Vec<PurchaseRequest>, ApiError> {
        let requests = self.requests.lock();
        Ok(requests
            .values()
            .filter(|r| r.requested_by == requested_by)
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        id: i64,
        update: UpdatePurchaseRequest,
    ) -> Result<PurchaseRequest, ApiError> {
        let mut requests = self.requests.lock();
        let request = requests
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
        let mut requests = self.requests.lock();
        let request = requests
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Purchase request {} not found", id)))?;
        request.status = status;
        request.updated_at = chrono::Utc::now();
        Ok(request.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.requests.lock().remove(&id);
        Ok(())
    }
}

/// In-memory purchase request line repository
pub struct InMemoryPurchaseRequestLineRepository {
    lines: Mutex<std::collections::HashMap<i64, Vec<PurchaseRequestLine>>>,
    next_id: Mutex<i64>,
}

impl InMemoryPurchaseRequestLineRepository {
    pub fn new() -> Self {
        Self {
            lines: Mutex::new(std::collections::HashMap::new()),
            next_id: Mutex::new(1),
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
        let mut next_id = self.next_id.lock();
        let mut lines = Vec::new();

        for (i, create) in create_lines.into_iter().enumerate() {
            let id = *next_id;
            *next_id += 1;

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

        self.lines.lock().insert(request_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_request(&self, request_id: i64) -> Result<Vec<PurchaseRequestLine>, ApiError> {
        Ok(self
            .lines
            .lock()
            .get(&request_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn delete_by_request(&self, request_id: i64) -> Result<(), ApiError> {
        self.lines.lock().remove(&request_id);
        Ok(())
    }
}
