//! Invoice repository

use async_trait::async_trait;
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::common::SoftDeletable;
use crate::domain::invoice::model::{
    CreateInvoice, CreatePayment, Invoice, InvoiceLine, InvoiceStatus, InvoiceType, Payment,
};
use crate::error::ApiError;

/// Repository trait for Invoice operations
#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn create(&self, invoice: CreateInvoice) -> Result<Invoice, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Invoice>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError>;
    async fn find_by_number(
        &self,
        tenant_id: i64,
        number: &str,
    ) -> Result<Option<Invoice>, ApiError>;
    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<Invoice>, ApiError>;
    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: InvoiceStatus,
    ) -> Result<Vec<Invoice>, ApiError>;

    /// Find invoices by tenant with pagination
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Invoice>, ApiError>;

    /// Find invoices by status with pagination
    async fn find_by_status_paginated(
        &self,
        tenant_id: i64,
        status: InvoiceStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Invoice>, ApiError>;

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: InvoiceStatus,
    ) -> Result<Invoice, ApiError>;
    async fn update_paid_amount(
        &self,
        id: i64,
        tenant_id: i64,
        paid_amount: Decimal,
    ) -> Result<Invoice, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Soft delete an invoice
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    /// Restore a soft-deleted invoice
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Invoice, ApiError>;

    /// Find soft-deleted invoices (admin use)
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError>;

    /// Search invoices by number or notes
    async fn search(&self, tenant_id: i64, query: &str) -> Result<Vec<Invoice>, ApiError>;

    /// Search invoices with pagination
    async fn search_paginated(
        &self,
        tenant_id: i64,
        query: &str,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Invoice>, ApiError>;

    /// Hard delete an invoice (permanent destruction — admin only)
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for InvoiceLine operations.
/// Note: InvoiceLine does not have a tenant_id field; it is a child entity of Invoice.
/// Tenant isolation should be enforced by looking up the parent Invoice first.
#[async_trait]
pub trait InvoiceLineRepository: Send + Sync {
    async fn create_many(
        &self,
        invoice_id: i64,
        lines: Vec<crate::domain::invoice::model::CreateInvoiceLine>,
    ) -> Result<Vec<InvoiceLine>, ApiError>;
    async fn find_by_invoice(&self, invoice_id: i64) -> Result<Vec<InvoiceLine>, ApiError>;
    async fn delete_by_invoice(&self, invoice_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for Payment operations.
/// Note: Payment has tenant_id; all single-record lookups must enforce tenant isolation.
#[async_trait]
pub trait PaymentRepository: Send + Sync {
    async fn create(&self, payment: CreatePayment) -> Result<Payment, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Payment>, ApiError>;
    async fn find_by_invoice(&self, invoice_id: i64) -> Result<Vec<Payment>, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type aliases
pub type BoxInvoiceRepository = Arc<dyn InvoiceRepository>;
pub type BoxInvoiceLineRepository = Arc<dyn InvoiceLineRepository>;
pub type BoxPaymentRepository = Arc<dyn PaymentRepository>;

fn generate_invoice_number(invoice_type: &InvoiceType, count: i64) -> String {
    let prefix = match invoice_type {
        InvoiceType::SalesInvoice => "SI",
        InvoiceType::PurchaseInvoice => "PI",
        InvoiceType::SalesReturn => "SR",
        InvoiceType::PurchaseReturn => "PR",
    };
    format!("{}-{:06}", prefix, count)
}

/// Inner state for InMemoryInvoiceRepository
struct InMemoryInvoiceInner {
    invoices: std::collections::HashMap<i64, Invoice>,
    next_id: i64,
    tenant_invoices: std::collections::HashMap<i64, Vec<i64>>,
    cari_invoices: std::collections::HashMap<i64, Vec<i64>>,
}

/// In-memory invoice repository
pub struct InMemoryInvoiceRepository {
    inner: Mutex<InMemoryInvoiceInner>,
}

impl InMemoryInvoiceRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryInvoiceInner {
                invoices: std::collections::HashMap::new(),
                next_id: 1,
                tenant_invoices: std::collections::HashMap::new(),
                cari_invoices: std::collections::HashMap::new(),
            }),
        }
    }
}

impl Default for InMemoryInvoiceRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InvoiceRepository for InMemoryInvoiceRepository {
    async fn create(&self, create: CreateInvoice) -> Result<Invoice, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let invoice_number = generate_invoice_number(&create.invoice_type, id);

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

