//! Inter-company repository

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::domain::inter_company::model::{
    CreateInterCompanyInvoice, CreateInterCompanyStockTransfer, InterCompanyInvoice,
    InterCompanyStockTransfer,
};
use crate::error::ApiError;

/// Repository trait for inter-company operations.
#[async_trait]
pub trait InterCompanyRepository: Send + Sync {
    async fn create_invoice(
        &self,
        invoice: CreateInterCompanyInvoice,
    ) -> Result<InterCompanyInvoice, ApiError>;
    async fn get_invoice(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<InterCompanyInvoice>, ApiError>;
    async fn list_invoices(&self, tenant_id: i64) -> Result<Vec<InterCompanyInvoice>, ApiError>;

    async fn create_stock_transfer(
        &self,
        transfer: CreateInterCompanyStockTransfer,
    ) -> Result<InterCompanyStockTransfer, ApiError>;
    async fn get_stock_transfer(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<InterCompanyStockTransfer>, ApiError>;
    async fn list_stock_transfers(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<InterCompanyStockTransfer>, ApiError>;
}

/// Type alias for boxed inter-company repository.
pub type BoxInterCompanyRepository = Arc<dyn InterCompanyRepository>;

struct InMemoryInterCompanyInner {
    invoices: std::collections::HashMap<i64, InterCompanyInvoice>,
    transfers: std::collections::HashMap<i64, InterCompanyStockTransfer>,
    next_invoice_id: i64,
    next_transfer_id: i64,
    tenant_invoices: std::collections::HashMap<i64, Vec<i64>>,
    tenant_transfers: std::collections::HashMap<i64, Vec<i64>>,
}

/// In-memory inter-company repository.
pub struct InMemoryInterCompanyRepository {
    inner: Mutex<InMemoryInterCompanyInner>,
}

impl InMemoryInterCompanyRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryInterCompanyInner {
                invoices: std::collections::HashMap::new(),
                transfers: std::collections::HashMap::new(),
                next_invoice_id: 1,
                next_transfer_id: 1,
                tenant_invoices: std::collections::HashMap::new(),
                tenant_transfers: std::collections::HashMap::new(),
            }),
        }
    }
}

impl Default for InMemoryInterCompanyRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InterCompanyRepository for InMemoryInterCompanyRepository {
    async fn create_invoice(
        &self,
        create: CreateInterCompanyInvoice,
    ) -> Result<InterCompanyInvoice, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_invoice_id;
        inner.next_invoice_id += 1;

        let invoice = InterCompanyInvoice {
            id,
            tenant_id: create.tenant_id,
            seller_company_id: create.seller_company_id,
            buyer_company_id: create.buyer_company_id,
            lines: create.lines,
            sales_invoice_id: create.sales_invoice_id,
            purchase_invoice_id: create.purchase_invoice_id,
            created_at: chrono::Utc::now(),
        };

        inner.invoices.insert(id, invoice.clone());
        inner
            .tenant_invoices
            .entry(create.tenant_id)
            .or_default()
            .push(id);

        Ok(invoice)
    }

    async fn get_invoice(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<InterCompanyInvoice>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .invoices
            .get(&id)
            .filter(|i| i.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_invoices(&self, tenant_id: i64) -> Result<Vec<InterCompanyInvoice>, ApiError> {
        let inner = self.inner.lock();
        let ids = inner
            .tenant_invoices
            .get(&tenant_id)
            .cloned()
            .unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| inner.invoices.get(id).cloned())
            .collect())
    }

    async fn create_stock_transfer(
        &self,
        create: CreateInterCompanyStockTransfer,
    ) -> Result<InterCompanyStockTransfer, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_transfer_id;
        inner.next_transfer_id += 1;

        let transfer = InterCompanyStockTransfer {
            id,
            tenant_id: create.tenant_id,
            from_company_id: create.from_company_id,
            to_company_id: create.to_company_id,
            product_id: create.product_id,
            warehouse_id: create.warehouse_id,
            quantity: create.quantity,
            out_movement_id: create.out_movement_id,
            in_movement_id: create.in_movement_id,
            created_by: create.created_by,
            created_at: chrono::Utc::now(),
        };

        inner.transfers.insert(id, transfer.clone());
        inner
            .tenant_transfers
            .entry(create.tenant_id)
            .or_default()
            .push(id);

        Ok(transfer)
    }

    async fn get_stock_transfer(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<InterCompanyStockTransfer>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .transfers
            .get(&id)
            .filter(|t| t.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_stock_transfers(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<InterCompanyStockTransfer>, ApiError> {
        let inner = self.inner.lock();
        let ids = inner
            .tenant_transfers
            .get(&tenant_id)
            .cloned()
            .unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| inner.transfers.get(id).cloned())
            .collect())
    }
}
