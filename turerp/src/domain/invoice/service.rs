//! Invoice service for business logic
use std::collections::HashSet;

use crate::common::pagination::PaginatedResult;
use crate::domain::cari::repository::BoxCariRepository;
use crate::domain::cost_center::repository::BoxCostCenterRepository;
use crate::domain::invoice::model::{
    CreateInvoice, CreatePayment, Invoice, InvoiceResponse, InvoiceStatus, Payment,
};
use crate::domain::invoice::repository::{
    BoxInvoiceLineRepository, BoxInvoiceRepository, BoxPaymentRepository,
};
use crate::domain::product::repository::BoxProductRepository;
use crate::error::ApiError;
use tracing;

/// Invoice service
#[derive(Clone)]
pub struct InvoiceService {
    invoice_repo: BoxInvoiceRepository,
    line_repo: BoxInvoiceLineRepository,
    payment_repo: BoxPaymentRepository,
    cari_repo: BoxCariRepository,
    cost_center_repo: BoxCostCenterRepository,
    product_repo: BoxProductRepository,
}

impl InvoiceService {
    pub fn new(
        invoice_repo: BoxInvoiceRepository,
        line_repo: BoxInvoiceLineRepository,
        payment_repo: BoxPaymentRepository,
        cari_repo: BoxCariRepository,
        cost_center_repo: BoxCostCenterRepository,
        product_repo: BoxProductRepository,
    ) -> Self {
        Self {
            invoice_repo,
            line_repo,
            payment_repo,
            cari_repo,
            cost_center_repo,
            product_repo,
        }
    }

    /// Parent-ownership precheck: the invoice's `cari_id` must belong to the
    /// caller's tenant. Mirrors the established pattern (assets/manufacturing/
    /// sales/purchase) — returns `NotFound` if the cari is foreign/missing/
    /// soft-deleted, BEFORE any repo INSERT.
    async fn ensure_cari_owned(&self, cari_id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.cari_repo
            .find_by_id(cari_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Cari {} not found", cari_id)))?;
        Ok(())
    }

    /// Parent-ownership precheck for the optional `cost_center_id`. `None` is a
    /// legitimate "no cost center" value and is never rejected.
    async fn ensure_cost_center_owned(
        &self,
        cost_center_id: Option<i64>,
        tenant_id: i64,
    ) -> Result<(), ApiError> {
        if let Some(cc_id) = cost_center_id {
            self.cost_center_repo
                .find_by_id(cc_id, tenant_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("Cost center {} not found", cc_id)))?;
        }
        Ok(())
    }

    /// Parent-ownership precheck for per-line `product_id` (batched). Collects
    /// every `Some(product_id)`, dedups, and issues a single tenant-scoped
    /// `find_by_ids`; a foreign/missing product is simply not returned → count
    /// mismatch → `NotFound`. `None` lines are legitimate no-product lines and
    /// are skipped.
    async fn ensure_line_products_owned(
        &self,
        lines: &[crate::domain::invoice::model::CreateInvoiceLine],
        tenant_id: i64,
    ) -> Result<(), ApiError> {
        let mut set: HashSet<i64> = lines.iter().filter_map(|l| l.product_id).collect();
        if set.is_empty() {
            return Ok(());
        }
        let mut ids: Vec<i64> = set.drain().collect();
        ids.sort_unstable();
        let found = self.product_repo.find_by_ids(&ids, tenant_id).await?;
        if found.len() != ids.len() {
            let found_ids: HashSet<i64> = found.iter().map(|p| p.id).collect();
            let missing = ids.iter().find(|id| !found_ids.contains(id));
            return Err(ApiError::NotFound(format!(
                "Product {} not found",
                missing.unwrap_or(&-1)
            )));
        }
        Ok(())
    }

    // Invoice operations
    #[tracing::instrument(skip(self, create), fields(tenant_id = create.tenant_id))]
    pub async fn create_invoice(&self, create: CreateInvoice) -> Result<InvoiceResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Validate each line
        for line in &create.lines {
            line.validate()
                .map_err(|e| ApiError::Validation(e.join(", ")))?;
        }

