//! Sales domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Sales order status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SalesOrderStatus {
    Draft,
    PendingApproval,
    Approved,
    InProgress,
    Shipped,
    Delivered,
    Cancelled,
    OnHold,
}

impl std::fmt::Display for SalesOrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "Draft"),
            Self::PendingApproval => write!(f, "PendingApproval"),
            Self::Approved => write!(f, "Approved"),
            Self::InProgress => write!(f, "InProgress"),
            Self::Shipped => write!(f, "Shipped"),
            Self::Delivered => write!(f, "Delivered"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::OnHold => write!(f, "OnHold"),
        }
    }
}

impl std::str::FromStr for SalesOrderStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(Self::Draft),
            "PendingApproval" => Ok(Self::PendingApproval),
            "Approved" => Ok(Self::Approved),
            "InProgress" => Ok(Self::InProgress),
            "Shipped" => Ok(Self::Shipped),
            "Delivered" => Ok(Self::Delivered),
            "Cancelled" => Ok(Self::Cancelled),
            "OnHold" => Ok(Self::OnHold),
            _ => Err(format!("Invalid sales order status: {}", s)),
        }
    }
}

/// Quotation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuotationStatus {
    Draft,
    Sent,
    UnderReview,
    Accepted,
    Rejected,
    Expired,
    ConvertedToOrder,
}

impl std::fmt::Display for QuotationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "Draft"),
            Self::Sent => write!(f, "Sent"),
            Self::UnderReview => write!(f, "UnderReview"),
            Self::Accepted => write!(f, "Accepted"),
            Self::Rejected => write!(f, "Rejected"),
            Self::Expired => write!(f, "Expired"),
            Self::ConvertedToOrder => write!(f, "ConvertedToOrder"),
        }
    }
}

impl std::str::FromStr for QuotationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(Self::Draft),
            "Sent" => Ok(Self::Sent),
            "UnderReview" => Ok(Self::UnderReview),
            "Accepted" => Ok(Self::Accepted),
            "Rejected" => Ok(Self::Rejected),
            "Expired" => Ok(Self::Expired),
            "ConvertedToOrder" => Ok(Self::ConvertedToOrder),
            _ => Err(format!("Invalid quotation status: {}", s)),
        }
    }
}

/// Sales order entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrder {
    pub id: i64,
    pub tenant_id: i64,
    pub order_number: String,
    pub cari_id: i64,
    pub status: SalesOrderStatus,
    pub order_date: DateTime<Utc>,
    pub delivery_date: Option<DateTime<Utc>>,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub notes: Option<String>,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Sales order line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderLine {
    pub id: i64,
    pub order_id: i64,
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub discount_rate: Decimal,
    pub line_total: Decimal,
    pub sort_order: i32,
}

/// Quotation entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quotation {
    pub id: i64,
    pub tenant_id: i64,
    pub quotation_number: String,
    pub cari_id: i64,
    pub status: QuotationStatus,
    pub valid_until: DateTime<Utc>,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub sales_order_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Quotation line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotationLine {
    pub id: i64,
    pub quotation_id: i64,
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub discount_rate: Decimal,
    pub line_total: Decimal,
    pub sort_order: i32,
}

/// Create sales order request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSalesOrder {
    pub tenant_id: i64,
    pub cari_id: i64,
    pub order_date: DateTime<Utc>,
    pub delivery_date: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub lines: Vec<CreateSalesOrderLine>,
}

impl CreateSalesOrder {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.lines.is_empty() {
            errors.push("Order must have at least one line item".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create sales order line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSalesOrderLine {
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub discount_rate: Decimal,
}

impl CreateSalesOrderLine {
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

/// Create quotation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQuotation {
    pub tenant_id: i64,
    pub cari_id: i64,
    pub valid_until: DateTime<Utc>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub lines: Vec<CreateQuotationLine>,
}

impl CreateQuotation {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.lines.is_empty() {
            errors.push("Quotation must have at least one line item".to_string());
        }
        if self.valid_until < Utc::now() {
            errors.push("Valid until date must be in the future".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create quotation line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQuotationLine {
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub discount_rate: Decimal,
}

impl CreateQuotationLine {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.description.trim().is_empty() {
            errors.push("Description is required".to_string());
        }
        if self.quantity <= Decimal::ZERO {
            errors.push("Quantity must be positive".to_string());
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

/// Sales order response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesOrderResponse {
    pub id: i64,
    pub order_number: String,
    pub cari_id: i64,
    pub status: SalesOrderStatus,
    pub order_date: DateTime<Utc>,
    pub delivery_date: Option<DateTime<Utc>>,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub notes: Option<String>,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub lines: Vec<SalesOrderLine>,
}

impl From<(SalesOrder, Vec<SalesOrderLine>)> for SalesOrderResponse {
    fn from((order, lines): (SalesOrder, Vec<SalesOrderLine>)) -> Self {
        Self {
            id: order.id,
            order_number: order.order_number,
            cari_id: order.cari_id,
            status: order.status,
            order_date: order.order_date,
            delivery_date: order.delivery_date,
            subtotal: order.subtotal,
            tax_amount: order.tax_amount,
            discount_amount: order.discount_amount,
            total_amount: order.total_amount,
            notes: order.notes,
            shipping_address: order.shipping_address,
            billing_address: order.billing_address,
            lines,
        }
    }
}

/// Quotation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotationResponse {
    pub id: i64,
    pub quotation_number: String,
    pub cari_id: i64,
    pub status: QuotationStatus,
    pub valid_until: DateTime<Utc>,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub sales_order_id: Option<i64>,
    pub lines: Vec<QuotationLine>,
}

impl From<(Quotation, Vec<QuotationLine>)> for QuotationResponse {
    fn from((quotation, lines): (Quotation, Vec<QuotationLine>)) -> Self {
        Self {
            id: quotation.id,
            quotation_number: quotation.quotation_number,
            cari_id: quotation.cari_id,
            status: quotation.status,
            valid_until: quotation.valid_until,
            subtotal: quotation.subtotal,
            tax_amount: quotation.tax_amount,
            discount_amount: quotation.discount_amount,
            total_amount: quotation.total_amount,
            notes: quotation.notes,
            terms: quotation.terms,
            sales_order_id: quotation.sales_order_id,
            lines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use rust_decimal_macros::dec;

    #[test]
    fn test_create_sales_order_validation() {
        let valid = CreateSalesOrder {
            tenant_id: 1,
            cari_id: 1,
            order_date: Utc::now(),
            delivery_date: Some(Utc::now() + Duration::days(7)),
            notes: None,
            shipping_address: None,
            billing_address: None,
            lines: vec![CreateSalesOrderLine {
                product_id: Some(1),
                description: "Test".to_string(),
                quantity: dec!(1),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateSalesOrder {
            tenant_id: 1,
            cari_id: 1,
            order_date: Utc::now(),
            delivery_date: None,
            notes: None,
            shipping_address: None,
            billing_address: None,
            lines: vec![],
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_quotation_validation() {
        let valid = CreateQuotation {
            tenant_id: 1,
            cari_id: 1,
            valid_until: Utc::now() + Duration::days(30),
            notes: None,
            terms: None,
            lines: vec![CreateQuotationLine {
                product_id: Some(1),
                description: "Test".to_string(),
                quantity: dec!(1),
                unit_price: dec!(100),
                tax_rate: dec!(18),
                discount_rate: dec!(0),
            }],
        };
        assert!(valid.validate().is_ok());
    }
}
