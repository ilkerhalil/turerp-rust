//! Inter-company transaction service for cross-company invoices and stock transfers.

use std::sync::Arc;

use rust_decimal::Decimal;

use crate::domain::company::service::CompanyService;
use crate::domain::inter_company::model::{
    CreateInterCompanyInvoice, CreateInterCompanyStockTransfer, InterCompanyInvoiceLine,
    InterCompanyInvoiceResult, InterCompanyStockTransferResult,
};
use crate::domain::inter_company::repository::BoxInterCompanyRepository;
use crate::domain::invoice::model::{CreateInvoice, CreateInvoiceLine, InvoiceType};
use crate::domain::invoice::service::InvoiceService;
use crate::domain::product::service::ProductService;
use crate::domain::stock::model::{CreateStockMovement, MovementType};
use crate::domain::stock::service::StockService;
use crate::error::ApiError;

/// Service for inter-company transactions.
#[derive(Clone)]
pub struct InterCompanyService {
    company_service: Arc<CompanyService>,
    invoice_service: Arc<InvoiceService>,
    stock_service: Arc<StockService>,
    product_service: Arc<ProductService>,
    repository: BoxInterCompanyRepository,
}

impl InterCompanyService {
    pub fn new(
        company_service: Arc<CompanyService>,
        invoice_service: Arc<InvoiceService>,
        stock_service: Arc<StockService>,
        product_service: Arc<ProductService>,
        repository: BoxInterCompanyRepository,
    ) -> Self {
        Self {
            company_service,
            invoice_service,
            stock_service,
            product_service,
            repository,
        }
    }

    /// Create a cross-company sales invoice and corresponding purchase invoice.
    #[tracing::instrument(skip(self))]
    pub async fn create_cross_company_invoice(
        &self,
        tenant_id: i64,
        seller_company_id: i64,
        buyer_company_id: i64,
        lines: Vec<InterCompanyInvoiceLine>,
    ) -> Result<InterCompanyInvoiceResult, ApiError> {
        // Validate both companies exist and belong to the tenant
        let _ = self
            .company_service
            .get_company(seller_company_id, tenant_id)
            .await?;
        let _ = self
            .company_service
            .get_company(buyer_company_id, tenant_id)
            .await?;

        // Transfer pricing validation: unit price must be within 20% of standard sale price
        let product_ids: Vec<i64> = lines.iter().map(|l| l.product_id).collect();
        let products = self
            .product_service
            .get_products_batch(&product_ids, tenant_id)
            .await?;
        let product_map: std::collections::HashMap<i64, _> =
            products.into_iter().map(|p| (p.id, p)).collect();

        for line in &lines {
            let product = product_map.get(&line.product_id).ok_or_else(|| {
                ApiError::NotFound(format!("Product {} not found", line.product_id))
            })?;
            if product.sale_price > Decimal::ZERO {
                let lower = product.sale_price * Decimal::from(8) / Decimal::from(10);
                let upper = product.sale_price * Decimal::from(12) / Decimal::from(10);
                if line.unit_price < lower || line.unit_price > upper {
                    return Err(ApiError::Validation(format!(
                        "Transfer pricing violation for product {}: unit price {} is outside the acceptable range {} - {} (standard sale price: {})",
                        product.code, line.unit_price, lower, upper, product.sale_price
                    )));
                }
            }
        }

        // Create sales invoice for seller
        let sales_lines: Vec<CreateInvoiceLine> = lines
            .iter()
            .map(|l| CreateInvoiceLine {
                product_id: Some(l.product_id),
                description: l.description.clone(),
                quantity: l.quantity,
                unit_price: l.unit_price,
                tax_rate: l.vat_rate,
                discount_rate: Decimal::ZERO,
            })
            .collect();

        let sales_invoice = self
            .invoice_service
            .create_invoice(CreateInvoice {
                tenant_id,
                company_id: seller_company_id,
                invoice_type: InvoiceType::SalesInvoice,
                cari_id: buyer_company_id,
                issue_date: chrono::Utc::now(),
                due_date: chrono::Utc::now() + chrono::Duration::days(30),
                currency: "TRY".to_string(),
                exchange_rate: Decimal::ONE,
                notes: Some(format!(
                    "Cross-company sale to company {}",
                    buyer_company_id
                )),
                cost_center_id: None,
                lines: sales_lines,
            })
            .await?;

        // Create purchase invoice for buyer
        let purchase_lines: Vec<CreateInvoiceLine> = lines
            .iter()
            .map(|l| CreateInvoiceLine {
                product_id: Some(l.product_id),
                description: l.description.clone(),
                quantity: l.quantity,
                unit_price: l.unit_price,
                tax_rate: l.vat_rate,
                discount_rate: Decimal::ZERO,
            })
            .collect();

        let purchase_invoice = self
            .invoice_service
            .create_invoice(CreateInvoice {
                tenant_id,
                company_id: buyer_company_id,
                invoice_type: InvoiceType::PurchaseInvoice,
                cari_id: seller_company_id,
                issue_date: chrono::Utc::now(),
                due_date: chrono::Utc::now() + chrono::Duration::days(30),
                currency: "TRY".to_string(),
                exchange_rate: Decimal::ONE,
                notes: Some(format!(
                    "Cross-company purchase from company {}",
                    seller_company_id
                )),
                cost_center_id: None,
                lines: purchase_lines,
            })
            .await?;

        let result = InterCompanyInvoiceResult {
            sales_invoice_id: sales_invoice.id,
            purchase_invoice_id: purchase_invoice.id,
        };

        self.repository
            .create_invoice(CreateInterCompanyInvoice {
                tenant_id,
                seller_company_id,
                buyer_company_id,
                lines,
                sales_invoice_id: result.sales_invoice_id,
                purchase_invoice_id: result.purchase_invoice_id,
            })
            .await?;

        Ok(result)
    }

