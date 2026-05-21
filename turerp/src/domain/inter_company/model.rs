//! Inter-company models for cross-company invoices and stock transfers.

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