        let invoice = Invoice {
            id,
            tenant_id: create.tenant_id,
            invoice_number,
            invoice_type: create.invoice_type,
            status: InvoiceStatus::Draft,
            cari_id: create.cari_id,
            issue_date: create.issue_date,
            due_date: create.due_date,
            subtotal,
            tax_amount,
            discount_amount,
            total_amount,
            paid_amount: Decimal::ZERO,
            currency: create.currency,
            exchange_rate: create.exchange_rate,
            notes: create.notes,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        inner.invoices.insert(id, invoice.clone());
        inner
            .tenant_invoices
            .entry(create.tenant_id)
            .or_default()
            .push(id);
        inner
            .cari_invoices
            .entry(create.cari_id)
            .or_default()
            .push(id);

        Ok(invoice)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Invoice>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .invoices
            .get(&id)
            .filter(|i| i.tenant_id == tenant_id && !i.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError> {
        let inner = self.inner.lock();
        let ids = inner
            .tenant_invoices
            .get(&tenant_id)
            .cloned()
            .unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| inner.invoices.get(id).cloned())
            .filter(|i| !i.is_deleted())
            .collect())
    }

    async fn find_by_number(
        &self,
        tenant_id: i64,
        number: &str,
    ) -> Result<Option<Invoice>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .invoices
            .values()
            .find(|i| i.tenant_id == tenant_id && i.invoice_number == number && !i.is_deleted())
            .cloned())
    }

    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<Invoice>, ApiError> {
        let inner = self.inner.lock();
        let ids = inner
            .cari_invoices
            .get(&cari_id)
            .cloned()
            .unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| inner.invoices.get(id).cloned())
            .filter(|i| !i.is_deleted())
            .collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: InvoiceStatus,
    ) -> Result<Vec<Invoice>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .invoices
            .values()
            .filter(|i| i.tenant_id == tenant_id && i.status == status && !i.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Invoice>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .invoices
            .values()
            .filter(|i| i.tenant_id == tenant_id && !i.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
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
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .invoices
            .values()
            .filter(|i| i.tenant_id == tenant_id && i.status == status && !i.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: InvoiceStatus,
    ) -> Result<Invoice, ApiError> {
        let mut inner = self.inner.lock();
        let invoice = inner
            .invoices
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;
        if invoice.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Invoice {} not found", id)));
        }
        invoice.status = status;
        invoice.updated_at = chrono::Utc::now();
        Ok(invoice.clone())
    }

    async fn update_paid_amount(
        &self,
        id: i64,
        tenant_id: i64,
        paid_amount: Decimal,
    ) -> Result<Invoice, ApiError> {
        let mut inner = self.inner.lock();
        let invoice = inner
            .invoices
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;
        if invoice.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Invoice {} not found", id)));
        }

        invoice.paid_amount = paid_amount;

        // Update status based on paid amount
        invoice.status = if paid_amount >= invoice.total_amount {
            InvoiceStatus::Paid
        } else if paid_amount > Decimal::ZERO {
            InvoiceStatus::PartiallyPaid
        } else if invoice.status == InvoiceStatus::Paid {
            InvoiceStatus::Approved
        } else {
            invoice.status.clone()
        };

        invoice.updated_at = chrono::Utc::now();
        Ok(invoice.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let invoice = {
            let inner = self.inner.lock();
            inner.invoices.get(&id).cloned()
        }
        .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;

        if invoice.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Invoice {} not found", id)));
        }

        let mut inner = self.inner.lock();
        inner.invoices.remove(&id);
        inner
            .tenant_invoices
            .entry(invoice.tenant_id)
            .and_modify(|v| {
                v.retain(|&x| x != id);
            });
        inner.cari_invoices.entry(invoice.cari_id).and_modify(|v| {
            v.retain(|&x| x != id);
        });
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let invoice = inner
            .invoices
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;

        if invoice.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Invoice {} not found", id)));
        }

        invoice.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Invoice, ApiError> {
        let mut inner = self.inner.lock();

        let invoice = inner
            .invoices
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;

        if invoice.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Invoice {} not found", id)));
        }

        invoice.restore();
        Ok(invoice.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .invoices
            .values()
            .filter(|i| i.tenant_id == tenant_id && i.is_deleted())
            .cloned()
            .collect())
    }

    async fn search(&self, tenant_id: i64, query: &str) -> Result<Vec<Invoice>, ApiError> {
        let inner = self.inner.lock();
        let query_lower = query.to_lowercase();
        Ok(inner
            .invoices
            .values()
            .filter(|i| {
                i.tenant_id == tenant_id
                    && !i.is_deleted()
                    && (i.invoice_number.to_lowercase().contains(&query_lower)
                        || i.notes
                            .as_ref()
                            .map(|n| n.to_lowercase().contains(&query_lower))
                            .unwrap_or(false))
            })
            .cloned()
            .collect())
    }

    async fn search_paginated(
        &self,
        tenant_id: i64,
        query: &str,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Invoice>, ApiError> {
        let inner = self.inner.lock();
        let query_lower = query.to_lowercase();
        let all: Vec<_> = inner
            .invoices
            .values()
            .filter(|i| {
                i.tenant_id == tenant_id
                    && !i.is_deleted()
                    && (i.invoice_number.to_lowercase().contains(&query_lower)
                        || i.notes
                            .as_ref()
                            .map(|n| n.to_lowercase().contains(&query_lower))
                            .unwrap_or(false))
            })
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let invoice = {
            let inner = self.inner.lock();
            inner.invoices.get(&id).cloned()
        }
        .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;

        if invoice.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Invoice {} not found", id)));
        }

        let mut inner = self.inner.lock();
        inner.invoices.remove(&id);
        inner
            .tenant_invoices
            .entry(invoice.tenant_id)
            .and_modify(|v| {
                v.retain(|&x| x != id);
            });
        inner.cari_invoices.entry(invoice.cari_id).and_modify(|v| {
            v.retain(|&x| x != id);
        });
        Ok(())
    }
}

