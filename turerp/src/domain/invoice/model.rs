//! Invoice domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::impl_soft_deletable;

/// Invoice status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, ToSchema)]
pub enum InvoiceStatus {
    #[default]
    Draft,
    Pending,       // Awaiting approval
    Approved,      // Approved, awaiting payment
    Sent,          // Invoice sent to customer
    PartiallyPaid, // Partial payment received
    Paid,          // Fully paid
    Overdue,       // Payment overdue
    Cancelled,
    Refunded,
}

impl std::fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvoiceStatus::Draft => write!(f, "Draft"),
            InvoiceStatus::Pending => write!(f, "Pending"),
            InvoiceStatus::Approved => write!(f, "Approved"),
            InvoiceStatus::Sent => write!(f, "Sent"),
            InvoiceStatus::PartiallyPaid => write!(f, "PartiallyPaid"),
            InvoiceStatus::Paid => write!(f, "Paid"),
            InvoiceStatus::Overdue => write!(f, "Overdue"),
            InvoiceStatus::Cancelled => write!(f, "Cancelled"),
            InvoiceStatus::Refunded => write!(f, "Refunded"),
        }
    }
}

impl std::str::FromStr for InvoiceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(InvoiceStatus::Draft),
            "Pending" => Ok(InvoiceStatus::Pending),
            "Approved" => Ok(InvoiceStatus::Approved),
            "Sent" => Ok(InvoiceStatus::Sent),
            "PartiallyPaid" => Ok(InvoiceStatus::PartiallyPaid),
            "Paid" => Ok(InvoiceStatus::Paid),
            "Overdue" => Ok(InvoiceStatus::Overdue),
            "Cancelled" => Ok(InvoiceStatus::Cancelled),
            "Refunded" => Ok(InvoiceStatus::Refunded),
            _ => Err(format!("Invalid invoice status: {}", s)),
        }
    }
}

/// Invoice type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum InvoiceType {
    SalesInvoice,
    PurchaseInvoice,
    SalesReturn,
    PurchaseReturn,
}

impl std::fmt::Display for InvoiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvoiceType::SalesInvoice => write!(f, "SalesInvoice"),
            InvoiceType::PurchaseInvoice => write!(f, "PurchaseInvoice"),
            InvoiceType::SalesReturn => write!(f, "SalesReturn"),
            InvoiceType::PurchaseReturn => write!(f, "PurchaseReturn"),
        }
    }
}

impl std::str::FromStr for InvoiceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SalesInvoice" => Ok(InvoiceType::SalesInvoice),
            "PurchaseInvoice" => Ok(InvoiceType::PurchaseInvoice),
            "SalesReturn" => Ok(InvoiceType::SalesReturn),
            "PurchaseReturn" => Ok(InvoiceType::PurchaseReturn),
            _ => Err(format!("Invalid invoice type: {}", s)),
        }
    }
}

/// Invoice entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Invoice {
    pub id: i64,
    pub tenant_id: i64,
    pub invoice_number: String,
    pub invoice_type: InvoiceType,
    pub status: InvoiceStatus,
    pub cari_id: i64,
    pub issue_date: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub currency: String,
    pub exchange_rate: Decimal,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(Invoice);

/// Invoice line item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvoiceLine {
    pub id: i64,
    pub invoice_id: i64,
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub discount_rate: Decimal,
    pub line_total: Decimal,
    pub sort_order: i32,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(InvoiceLine);

/// Payment record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Payment {
    pub id: i64,
    pub tenant_id: i64,
    pub invoice_id: i64,
    pub amount: Decimal,
    pub currency: String,
    pub payment_date: DateTime<Utc>,
    pub payment_method: String,
    pub reference_number: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(Payment);

/// Create invoice request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateInvoice {
    pub tenant_id: i64,
    pub invoice_type: InvoiceType,
    pub cari_id: i64,
    pub issue_date: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub currency: String,
    #[serde(default = "default_exchange_rate")]
    pub exchange_rate: Decimal,
    pub notes: Option<String>,
    pub lines: Vec<CreateInvoiceLine>,
}

fn default_exchange_rate() -> Decimal {
    Decimal::ONE
}

