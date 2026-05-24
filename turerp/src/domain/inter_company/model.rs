//! Inter-company models for cross-company invoices and stock transfers.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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

/// Stored record of an inter-company invoice transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterCompanyInvoice {
    pub id: i64,
    pub tenant_id: i64,
    pub seller_company_id: i64,
    pub buyer_company_id: i64,
    pub lines: Vec<InterCompanyInvoiceLine>,
    pub sales_invoice_id: i64,
    pub purchase_invoice_id: i64,
    pub created_at: DateTime<Utc>,
}

/// Input for creating an inter-company invoice record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInterCompanyInvoice {
    pub tenant_id: i64,
    pub seller_company_id: i64,
    pub buyer_company_id: i64,
    pub lines: Vec<InterCompanyInvoiceLine>,
    pub sales_invoice_id: i64,
    pub purchase_invoice_id: i64,
}

/// Stored record of an inter-company stock transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterCompanyStockTransfer {
    pub id: i64,
    pub tenant_id: i64,
    pub from_company_id: i64,
    pub to_company_id: i64,
    pub product_id: i64,
    pub warehouse_id: i64,
    pub quantity: Decimal,
    pub out_movement_id: i64,
    pub in_movement_id: i64,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
}

/// Input for creating an inter-company stock transfer record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInterCompanyStockTransfer {
    pub tenant_id: i64,
    pub from_company_id: i64,
    pub to_company_id: i64,
    pub product_id: i64,
    pub warehouse_id: i64,
    pub quantity: Decimal,
    pub out_movement_id: i64,
    pub in_movement_id: i64,
    pub created_by: i64,
}
