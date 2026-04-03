//! Sales repository

use async_trait::async_trait;
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::domain::sales::model::{
    CreateQuotation, CreateQuotationLine, CreateSalesOrder, CreateSalesOrderLine, Quotation,
    QuotationLine, QuotationStatus, SalesOrder, SalesOrderLine, SalesOrderStatus,
};
use crate::error::ApiError;

/// Repository trait for SalesOrder operations
#[async_trait]
pub trait SalesOrderRepository: Send + Sync {
    async fn create(&self, order: CreateSalesOrder) -> Result<SalesOrder, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<SalesOrder>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<SalesOrder>, ApiError>;
    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<SalesOrder>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: SalesOrderStatus,
    ) -> Result<Vec<SalesOrder>, ApiError>;
    async fn update_status(
        &self,
        id: i64,
        status: SalesOrderStatus,
    ) -> Result<SalesOrder, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for SalesOrderLine operations
#[async_trait]
pub trait SalesOrderLineRepository: Send + Sync {
    async fn create_many(
        &self,
        order_id: i64,
        lines: Vec<CreateSalesOrderLine>,
    ) -> Result<Vec<SalesOrderLine>, ApiError>;
    async fn find_by_order(&self, order_id: i64) -> Result<Vec<SalesOrderLine>, ApiError>;
    async fn delete_by_order(&self, order_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for Quotation operations
#[async_trait]
pub trait QuotationRepository: Send + Sync {
    async fn create(&self, quotation: CreateQuotation) -> Result<Quotation, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Quotation>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Quotation>, ApiError>;
    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<Quotation>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: QuotationStatus,
    ) -> Result<Vec<Quotation>, ApiError>;
    async fn update_status(&self, id: i64, status: QuotationStatus) -> Result<Quotation, ApiError>;
    async fn link_to_order(&self, id: i64, order_id: i64) -> Result<Quotation, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for QuotationLine operations
#[async_trait]
pub trait QuotationLineRepository: Send + Sync {
    async fn create_many(
        &self,
        quotation_id: i64,
        lines: Vec<CreateQuotationLine>,
    ) -> Result<Vec<QuotationLine>, ApiError>;
    async fn find_by_quotation(&self, quotation_id: i64) -> Result<Vec<QuotationLine>, ApiError>;
    async fn delete_by_quotation(&self, quotation_id: i64) -> Result<(), ApiError>;
}

/// Type aliases
pub type BoxSalesOrderRepository = Arc<dyn SalesOrderRepository>;
pub type BoxSalesOrderLineRepository = Arc<dyn SalesOrderLineRepository>;
pub type BoxQuotationRepository = Arc<dyn QuotationRepository>;
pub type BoxQuotationLineRepository = Arc<dyn QuotationLineRepository>;

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

fn generate_order_number(count: i64) -> String {
    format!("SO-{:06}", count)
}

fn generate_quotation_number(count: i64) -> String {
    format!("QT-{:06}", count)
}

/// Inner state for InMemorySalesOrderRepository
struct InMemorySalesOrderInner {
    orders: std::collections::HashMap<i64, SalesOrder>,
    next_id: i64,
}

/// In-memory sales order repository
pub struct InMemorySalesOrderRepository {
    inner: Mutex<InMemorySalesOrderInner>,
}

impl InMemorySalesOrderRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemorySalesOrderInner {
                orders: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemorySalesOrderRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SalesOrderRepository for InMemorySalesOrderRepository {
    async fn create(&self, create: CreateSalesOrder) -> Result<SalesOrder, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let order_number = generate_order_number(id);
        let (subtotal, tax_amount, discount_amount, total_amount) = calculate_totals(&create.lines);
        let now = chrono::Utc::now();

        let order = SalesOrder {
            id,
            tenant_id: create.tenant_id,
            order_number,
            cari_id: create.cari_id,
            status: SalesOrderStatus::Draft,
            order_date: create.order_date,
            delivery_date: create.delivery_date,
            subtotal,
            tax_amount,
            discount_amount,
            total_amount,
            notes: create.notes,
            shipping_address: create.shipping_address,
            billing_address: create.billing_address,
            created_at: now,
            updated_at: now,
        };

        inner.orders.insert(id, order.clone());
        Ok(order)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<SalesOrder>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.orders.get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<SalesOrder>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .orders
            .values()
            .filter(|o| o.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<SalesOrder>, ApiError> {
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
        status: SalesOrderStatus,
    ) -> Result<Vec<SalesOrder>, ApiError> {
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
        status: SalesOrderStatus,
    ) -> Result<SalesOrder, ApiError> {
        let mut inner = self.inner.lock();
        let order = inner
            .orders
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Sales order {} not found", id)))?;
        order.status = status;
        order.updated_at = chrono::Utc::now();
        Ok(order.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.orders.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemorySalesOrderLineRepository
struct InMemorySalesOrderLineInner {
    lines: std::collections::HashMap<i64, Vec<SalesOrderLine>>,
    next_id: i64,
}

/// In-memory sales order line repository
pub struct InMemorySalesOrderLineRepository {
    inner: Mutex<InMemorySalesOrderLineInner>,
}

impl InMemorySalesOrderLineRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemorySalesOrderLineInner {
                lines: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemorySalesOrderLineRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SalesOrderLineRepository for InMemorySalesOrderLineRepository {
    async fn create_many(
        &self,
        order_id: i64,
        create_lines: Vec<CreateSalesOrderLine>,
    ) -> Result<Vec<SalesOrderLine>, ApiError> {
        let mut inner = self.inner.lock();
        let mut lines = Vec::new();

        for (i, create) in create_lines.into_iter().enumerate() {
            let id = inner.next_id;
            inner.next_id += 1;

            let line_total = create.calculate_line_total();

            lines.push(SalesOrderLine {
                id,
                order_id,
                product_id: create.product_id,
                description: create.description,
                quantity: create.quantity,
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

    async fn find_by_order(&self, order_id: i64) -> Result<Vec<SalesOrderLine>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.lines.get(&order_id).cloned().unwrap_or_default())
    }

    async fn delete_by_order(&self, order_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.lines.remove(&order_id);
        Ok(())
    }
}

/// Inner state for InMemoryQuotationRepository
struct InMemoryQuotationInner {
    quotations: std::collections::HashMap<i64, Quotation>,
    next_id: i64,
}

/// In-memory quotation repository
pub struct InMemoryQuotationRepository {
    inner: Mutex<InMemoryQuotationInner>,
}

impl InMemoryQuotationRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryQuotationInner {
                quotations: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryQuotationRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QuotationRepository for InMemoryQuotationRepository {
    async fn create(&self, create: CreateQuotation) -> Result<Quotation, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let quotation_number = generate_quotation_number(id);
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
        let now = chrono::Utc::now();

        let quotation = Quotation {
            id,
            tenant_id: create.tenant_id,
            quotation_number,
            cari_id: create.cari_id,
            status: QuotationStatus::Draft,
            valid_until: create.valid_until,
            subtotal,
            tax_amount,
            discount_amount,
            total_amount,
            notes: create.notes,
            terms: create.terms,
            sales_order_id: None,
            created_at: now,
            updated_at: now,
        };

        inner.quotations.insert(id, quotation.clone());
        Ok(quotation)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Quotation>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.quotations.get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Quotation>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .quotations
            .values()
            .filter(|q| q.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<Quotation>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .quotations
            .values()
            .filter(|q| q.cari_id == cari_id)
            .cloned()
            .collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: QuotationStatus,
    ) -> Result<Vec<Quotation>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .quotations
            .values()
            .filter(|q| q.tenant_id == tenant_id && q.status == status)
            .cloned()
            .collect())
    }

    async fn update_status(&self, id: i64, status: QuotationStatus) -> Result<Quotation, ApiError> {
        let mut inner = self.inner.lock();
        let quotation = inner
            .quotations
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Quotation {} not found", id)))?;
        quotation.status = status;
        quotation.updated_at = chrono::Utc::now();
        Ok(quotation.clone())
    }

    async fn link_to_order(&self, id: i64, order_id: i64) -> Result<Quotation, ApiError> {
        let mut inner = self.inner.lock();
        let quotation = inner
            .quotations
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Quotation {} not found", id)))?;
        quotation.sales_order_id = Some(order_id);
        quotation.status = QuotationStatus::ConvertedToOrder;
        quotation.updated_at = chrono::Utc::now();
        Ok(quotation.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.quotations.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryQuotationLineRepository
struct InMemoryQuotationLineInner {
    lines: std::collections::HashMap<i64, Vec<QuotationLine>>,
    next_id: i64,
}

/// In-memory quotation line repository
pub struct InMemoryQuotationLineRepository {
    inner: Mutex<InMemoryQuotationLineInner>,
}

impl InMemoryQuotationLineRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryQuotationLineInner {
                lines: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryQuotationLineRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QuotationLineRepository for InMemoryQuotationLineRepository {
    async fn create_many(
        &self,
        quotation_id: i64,
        create_lines: Vec<CreateQuotationLine>,
    ) -> Result<Vec<QuotationLine>, ApiError> {
        let mut inner = self.inner.lock();
        let mut lines = Vec::new();

        for (i, create) in create_lines.into_iter().enumerate() {
            let id = inner.next_id;
            inner.next_id += 1;

            let line_subtotal = create.quantity * create.unit_price;
            let line_discount = line_subtotal * (create.discount_rate / Decimal::ONE_HUNDRED);
            let after_discount = line_subtotal - line_discount;
            let line_tax = after_discount * (create.tax_rate / Decimal::ONE_HUNDRED);
            let line_total = after_discount + line_tax;

            lines.push(QuotationLine {
                id,
                quotation_id,
                product_id: create.product_id,
                description: create.description,
                quantity: create.quantity,
                unit_price: create.unit_price,
                tax_rate: create.tax_rate,
                discount_rate: create.discount_rate,
                line_total,
                sort_order: i as i32,
            });
        }

        inner.lines.insert(quotation_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_quotation(&self, quotation_id: i64) -> Result<Vec<QuotationLine>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.lines.get(&quotation_id).cloned().unwrap_or_default())
    }

    async fn delete_by_quotation(&self, quotation_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.lines.remove(&quotation_id);
        Ok(())
    }
}
