//! Invoice service for business logic
use crate::domain::invoice::model::{
    CreateInvoice, CreatePayment, Invoice, InvoiceResponse, InvoiceStatus, Payment,
};
use crate::domain::invoice::repository::{
    BoxInvoiceLineRepository, BoxInvoiceRepository, BoxPaymentRepository,
};
use crate::error::ApiError;

/// Invoice service
#[derive(Clone)]
pub struct InvoiceService {
    invoice_repo: BoxInvoiceRepository,
    line_repo: BoxInvoiceLineRepository,
    payment_repo: BoxPaymentRepository,
}

impl InvoiceService {
    pub fn new(
        invoice_repo: BoxInvoiceRepository,
        line_repo: BoxInvoiceLineRepository,
        payment_repo: BoxPaymentRepository,
    ) -> Self {
        Self {
            invoice_repo,
            line_repo,
            payment_repo,
        }
    }

    // Invoice operations
    pub async fn create_invoice(&self, create: CreateInvoice) -> Result<InvoiceResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Validate each line
        for line in &create.lines {
            line.validate()
                .map_err(|e| ApiError::Validation(e.join(", ")))?;
        }

        // Create invoice
        let invoice = self.invoice_repo.create(create.clone()).await?;

        // Create lines
        let lines = self.line_repo.create_many(invoice.id, create.lines).await?;

