//! Invoice domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Invoice status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InvoiceStatus {
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

/// Invoice type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InvoiceType {
    SalesInvoice,
    PurchaseInvoice,
    SalesReturn,
    PurchaseReturn,
}

/// Invoice entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: i64,
    pub tenant_id: i64,
    pub invoice_number: String,
    pub invoice_type: InvoiceType,
    pub status: InvoiceStatus,
    pub cari_id: i64,
    pub issue_date: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub subtotal: f64,
    pub tax_amount: f64,
    pub discount_amount: f64,
    pub total_amount: f64,
    pub paid_amount: f64,
    pub currency: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Invoice line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub id: i64,
    pub invoice_id: i64,
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub tax_rate: f64,
    pub discount_rate: f64,
    pub line_total: f64,
    pub sort_order: i32,
}

/// Payment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: i64,
    pub tenant_id: i64,
    pub invoice_id: i64,
    pub amount: f64,
    pub payment_date: DateTime<Utc>,
    pub payment_method: String,
    pub reference_number: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Create invoice request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvoice {
    pub tenant_id: i64,
    pub invoice_type: InvoiceType,
    pub cari_id: i64,
    pub issue_date: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub currency: String,
    pub notes: Option<String>,
    pub lines: Vec<CreateInvoiceLine>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvoiceLine {
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub tax_rate: f64,
    pub discount_rate: f64,
}

impl CreateInvoiceLine {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.description.trim().is_empty() {
            errors.push("Description is required".to_string());
        }
        if self.quantity <= 0.0 {
            errors.push("Quantity must be positive".to_string());
        }
        if self.unit_price < 0.0 {
            errors.push("Unit price cannot be negative".to_string());
        }
        if self.tax_rate < 0.0 || self.tax_rate > 100.0 {
            errors.push("Tax rate must be between 0 and 100".to_string());
        }
        if self.discount_rate < 0.0 || self.discount_rate > 100.0 {
            errors.push("Discount rate must be between 0 and 100".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn calculate_line_total(&self) -> f64 {
        let subtotal = self.quantity * self.unit_price;
        let discount = subtotal * (self.discount_rate / 100.0);
        let after_discount = subtotal - discount;
        let tax = after_discount * (self.tax_rate / 100.0);
        after_discount + tax
    }
}

/// Create payment request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePayment {
    pub tenant_id: i64,
    pub invoice_id: i64,
    pub amount: f64,
    pub payment_date: DateTime<Utc>,
    pub payment_method: String,
    pub reference_number: Option<String>,
    pub notes: Option<String>,
}

impl CreatePayment {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.amount <= 0.0 {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceResponse {
    pub id: i64,
    pub invoice_number: String,
    pub invoice_type: InvoiceType,
    pub status: InvoiceStatus,
    pub cari_id: i64,
    pub issue_date: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub subtotal: f64,
    pub tax_amount: f64,
    pub discount_amount: f64,
    pub total_amount: f64,
    pub paid_amount: f64,
    pub currency: String,
    pub notes: Option<String>,
    pub lines: Vec<InvoiceLine>,
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
            notes: invoice.notes,
            lines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

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
        assert!(valid.validate().is_ok());

        let invalid = CreateInvoice {
            tenant_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id: 1,
            issue_date: now,
            due_date: now - chrono::Duration::days(1),
            currency: "USD".to_string(),
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
            quantity: 2.0,
            unit_price: 100.0,
            tax_rate: 18.0,
            discount_rate: 10.0,
        };
        // 2 * 100 = 200
        // 10% discount = 20
        // After discount = 180
        // 18% tax = 32.4
        // Total = 212.4
        assert!((line.calculate_line_total() - 212.4).abs() < 0.01);
    }
}
