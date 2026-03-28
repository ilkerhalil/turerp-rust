//! Invoice repository

use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::invoice::model::{
    CreateInvoice, CreatePayment, Invoice, InvoiceLine, InvoiceStatus, InvoiceType, Payment,
};
use crate::error::ApiError;

/// Repository trait for Invoice operations
#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn create(&self, invoice: CreateInvoice) -> Result<Invoice, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Invoice>, ApiError>;
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
    async fn update_status(&self, id: i64, status: InvoiceStatus) -> Result<Invoice, ApiError>;
    async fn update_paid_amount(&self, id: i64, paid_amount: f64) -> Result<Invoice, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for InvoiceLine operations
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

/// Repository trait for Payment operations
#[async_trait]
pub trait PaymentRepository: Send + Sync {
    async fn create(&self, payment: CreatePayment) -> Result<Payment, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Payment>, ApiError>;
    async fn find_by_invoice(&self, invoice_id: i64) -> Result<Vec<Payment>, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Type aliases
pub type BoxInvoiceRepository = Arc<dyn InvoiceRepository>;
pub type BoxInvoiceLineRepository = Arc<dyn InvoiceLineRepository>;
pub type BoxPaymentRepository = Arc<dyn PaymentRepository>;

/// In-memory invoice repository
pub struct InMemoryInvoiceRepository {
    invoices: std::sync::Mutex<std::collections::HashMap<i64, Invoice>>,
    next_id: std::sync::Mutex<i64>,
    tenant_invoices: std::sync::Mutex<std::collections::HashMap<i64, Vec<i64>>>,
    cari_invoices: std::sync::Mutex<std::collections::HashMap<i64, Vec<i64>>>,
}

impl InMemoryInvoiceRepository {
    pub fn new() -> Self {
        Self {
            invoices: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
            tenant_invoices: std::sync::Mutex::new(std::collections::HashMap::new()),
            cari_invoices: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for InMemoryInvoiceRepository {
    fn default() -> Self {
        Self::new()
    }
}

fn generate_invoice_number(invoice_type: &InvoiceType, count: i64) -> String {
    let prefix = match invoice_type {
        InvoiceType::SalesInvoice => "SI",
        InvoiceType::PurchaseInvoice => "PI",
        InvoiceType::SalesReturn => "SR",
        InvoiceType::PurchaseReturn => "PR",
    };
    format!("{}-{:06}", prefix, count)
}

#[async_trait]
impl InvoiceRepository for InMemoryInvoiceRepository {
    async fn create(&self, create: CreateInvoice) -> Result<Invoice, ApiError> {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let invoice_number = generate_invoice_number(&create.invoice_type, id);

        let mut subtotal = 0.0;
        let mut tax_amount = 0.0;
        let mut discount_amount = 0.0;

        for line in &create.lines {
            let line_subtotal = line.quantity * line.unit_price;
            let line_discount = line_subtotal * (line.discount_rate / 100.0);
            let after_discount = line_subtotal - line_discount;
            let line_tax = after_discount * (line.tax_rate / 100.0);

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
            paid_amount: 0.0,
            currency: create.currency,
            notes: create.notes,
            created_at: now,
            updated_at: now,
        };

        self.invoices.lock().unwrap().insert(id, invoice.clone());
        self.tenant_invoices
            .lock()
            .unwrap()
            .entry(create.tenant_id)
            .or_default()
            .push(id);
        self.cari_invoices
            .lock()
            .unwrap()
            .entry(create.cari_id)
            .or_default()
            .push(id);

        Ok(invoice)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Invoice>, ApiError> {
        Ok(self.invoices.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError> {
        let tenant_invoices = self.tenant_invoices.lock().unwrap();
        let invoices = self.invoices.lock().unwrap();
        let ids = tenant_invoices.get(&tenant_id).cloned().unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| invoices.get(id).cloned())
            .collect())
    }

    async fn find_by_number(
        &self,
        tenant_id: i64,
        number: &str,
    ) -> Result<Option<Invoice>, ApiError> {
        let invoices = self.invoices.lock().unwrap();
        Ok(invoices
            .values()
            .find(|i| i.tenant_id == tenant_id && i.invoice_number == number)
            .cloned())
    }

    async fn find_by_cari(&self, cari_id: i64) -> Result<Vec<Invoice>, ApiError> {
        let cari_invoices = self.cari_invoices.lock().unwrap();
        let invoices = self.invoices.lock().unwrap();
        let ids = cari_invoices.get(&cari_id).cloned().unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| invoices.get(id).cloned())
            .collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: InvoiceStatus,
    ) -> Result<Vec<Invoice>, ApiError> {
        let invoices = self.invoices.lock().unwrap();
        Ok(invoices
            .values()
            .filter(|i| i.tenant_id == tenant_id && i.status == status)
            .cloned()
            .collect())
    }

    async fn update_status(&self, id: i64, status: InvoiceStatus) -> Result<Invoice, ApiError> {
        let mut invoices = self.invoices.lock().unwrap();
        let invoice = invoices
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;
        invoice.status = status;
        invoice.updated_at = chrono::Utc::now();
        Ok(invoice.clone())
    }

    async fn update_paid_amount(&self, id: i64, paid_amount: f64) -> Result<Invoice, ApiError> {
        let mut invoices = self.invoices.lock().unwrap();
        let invoice = invoices
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;

        invoice.paid_amount = paid_amount;

        // Update status based on paid amount
        invoice.status = if paid_amount >= invoice.total_amount {
            InvoiceStatus::Paid
        } else if paid_amount > 0.0 {
            InvoiceStatus::PartiallyPaid
        } else if invoice.status == InvoiceStatus::Paid {
            InvoiceStatus::Approved
        } else {
            invoice.status.clone()
        };

        invoice.updated_at = chrono::Utc::now();
        Ok(invoice.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.invoices.lock().unwrap().remove(&id);
        Ok(())
    }
}

/// In-memory invoice line repository
pub struct InMemoryInvoiceLineRepository {
    lines: std::sync::Mutex<std::collections::HashMap<i64, Vec<InvoiceLine>>>,
    next_id: std::sync::Mutex<i64>,
}

impl InMemoryInvoiceLineRepository {
    pub fn new() -> Self {
        Self {
            lines: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
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
        let mut next_id = self.next_id.lock().unwrap();
        let mut lines = Vec::new();

        for (i, create) in create_lines.into_iter().enumerate() {
            let id = *next_id;
            *next_id += 1;

            let line_subtotal = create.quantity * create.unit_price;
            let line_discount = line_subtotal * (create.discount_rate / 100.0);
            let after_discount = line_subtotal - line_discount;
            let line_tax = after_discount * (create.tax_rate / 100.0);
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

        self.lines.lock().unwrap().insert(invoice_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_invoice(&self, invoice_id: i64) -> Result<Vec<InvoiceLine>, ApiError> {
        Ok(self
            .lines
            .lock()
            .unwrap()
            .get(&invoice_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn delete_by_invoice(&self, invoice_id: i64) -> Result<(), ApiError> {
        self.lines.lock().unwrap().remove(&invoice_id);
        Ok(())
    }
}

/// In-memory payment repository
pub struct InMemoryPaymentRepository {
    payments: std::sync::Mutex<std::collections::HashMap<i64, Payment>>,
    next_id: std::sync::Mutex<i64>,
    invoice_payments: std::sync::Mutex<std::collections::HashMap<i64, Vec<i64>>>,
}

impl InMemoryPaymentRepository {
    pub fn new() -> Self {
        Self {
            payments: std::sync::Mutex::new(std::collections::HashMap::new()),
            next_id: std::sync::Mutex::new(1),
            invoice_payments: std::sync::Mutex::new(std::collections::HashMap::new()),
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

        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let payment = Payment {
            id,
            tenant_id: create.tenant_id,
            invoice_id: create.invoice_id,
            amount: create.amount,
            payment_date: create.payment_date,
            payment_method: create.payment_method,
            reference_number: create.reference_number,
            notes: create.notes,
            created_at: chrono::Utc::now(),
        };

        self.payments.lock().unwrap().insert(id, payment.clone());
        self.invoice_payments
            .lock()
            .unwrap()
            .entry(create.invoice_id)
            .or_default()
            .push(id);

        Ok(payment)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Payment>, ApiError> {
        Ok(self.payments.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_invoice(&self, invoice_id: i64) -> Result<Vec<Payment>, ApiError> {
        let invoice_payments = self.invoice_payments.lock().unwrap();
        let payments = self.payments.lock().unwrap();
        let ids = invoice_payments
            .get(&invoice_id)
            .cloned()
            .unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| payments.get(id).cloned())
            .collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        self.payments.lock().unwrap().remove(&id);
        Ok(())
    }
}
