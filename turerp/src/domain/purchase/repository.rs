//! Purchase repository

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::purchase::model::{
    CreateGoodsReceipt, CreateGoodsReceiptLine, CreatePurchaseOrder, CreatePurchaseOrderLine,
    GoodsReceipt, GoodsReceiptLine, GoodsReceiptStatus, PurchaseOrder, PurchaseOrderLine,
    PurchaseOrderStatus,
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

/// In-memory purchase order repository
pub struct InMemoryPurchaseOrderRepository {
    orders: std::sync::Mutex<std::collections::HashMap<i64, PurchaseOrder>>,
    next_id: std::sync::Mutex<i64>,
}

impl InMemoryPurchaseOrderRepository {
    pub fn new() -> Self {
        Self {
            orders: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
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
        let mut next_id = self.next_id.lock().unwrap();
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

        self.orders.lock().unwrap().insert(id, order.clone());
        Ok(order)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseOrder>, ApiError> {
        Ok(self.orders.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<PurchaseOrder>, ApiError> {
        let orders = self.orders.lock().unwrap();
        Ok(orders
            .values()
            .filter(|o| o.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<PurchaseOrder>, ApiError> {
        let orders = self.orders.lock().unwrap();
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
        let orders = self.orders.lock().unwrap();
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
        let mut orders = self.orders.lock().unwrap();
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
        self.orders.lock().unwrap().remove(&id);
        Ok(())
    }
}

/// In-memory purchase order line repository
pub struct InMemoryPurchaseOrderLineRepository {
    lines: std::sync::Mutex<std::collections::HashMap<i64, Vec<PurchaseOrderLine>>>,
    next_id: std::sync::Mutex<i64>,
}

impl InMemoryPurchaseOrderLineRepository {
    pub fn new() -> Self {
        Self {
            lines: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
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
        let mut next_id = self.next_id.lock().unwrap();
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

        self.lines.lock().unwrap().insert(order_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_order(&self, order_id: i64) -> Result<Vec<PurchaseOrderLine>, ApiError> {
        Ok(self
            .lines
            .lock()
            .unwrap()
            .get(&order_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<PurchaseOrderLine>, ApiError> {
        let lines = self.lines.lock().unwrap();
        for line_set in lines.values() {
            if let Some(line) = line_set.iter().find(|l| l.id == id) {
                return Ok(Some(line.clone()));
            }
        }
        Ok(None)
    }

    async fn delete_by_order(&self, order_id: i64) -> Result<(), ApiError> {
        self.lines.lock().unwrap().remove(&order_id);
        Ok(())
    }
}

/// In-memory goods receipt repository
pub struct InMemoryGoodsReceiptRepository {
    receipts: std::sync::Mutex<std::collections::HashMap<i64, GoodsReceipt>>,
    next_id: std::sync::Mutex<i64>,
}

impl InMemoryGoodsReceiptRepository {
    pub fn new() -> Self {
        Self {
            receipts: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
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
        let mut next_id = self.next_id.lock().unwrap();
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

        self.receipts.lock().unwrap().insert(id, receipt.clone());
        Ok(receipt)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<GoodsReceipt>, ApiError> {
        Ok(self.receipts.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_order(&self, order_id: i64) -> Result<Vec<GoodsReceipt>, ApiError> {
        let receipts = self.receipts.lock().unwrap();
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
        let mut receipts = self.receipts.lock().unwrap();
        let receipt = receipts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Goods receipt {} not found", id)))?;
        receipt.status = status;
        Ok(receipt.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.receipts.lock().unwrap().remove(&id);
        Ok(())
    }
}

/// In-memory goods receipt line repository
pub struct InMemoryGoodsReceiptLineRepository {
    lines: std::sync::Mutex<std::collections::HashMap<i64, Vec<GoodsReceiptLine>>>,
    next_id: std::sync::Mutex<i64>,
}

impl InMemoryGoodsReceiptLineRepository {
    pub fn new() -> Self {
        Self {
            lines: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
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
        let mut next_id = self.next_id.lock().unwrap();
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

        self.lines.lock().unwrap().insert(receipt_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_receipt(&self, receipt_id: i64) -> Result<Vec<GoodsReceiptLine>, ApiError> {
        Ok(self
            .lines
            .lock()
            .unwrap()
            .get(&receipt_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn delete_by_receipt(&self, receipt_id: i64) -> Result<(), ApiError> {
        self.lines.lock().unwrap().remove(&receipt_id);
        Ok(())
    }
}
