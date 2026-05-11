//! Inter-company transaction service for cross-company invoices and stock transfers.

use crate::domain::company::service::CompanyService;
use crate::domain::invoice::model::{CreateInvoice, CreateInvoiceLine, InvoiceType};
use crate::domain::invoice::service::InvoiceService;
use crate::domain::product::service::ProductService;
use crate::domain::stock::model::{CreateStockMovement, MovementType};
use crate::domain::stock::service::StockService;
use crate::error::ApiError;
use num_traits::FromPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Maximum allowed deviation from standard sale price for transfer pricing.
const TRANSFER_PRICE_TOLERANCE: f64 = 0.20;

/// Service for inter-company transactions.
#[derive(Clone)]
pub struct InterCompanyService {
    company_service: Arc<CompanyService>,
    invoice_service: Arc<InvoiceService>,
    stock_service: Arc<StockService>,
    product_service: Arc<ProductService>,
}

impl InterCompanyService {
    pub fn new(
        company_service: Arc<CompanyService>,
        invoice_service: Arc<InvoiceService>,
        stock_service: Arc<StockService>,
        product_service: Arc<ProductService>,
    ) -> Self {
        Self {
            company_service,
            invoice_service,
            stock_service,
            product_service,
        }
    }

    /// Create a cross-company sales invoice and corresponding purchase invoice.
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
        for line in &lines {
            let product = self
                .product_service
                .get_product(line.product_id, tenant_id)
                .await?;
            if product.sale_price > Decimal::ZERO {
                let lower = product.sale_price
                    * Decimal::from_f64(1.0 - TRANSFER_PRICE_TOLERANCE)
                        .unwrap_or_else(|| Decimal::from(8) / Decimal::from(10));
                let upper = product.sale_price
                    * Decimal::from_f64(1.0 + TRANSFER_PRICE_TOLERANCE)
                        .unwrap_or_else(|| Decimal::from(12) / Decimal::from(10));
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

        Ok(InterCompanyInvoiceResult {
            sales_invoice_id: sales_invoice.id,
            purchase_invoice_id: purchase_invoice.id,
        })
    }

    /// Transfer stock between companies within the same tenant.
    pub async fn transfer_stock_between_companies(
        &self,
        tenant_id: i64,
        from_company_id: i64,
        to_company_id: i64,
        product_id: i64,
        warehouse_id: i64,
        quantity: Decimal,
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
                    warehouse_id,
                    company_id: from_company_id,
                    product_id,
                    movement_type: MovementType::Sale,
                    quantity,
                    reference_type: Some("InterCompanyTransfer".to_string()),
                    reference_id: Some(to_company_id),
                    notes: Some(format!("Stock transfer to company {}", to_company_id)),
                    created_by: 0,
                },
                tenant_id,
            )
            .await?;

        // Stock in to destination company
        let in_movement = self
            .stock_service
            .create_stock_movement(
                CreateStockMovement {
                    warehouse_id,
                    company_id: to_company_id,
                    product_id,
                    movement_type: MovementType::Purchase,
                    quantity,
                    reference_type: Some("InterCompanyTransfer".to_string()),
                    reference_id: Some(from_company_id),
                    notes: Some(format!("Stock transfer from company {}", from_company_id)),
                    created_by: 0,
                },
                tenant_id,
            )
            .await?;

        Ok(InterCompanyStockTransferResult {
            out_movement_id: out_movement.id,
            in_movement_id: in_movement.id,
        })
    }
}

/// Line item for an inter-company invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterCompanyInvoiceLine {
    pub product_id: i64,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub vat_rate: Decimal,
}

/// Result of a cross-company invoice.
#[derive(Debug, Clone, Serialize)]
pub struct InterCompanyInvoiceResult {
    pub sales_invoice_id: i64,
    pub purchase_invoice_id: i64,
}

/// Result of an inter-company stock transfer.
#[derive(Debug, Clone, Serialize)]
pub struct InterCompanyStockTransferResult {
    pub out_movement_id: i64,
    pub in_movement_id: i64,
}