/// Inner state for InMemoryInvoiceLineRepository
struct InMemoryInvoiceLineInner {
    lines: std::collections::HashMap<i64, Vec<InvoiceLine>>,
    next_id: i64,
}

/// In-memory invoice line repository
pub struct InMemoryInvoiceLineRepository {
    inner: Mutex<InMemoryInvoiceLineInner>,
}

impl InMemoryInvoiceLineRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryInvoiceLineInner {
                lines: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryInvoiceLineRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InvoiceLineRepository for InMemoryInvoiceLineRepository {
    async fn create_many(
        &self,
        invoice_id: i64,
        create_lines: Vec<crate::domain::invoice::model::CreateInvoiceLine>,
    ) -> Result<Vec<InvoiceLine>, ApiError> {
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

            lines.push(InvoiceLine {
                id,
                invoice_id,
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

        inner.lines.insert(invoice_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_invoice(&self, invoice_id: i64) -> Result<Vec<InvoiceLine>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.lines.get(&invoice_id).cloned().unwrap_or_default())
    }

    async fn delete_by_invoice(&self, invoice_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.lines.remove(&invoice_id);
        Ok(())
    }
}

/// Inner state for InMemoryPaymentRepository
struct InMemoryPaymentInner {
    payments: std::collections::HashMap<i64, Payment>,
    next_id: i64,
    invoice_payments: std::collections::HashMap<i64, Vec<i64>>,
}

/// In-memory payment repository
pub struct InMemoryPaymentRepository {
    inner: Mutex<InMemoryPaymentInner>,
}

impl InMemoryPaymentRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryPaymentInner {
                payments: std::collections::HashMap::new(),
                next_id: 1,
                invoice_payments: std::collections::HashMap::new(),
            }),
        }
    }
}

impl Default for InMemoryPaymentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PaymentRepository for InMemoryPaymentRepository {
    async fn create(&self, create: CreatePayment) -> Result<Payment, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let payment = Payment {
            id,
            tenant_id: create.tenant_id,
            invoice_id: create.invoice_id,
            amount: create.amount,
            currency: create.currency,
            payment_date: create.payment_date,
            payment_method: create.payment_method,
            reference_number: create.reference_number,
            notes: create.notes,
            created_at: chrono::Utc::now(),
        };

        inner.payments.insert(id, payment.clone());
        inner
            .invoice_payments
            .entry(create.invoice_id)
            .or_default()
            .push(id);

        Ok(payment)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Payment>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .payments
            .get(&id)
            .filter(|p| p.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_invoice(&self, invoice_id: i64) -> Result<Vec<Payment>, ApiError> {
        let inner = self.inner.lock();
        let ids = inner
            .invoice_payments
            .get(&invoice_id)
            .cloned()
            .unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| inner.payments.get(id).cloned())
            .collect())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let payment = inner
            .payments
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Payment {} not found", id)))?;

        if payment.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Payment {} not found", id)));
        }

        inner.payments.remove(&id);
        Ok(())
    }
}