impl CreateInvoice {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.due_date < self.issue_date {
            errors.push("Due date must be after issue date".to_string());
        }
        if self.lines.is_empty() {
            errors.push("Invoice must have at least one line item".to_string());
        }
        if self.currency.trim().is_empty() {
            errors.push("Currency is required".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create invoice line
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateInvoiceLine {
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub discount_rate: Decimal,
}

impl CreateInvoiceLine {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.description.trim().is_empty() {
            errors.push("Description is required".to_string());
        }
        if self.quantity <= Decimal::ZERO {
            errors.push("Quantity must be positive".to_string());
        }
        if self.unit_price < Decimal::ZERO {
            errors.push("Unit price cannot be negative".to_string());
        }
        if self.tax_rate < Decimal::ZERO || self.tax_rate > Decimal::ONE_HUNDRED {
            errors.push("Tax rate must be between 0 and 100".to_string());
        }
        if self.discount_rate < Decimal::ZERO || self.discount_rate > Decimal::ONE_HUNDRED {
            errors.push("Discount rate must be between 0 and 100".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn calculate_line_total(&self) -> Decimal {
        let subtotal = self.quantity * self.unit_price;
        let discount = subtotal * (self.discount_rate / Decimal::ONE_HUNDRED);
        let after_discount = subtotal - discount;
        let tax = after_discount * (self.tax_rate / Decimal::ONE_HUNDRED);
        after_discount + tax
    }
}

/// Create payment request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePayment {
    pub tenant_id: i64,
    pub invoice_id: i64,
    pub amount: Decimal,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub payment_date: DateTime<Utc>,
    pub payment_method: String,
    pub reference_number: Option<String>,
    pub notes: Option<String>,
}

fn default_currency() -> String {
    "TRY".to_string()
}

impl CreatePayment {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.amount <= Decimal::ZERO {
            errors.push("Payment amount must be positive".to_string());
        }
        if self.payment_method.trim().is_empty() {
            errors.push("Payment method is required".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Invoice response for API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InvoiceResponse {
    pub id: i64,
    pub invoice_number: String,
    pub invoice_type: InvoiceType,
    pub status: InvoiceStatus,
    pub cari_id: i64,
    pub issue_date: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub currency: String,
    pub exchange_rate: Decimal,
    pub notes: Option<String>,
    pub lines: Vec<InvoiceLine>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl From<(Invoice, Vec<InvoiceLine>)> for InvoiceResponse {
    fn from((invoice, lines): (Invoice, Vec<InvoiceLine>)) -> Self {
        Self {
            id: invoice.id,
            invoice_number: invoice.invoice_number,
            invoice_type: invoice.invoice_type,
            status: invoice.status,
            cari_id: invoice.cari_id,
            issue_date: invoice.issue_date,
            due_date: invoice.due_date,
            subtotal: invoice.subtotal,
            tax_amount: invoice.tax_amount,
            discount_amount: invoice.discount_amount,
            total_amount: invoice.total_amount,
            paid_amount: invoice.paid_amount,
            currency: invoice.currency,
            exchange_rate: invoice.exchange_rate,
            notes: invoice.notes,
            lines,
            deleted_at: invoice.deleted_at,
            deleted_by: invoice.deleted_by,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::SoftDeletable;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    #[test]
    fn test_create_invoice_validation() {
        let now = Utc::now();
        let valid = CreateInvoice {
            tenant_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id: 1,
            issue_date: now,
            due_date: now + chrono::Duration::days(30),
            currency: "USD".to_string(),
            exchange_rate: Decimal::ONE,
            notes: None,
            lines: vec![CreateInvoiceLine {
                product_id: Some(1),
                description: "Test Product".to_string(),
                quantity: dec!(1),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateInvoice {
            tenant_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id: 1,
            issue_date: now,
            due_date: now - chrono::Duration::days(1),
            currency: "USD".to_string(),
            exchange_rate: Decimal::ONE,
            notes: None,
            lines: vec![],
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_invoice_line_calculation() {
        let line = CreateInvoiceLine {
            product_id: Some(1),
            description: "Test".to_string(),
            quantity: dec!(2),
            unit_price: dec!(100),
            tax_rate: dec!(18),
            discount_rate: dec!(10),
        };
        // 2 * 100 = 200
        // 10% discount = 20
        // After discount = 180
        // 18% tax = 32.4
        // Total = 212.4
        assert_eq!(line.calculate_line_total(), dec!(212.4));
    }
}