        Ok(InvoiceResponse::from((invoice, lines)))
    }

    pub async fn get_invoice(&self, id: i64) -> Result<InvoiceResponse, ApiError> {
        let invoice = self
            .invoice_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;

        let lines = self.line_repo.find_by_invoice(id).await?;

        Ok(InvoiceResponse::from((invoice, lines)))
    }

    pub async fn get_invoices_by_tenant(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError> {
        self.invoice_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_invoices_by_cari(&self, cari_id: i64) -> Result<Vec<Invoice>, ApiError> {
        self.invoice_repo.find_by_cari(cari_id).await
    }

    pub async fn get_invoices_by_status(
        &self,
        tenant_id: i64,
        status: InvoiceStatus,
    ) -> Result<Vec<Invoice>, ApiError> {
        self.invoice_repo.find_by_status(tenant_id, status).await
    }

    pub async fn update_invoice_status(
        &self,
        id: i64,
        status: InvoiceStatus,
    ) -> Result<Invoice, ApiError> {
        self.invoice_repo.update_status(id, status).await
    }

    pub async fn delete_invoice(&self, id: i64) -> Result<(), ApiError> {
        // Delete associated lines first
        self.line_repo.delete_by_invoice(id).await?;
        self.invoice_repo.delete(id).await
    }

    // Payment operations
    pub async fn create_payment(&self, create: CreatePayment) -> Result<Payment, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Verify invoice exists and is payable
        let invoice = self
            .invoice_repo
            .find_by_id(create.invoice_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Invoice {} not found", create.invoice_id))
            })?;

        if invoice.status == InvoiceStatus::Paid {
            return Err(ApiError::BadRequest(
                "Invoice is already fully paid".to_string(),
            ));
        }
        if invoice.status == InvoiceStatus::Cancelled {
            return Err(ApiError::BadRequest(
                "Cannot add payment to cancelled invoice".to_string(),
            ));
        }

        // Calculate new paid amount
        let new_paid = invoice.paid_amount + create.amount;
        if new_paid > invoice.total_amount {
            return Err(ApiError::BadRequest(format!(
                "Payment exceeds remaining amount. Remaining: {}",
                invoice.total_amount - invoice.paid_amount
            )));
        }

        // Create payment
        let payment = self.payment_repo.create(create).await?;

        // Update invoice paid amount
        self.invoice_repo
            .update_paid_amount(invoice.id, new_paid)
            .await?;

        Ok(payment)
    }

    pub async fn get_payments_by_invoice(&self, invoice_id: i64) -> Result<Vec<Payment>, ApiError> {
        self.payment_repo.find_by_invoice(invoice_id).await
    }

    // Utility methods
    pub async fn get_outstanding_invoices(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError> {
        let invoices = self.invoice_repo.find_by_tenant(tenant_id).await?;

        Ok(invoices
            .into_iter()
            .filter(|i| i.paid_amount < i.total_amount && i.status != InvoiceStatus::Cancelled)
            .collect())
    }

    pub async fn get_overdue_invoices(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError> {
        let invoices = self.invoice_repo.find_by_tenant(tenant_id).await?;
        let now = chrono::Utc::now();

        Ok(invoices
            .into_iter()
            .filter(|i| {
                i.due_date < now
                    && i.paid_amount < i.total_amount
                    && i.status != InvoiceStatus::Cancelled
                    && i.status != InvoiceStatus::Paid
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::invoice::model::{CreateInvoiceLine, InvoiceType};
    use crate::domain::invoice::repository::{
        InMemoryInvoiceLineRepository, InMemoryInvoiceRepository, InMemoryPaymentRepository,
    };
    use chrono::Utc;
    use std::sync::Arc;

    fn create_service() -> InvoiceService {
        let invoice_repo = Arc::new(InMemoryInvoiceRepository::new()) as BoxInvoiceRepository;
        let line_repo = Arc::new(InMemoryInvoiceLineRepository::new()) as BoxInvoiceLineRepository;
        let payment_repo = Arc::new(InMemoryPaymentRepository::new()) as BoxPaymentRepository;
        InvoiceService::new(invoice_repo, line_repo, payment_repo)
    }

    #[tokio::test]
    async fn test_create_invoice() {
        let service = create_service();
        let now = Utc::now();

        let create = CreateInvoice {
            tenant_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id: 1,
            issue_date: now,
            due_date: now + chrono::Duration::days(30),
            currency: "USD".to_string(),
            notes: None,
            lines: vec![CreateInvoiceLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: 2.0,
                unit_price: 100.0,
                tax_rate: 18.0,
                discount_rate: 0.0,
            }],
        };

        let result = service.create_invoice(create).await;
        assert!(result.is_ok());
        let invoice = result.unwrap();
        assert_eq!(invoice.lines.len(), 1);
        // 2 * 100 = 200, tax = 36, total = 236
        assert!((invoice.total_amount - 236.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_create_payment() {
        let service = create_service();
        let now = Utc::now();

        // Create invoice
        let create = CreateInvoice {
            tenant_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id: 1,
            issue_date: now,
            due_date: now + chrono::Duration::days(30),
            currency: "USD".to_string(),
            notes: None,
            lines: vec![CreateInvoiceLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: 1.0,
                unit_price: 100.0,
                tax_rate: 18.0,
                discount_rate: 0.0,
            }],
        };

        let invoice = service.create_invoice(create).await.unwrap();

        // Create payment
        let payment_create = CreatePayment {
            tenant_id: 1,
            invoice_id: invoice.id,
            amount: 50.0,
            payment_date: now,
            payment_method: "Bank Transfer".to_string(),
            reference_number: Some("TRF001".to_string()),
            notes: None,
        };

        let result = service.create_payment(payment_create).await;
        assert!(result.is_ok());

        // Check invoice status updated
        let updated = service.get_invoice(invoice.id).await.unwrap();
        assert_eq!(updated.status, InvoiceStatus::PartiallyPaid);
    }

    #[tokio::test]
    async fn test_full_payment() {
        let service = create_service();
        let now = Utc::now();

        let create = CreateInvoice {
            tenant_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id: 1,
            issue_date: now,
            due_date: now + chrono::Duration::days(30),
            currency: "USD".to_string(),
            notes: None,
            lines: vec![CreateInvoiceLine {
                product_id: Some(1),
                description: "Test".to_string(),
                quantity: 1.0,
                unit_price: 100.0,
                tax_rate: 18.0,
                discount_rate: 0.0,
            }],
        };

        let invoice = service.create_invoice(create).await.unwrap();
        let total = invoice.total_amount;

        let payment_create = CreatePayment {
            tenant_id: 1,
            invoice_id: invoice.id,
            amount: total,
            payment_date: now,
            payment_method: "Cash".to_string(),
            reference_number: None,
            notes: None,
        };

        service.create_payment(payment_create).await.unwrap();

        let updated = service.get_invoice(invoice.id).await.unwrap();
        assert_eq!(updated.status, InvoiceStatus::Paid);
    }
}