    /// Transfer stock between companies within the same tenant.
    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(skip(self))]
    pub async fn transfer_stock_between_companies(
        &self,
        tenant_id: i64,
        from_company_id: i64,
        to_company_id: i64,
        product_id: i64,
        warehouse_id: i64,
        quantity: Decimal,
        created_by: i64,
    ) -> Result<InterCompanyStockTransferResult, ApiError> {
        // Validate companies
        let _ = self
            .company_service
            .get_company(from_company_id, tenant_id)
            .await?;
        let _ = self
            .company_service
            .get_company(to_company_id, tenant_id)
            .await?;

        // Stock out from source company
        let out_movement = self
            .stock_service
            .create_stock_movement(
                CreateStockMovement {
                    tenant_id,
                    warehouse_id,
                    company_id: from_company_id,
                    product_id,
                    movement_type: MovementType::Sale,
                    quantity,
                    reference_type: Some("InterCompanyTransfer".to_string()),
                    reference_id: Some(to_company_id),
                    notes: Some(format!("Stock transfer to company {}", to_company_id)),
                    created_by,
                },
                tenant_id,
            )
            .await?;

        // Stock in to destination company
        let in_movement = self
            .stock_service
            .create_stock_movement(
                CreateStockMovement {
                    tenant_id,
                    warehouse_id,
                    company_id: to_company_id,
                    product_id,
                    movement_type: MovementType::Purchase,
                    quantity,
                    reference_type: Some("InterCompanyTransfer".to_string()),
                    reference_id: Some(from_company_id),
                    notes: Some(format!("Stock transfer from company {}", from_company_id)),
                    created_by,
                },
                tenant_id,
            )
            .await?;

        let result = InterCompanyStockTransferResult {
            out_movement_id: out_movement.id,
            in_movement_id: in_movement.id,
        };

        self.repository
            .create_stock_transfer(CreateInterCompanyStockTransfer {
                tenant_id,
                from_company_id,
                to_company_id,
                product_id,
                warehouse_id,
                quantity,
                out_movement_id: result.out_movement_id,
                in_movement_id: result.in_movement_id,
                created_by,
            })
            .await?;

        Ok(result)
    }
}