        // Parent-ownership prechecks: the body-controlled cari_id, cost_center_id,
        // and per-line product_id must belong to the caller's tenant before the
        // INSERT — closes the cross-tenant IDOR (a tenant-A admin referencing a
        // tenant-B cari/cost center/product). tenant_id is the auth-overwritten
        // value (handler sets create.tenant_id = auth_user.0.tenant_id).
        self.ensure_cari_owned(create.cari_id, create.tenant_id)
            .await?;
        self.ensure_cost_center_owned(create.cost_center_id, create.tenant_id)
            .await?;
        self.ensure_line_products_owned(&create.lines, create.tenant_id)
            .await?;

        // Create invoice
        let invoice = self.invoice_repo.create(create.clone()).await?;

        // Create lines — if this fails, roll back the orphan invoice
        let lines = match self
            .line_repo
            .create_many(invoice.id, create.lines, create.tenant_id)
            .await
        {
            Ok(lines) => lines,
            Err(e) => {
                if let Err(rollback_err) =
                    self.invoice_repo.delete(invoice.id, create.tenant_id).await
                {
                    tracing::warn!(error = %rollback_err, "Failed to roll back orphan invoice {} after line creation failed", invoice.id);
                }
                return Err(e);
            }
        };

        Ok(InvoiceResponse::from((invoice, lines)))
    }

    pub async fn get_invoice(&self, id: i64, tenant_id: i64) -> Result<InvoiceResponse, ApiError> {
        let invoice = self
            .invoice_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;

        let lines = self.line_repo.find_by_invoice(id, tenant_id).await?;

        Ok(InvoiceResponse::from((invoice, lines)))
    }

    /// Get invoice lines by invoice ID (used internally, e.g. for restore response)
    pub async fn get_invoice_lines(
        &self,
        invoice_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<crate::domain::invoice::model::InvoiceLine>, ApiError> {
        self.line_repo.find_by_invoice(invoice_id, tenant_id).await
    }

    pub async fn get_invoices_by_tenant(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError> {
        self.invoice_repo.find_by_tenant(tenant_id, 1000, 0).await
    }

    pub async fn get_invoices_by_cari(
        &self,
        tenant_id: i64,
        cari_id: i64,
    ) -> Result<Vec<Invoice>, ApiError> {
        self.invoice_repo.find_by_cari(tenant_id, cari_id).await
    }

    pub async fn get_invoices_by_status(
        &self,
        tenant_id: i64,
        status: InvoiceStatus,
    ) -> Result<Vec<Invoice>, ApiError> {
        self.invoice_repo.find_by_status(tenant_id, status).await
    }

    /// Get invoices by tenant with pagination
    pub async fn get_invoices_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Invoice>, ApiError> {
        crate::common::pagination::PaginationParams { page, per_page }
            .validate()
            .map_err(ApiError::Validation)?;
        self.invoice_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    /// Get invoices by status with pagination
    pub async fn get_invoices_by_status_paginated(
        &self,
        tenant_id: i64,
        status: InvoiceStatus,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Invoice>, ApiError> {
        crate::common::pagination::PaginationParams { page, per_page }
            .validate()
            .map_err(ApiError::Validation)?;
        self.invoice_repo
            .find_by_status_paginated(tenant_id, status, page, per_page)
            .await
    }

    pub async fn update_invoice_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: InvoiceStatus,
    ) -> Result<Invoice, ApiError> {
        self.invoice_repo.update_status(id, tenant_id, status).await
    }

    #[tracing::instrument(skip(self), fields(tenant_id = tenant_id))]
    pub async fn delete_invoice(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        // Delete associated lines first (tenant-scoped)
        self.line_repo.delete_by_invoice(id, tenant_id).await?;
        self.invoice_repo.delete(id, tenant_id).await
    }

    /// Soft delete an invoice (admin only)
    pub async fn soft_delete_invoice(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.invoice_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    /// Restore a soft-deleted invoice (admin only)
    pub async fn restore_invoice(&self, id: i64, tenant_id: i64) -> Result<Invoice, ApiError> {
        self.invoice_repo.restore(id, tenant_id).await
    }

    /// List soft-deleted invoices (admin only)
    pub async fn list_deleted_invoices(&self, tenant_id: i64) -> Result<Vec<Invoice>, ApiError> {
        self.invoice_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete an invoice (admin only, after soft delete)
    pub async fn destroy_invoice(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        // Rely on the tenant-scoped invoice_repo.destroy (returns NotFound for a
        // foreign invoice, Conflict if payments still exist). invoice_lines has
        // ON DELETE CASCADE on invoice_id, so deleting the invoice removes its
        // lines atomically. This deliberately does NOT call line_repo
        // .delete_by_invoice first: that previously ran an unscoped
        // `DELETE FROM invoice_lines WHERE invoice_id = $1` BEFORE the parent's
        // tenant was validated — a cross-tenant hard-delete leak (a tenant-A
        // admin could wipe tenant-B's invoice lines by guessing an invoice id,
        // then destroy returned NotFound with no rollback). It also avoids a
        // partial-state bug where lines were hard-deleted even when destroy
        // later returned Conflict on existing payments.
        self.invoice_repo.destroy(id, tenant_id).await
    }

    // Payment operations
    pub async fn create_payment(&self, create: CreatePayment) -> Result<Payment, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Verify invoice exists and is payable (with tenant check)
        let invoice = self
            .invoice_repo
            .find_by_id(create.invoice_id, create.tenant_id)
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

        // Update invoice paid amount — if this fails, roll back the payment
        if let Err(e) = self
            .invoice_repo
            .update_paid_amount(invoice.id, invoice.tenant_id, new_paid)
            .await
        {
            if let Err(rollback_err) = self
                .payment_repo
                .delete(payment.id, payment.tenant_id)
                .await
            {
                tracing::warn!(error = %rollback_err, "Failed to roll back payment {} after paid amount update failed", payment.id);
            }
            return Err(e);
        }

        Ok(payment)
    }

    pub async fn get_payments_by_invoice(
        &self,
        invoice_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Payment>, ApiError> {
        self.payment_repo
            .find_by_invoice(invoice_id, tenant_id)
            .await
    }

    /// Get all payments for a specific cari (customer)
    #[tracing::instrument(skip(self), fields(tenant_id = tenant_id))]
    pub async fn get_payments_by_cari(
        &self,
        tenant_id: i64,
        cari_id: i64,
    ) -> Result<Vec<Payment>, ApiError> {
        let invoices = self.invoice_repo.find_by_cari(tenant_id, cari_id).await?;
        let invoice_ids: Vec<i64> = invoices.into_iter().map(|i| i.id).collect();
        if invoice_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.payment_repo
            .find_by_invoices(&invoice_ids, tenant_id)
            .await
    }

    // Utility methods
    /// Search invoices by number or notes (full-text)
    #[tracing::instrument(skip(self), fields(tenant_id = tenant_id))]
    pub async fn search_invoices(
        &self,
        tenant_id: i64,
        query: &str,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<Invoice>, ApiError> {
        let limit = (per_page as i64).min(100);
        let offset = ((page.saturating_sub(1)) * per_page) as i64;
        self.invoice_repo
            .search(tenant_id, query, limit, offset)
            .await
    }

    /// Search invoices by number or notes with pagination
    pub async fn search_invoices_paginated(
        &self,
        tenant_id: i64,
        query: &str,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Invoice>, ApiError> {
        crate::common::pagination::PaginationParams { page, per_page }
            .validate()
            .map_err(ApiError::Validation)?;
        self.invoice_repo
            .search_paginated(tenant_id, query, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self), fields(tenant_id = tenant_id))]
    pub async fn get_outstanding_invoices(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<Invoice>, ApiError> {
        let limit = per_page as i64;
        let offset = ((page.saturating_sub(1)) * per_page) as i64;
        self.invoice_repo
            .find_outstanding(tenant_id, limit, offset)
            .await
    }

    #[tracing::instrument(skip(self), fields(tenant_id = tenant_id))]
    pub async fn get_overdue_invoices(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<Invoice>, ApiError> {
        let limit = per_page as i64;
        let offset = ((page.saturating_sub(1)) * per_page) as i64;
        self.invoice_repo
            .find_overdue(tenant_id, limit, offset)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cari::model::CreateCari;
    use crate::domain::cari::repository::InMemoryCariRepository;
    use crate::domain::cost_center::model::{CostCenterType, CreateCostCenter};
    use crate::domain::cost_center::repository::InMemoryCostCenterRepository;
    use crate::domain::invoice::model::{CreateInvoiceLine, InvoiceType};
    use crate::domain::invoice::repository::{
        InMemoryInvoiceLineRepository, InMemoryInvoiceRepository, InMemoryPaymentRepository,
    };
    use crate::domain::product::model::CreateProduct;
    use crate::domain::product::repository::InMemoryProductRepository;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    async fn create_service() -> InvoiceService {
        let invoice_repo = Arc::new(InMemoryInvoiceRepository::new()) as BoxInvoiceRepository;
        let line_repo = Arc::new(InMemoryInvoiceLineRepository::new()) as BoxInvoiceLineRepository;
        let payment_repo = Arc::new(InMemoryPaymentRepository::new()) as BoxPaymentRepository;

        // Seed the parent entities the create-invoice prechecks validate against.
        // InMemory repo create() auto-assigns ids starting at 1, matching the
        // cari_id=1 / product_id=Some(1) / tenant_id=1 used by the happy-path
        // tests below. A second cari/product/cost_center is seeded on tenant 2
        // (ids 2) so the cross-tenant IDOR rejection tests can reference a
        // foreign parent.
        let cari_repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
        cari_repo
            .create(CreateCari {
                code: "C1".to_string(),
                name: "Test Cari T1".to_string(),
                tenant_id: 1,
                created_by: 1,
                ..Default::default()
            })
            .await
            .expect("seed cari t1");
        cari_repo
            .create(CreateCari {
                code: "C2".to_string(),
                name: "Test Cari T2".to_string(),
                tenant_id: 2,
                created_by: 1,
                ..Default::default()
            })
            .await
            .expect("seed cari t2");

        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        product_repo
            .create(CreateProduct {
                tenant_id: 1,
                code: "P1".to_string(),
                name: "Test Product T1".to_string(),
                purchase_price: Decimal::ZERO,
                sale_price: Decimal::ZERO,
                tax_rate: Decimal::ZERO,
                ..Default::default()
            })
            .await
            .expect("seed product t1");
        product_repo
            .create(CreateProduct {
                tenant_id: 2,
                code: "P2".to_string(),
                name: "Test Product T2".to_string(),
                purchase_price: Decimal::ZERO,
                sale_price: Decimal::ZERO,
                tax_rate: Decimal::ZERO,
                ..Default::default()
            })
            .await
            .expect("seed product t2");

        let cost_center_repo =
            Arc::new(InMemoryCostCenterRepository::new()) as BoxCostCenterRepository;
        cost_center_repo
            .create(
                CreateCostCenter {
                    code: "CC1".to_string(),
                    name: "Test Cost Center T1".to_string(),
                    description: None,
                    center_type: CostCenterType::Cost,
                    parent_id: None,
                    is_active: true,
                },
                1,
            )
            .await
            .expect("seed cost center t1");
        cost_center_repo
            .create(
                CreateCostCenter {
                    code: "CC2".to_string(),
                    name: "Test Cost Center T2".to_string(),
                    description: None,
                    center_type: CostCenterType::Cost,
                    parent_id: None,
                    is_active: true,
                },
                2,
            )
            .await
            .expect("seed cost center t2");

        InvoiceService::new(
            invoice_repo,
            line_repo,
            payment_repo,
            cari_repo,
            cost_center_repo,
            product_repo,
        )
    }

    #[tokio::test]
    async fn test_create_invoice() {
        let service = create_service().await;
        let now = Utc::now();

        let create = CreateInvoice {
            tenant_id: 1,
            company_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id: 1,
            issue_date: now,
            due_date: now + chrono::Duration::days(30),
            currency: "USD".to_string(),
            exchange_rate: Decimal::ONE,
            notes: None,
            cost_center_id: None,
            lines: vec![CreateInvoiceLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: dec!(2),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };

        let result = service.create_invoice(create).await;
        assert!(result.is_ok());
        let invoice = result.unwrap();
        assert_eq!(invoice.lines.len(), 1);
        // 2 * 100 = 200, tax = 36, total = 236
        assert_eq!(invoice.total_amount, dec!(236));
    }

    #[tokio::test]
    async fn test_create_payment() {
        let service = create_service().await;
        let now = Utc::now();

        // Create invoice
        let create = CreateInvoice {
            tenant_id: 1,
            company_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id: 1,
            issue_date: now,
            due_date: now + chrono::Duration::days(30),
            currency: "USD".to_string(),
            exchange_rate: Decimal::ONE,
            notes: None,
            cost_center_id: None,
            lines: vec![CreateInvoiceLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: dec!(1),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };

        let invoice = service.create_invoice(create).await.unwrap();

        // Create payment
        let payment_create = CreatePayment {
            tenant_id: 1,
            company_id: 1,
            invoice_id: invoice.id,
            amount: dec!(50),
            payment_date: now,
            currency: "TRY".to_string(),
            payment_method: "Bank Transfer".to_string(),
            reference_number: Some("TRF001".to_string()),
            notes: None,
        };

        let result = service.create_payment(payment_create).await;
        assert!(result.is_ok());

        // Check invoice status updated
        let updated = service.get_invoice(invoice.id, 1).await.unwrap();
        assert_eq!(updated.status, InvoiceStatus::PartiallyPaid);
    }

    #[tokio::test]
    async fn test_full_payment() {
        let service = create_service().await;
        let now = Utc::now();

        let create = CreateInvoice {
            tenant_id: 1,
            company_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id: 1,
            issue_date: now,
            due_date: now + chrono::Duration::days(30),
            currency: "USD".to_string(),
            exchange_rate: Decimal::ONE,
            notes: None,
            cost_center_id: None,
            lines: vec![CreateInvoiceLine {
                product_id: Some(1),
                description: "Test".to_string(),
                quantity: dec!(1),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };

        let invoice = service.create_invoice(create).await.unwrap();
        let total = invoice.total_amount;

        let payment_create = CreatePayment {
            tenant_id: 1,
            company_id: 1,
            invoice_id: invoice.id,
            amount: total,
            payment_date: now,
            currency: "TRY".to_string(),
            payment_method: "Cash".to_string(),
            reference_number: None,
            notes: None,
        };

        service.create_payment(payment_create).await.unwrap();

        let updated = service.get_invoice(invoice.id, 1).await.unwrap();
        assert_eq!(updated.status, InvoiceStatus::Paid);
    }

    fn base_invoice(tenant_id: i64, cari_id: i64) -> CreateInvoice {
        let now = Utc::now();
        CreateInvoice {
            tenant_id,
            company_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id,
            issue_date: now,
            due_date: now + chrono::Duration::days(30),
            currency: "USD".to_string(),
            exchange_rate: Decimal::ONE,
            notes: None,
            cost_center_id: None,
            lines: vec![CreateInvoiceLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: dec!(1),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        }
    }

    #[tokio::test]
    async fn test_create_invoice_rejects_foreign_cari() {
        // Tenant 1 references cari 2, which belongs to tenant 2 -> NotFound.
        let service = create_service().await;
        let mut create = base_invoice(1, 2);
        create.lines[0].product_id = Some(1); // owned product, isolate cari failure
        let err = service.create_invoice(create).await.unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)), "got {:?}", err);
    }

    #[tokio::test]
    async fn test_create_invoice_rejects_foreign_product() {
        // Tenant 1 references product 2, which belongs to tenant 2 -> NotFound.
        let service = create_service().await;
        let mut create = base_invoice(1, 1); // owned cari, isolate product failure
        create.lines[0].product_id = Some(2);
        let err = service.create_invoice(create).await.unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)), "got {:?}", err);
    }

    #[tokio::test]
    async fn test_create_invoice_rejects_foreign_cost_center() {
        // Tenant 1 references cost center 2, which belongs to tenant 2 -> NotFound.
        let service = create_service().await;
        let mut create = base_invoice(1, 1); // owned cari + product, isolate CC failure
        create.cost_center_id = Some(2);
        let err = service.create_invoice(create).await.unwrap_err();
        assert!(matches!(err, ApiError::NotFound(_)), "got {:?}", err);
    }

    #[tokio::test]
    async fn test_create_invoice_allows_no_product_line() {
        // A line with product_id = None is a legitimate no-product line and must
        // NOT be rejected by the product-ownership precheck.
        let service = create_service().await;
        let mut create = base_invoice(1, 1);
        create.lines[0].product_id = None;
        let result = service.create_invoice(create).await;
        assert!(result.is_ok(), "got {:?}", result.err());
    }
}
