//! Purchase domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Purchase order status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PurchaseOrderStatus {
    Draft,
    PendingApproval,
    Approved,
    SentToVendor,
    PartialReceived,
    Received,
    Cancelled,
    OnHold,
}

impl std::fmt::Display for PurchaseOrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "Draft"),
            Self::PendingApproval => write!(f, "PendingApproval"),
            Self::Approved => write!(f, "Approved"),
            Self::SentToVendor => write!(f, "SentToVendor"),
            Self::PartialReceived => write!(f, "PartialReceived"),
            Self::Received => write!(f, "Received"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::OnHold => write!(f, "OnHold"),
        }
    }
}

/// Purchase request status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PurchaseRequestStatus {
    Draft,
    PendingApproval,
    Approved,
    Rejected,
    ConvertedToOrder,
}

impl PurchaseRequestStatus {
    /// Check if this status can transition to the target status
    pub fn can_transition_to(&self, target: &Self) -> bool {
        match self {
            Self::Draft => matches!(target, Self::PendingApproval),
            Self::PendingApproval => {
                matches!(target, Self::Approved | Self::Rejected | Self::Draft)
            }
            Self::Approved => matches!(target, Self::ConvertedToOrder),
            Self::Rejected => matches!(target, Self::Draft), // Allow re-submission after rejection
            Self::ConvertedToOrder => false,                 // Terminal state
        }
    }

    /// Get valid next statuses
    pub fn valid_next_statuses(&self) -> Vec<Self> {
        match self {
            Self::Draft => vec![Self::PendingApproval],
            Self::PendingApproval => vec![Self::Approved, Self::Rejected, Self::Draft],
            Self::Approved => vec![Self::ConvertedToOrder],
            Self::Rejected => vec![Self::Draft],
            Self::ConvertedToOrder => vec![],
        }
    }
}

impl std::fmt::Display for PurchaseRequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "Draft"),
            Self::PendingApproval => write!(f, "PendingApproval"),
            Self::Approved => write!(f, "Approved"),
            Self::Rejected => write!(f, "Rejected"),
            Self::ConvertedToOrder => write!(f, "ConvertedToOrder"),
        }
    }
}

/// Goods receipt status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoodsReceiptStatus {
    Pending,
    Partial,
    Completed,
    Cancelled,
}

/// Purchase order entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrder {
    pub id: i64,
    pub tenant_id: i64,
    pub order_number: String,
    pub cari_id: i64,
    pub status: PurchaseOrderStatus,
    pub order_date: DateTime<Utc>,
    pub expected_delivery_date: Option<DateTime<Utc>>,
    pub subtotal: f64,
    pub tax_amount: f64,
    pub discount_amount: f64,
    pub total_amount: f64,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Purchase order line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderLine {
    pub id: i64,
    pub order_id: i64,
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: f64,
    pub received_quantity: f64,
    pub unit_price: f64,
    pub tax_rate: f64,
    pub discount_rate: f64,
    pub line_total: f64,
    pub sort_order: i32,
}

/// Purchase request entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseRequest {
    pub id: i64,
    pub tenant_id: i64,
    pub request_number: String,
    pub status: PurchaseRequestStatus,
    pub requested_by: i64,
    pub department: Option<String>,
    pub priority: String,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Purchase request line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseRequestLine {
    pub id: i64,
    pub request_id: i64,
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: f64,
    pub notes: Option<String>,
    pub sort_order: i32,
}

/// Goods receipt entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodsReceipt {
    pub id: i64,
    pub tenant_id: i64,
    pub receipt_number: String,
    pub purchase_order_id: i64,
    pub status: GoodsReceiptStatus,
    pub receipt_date: DateTime<Utc>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Goods receipt line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodsReceiptLine {
    pub id: i64,
    pub receipt_id: i64,
    pub order_line_id: i64,
    pub product_id: Option<i64>,
    pub quantity: f64,
    pub condition: String,
    pub notes: Option<String>,
}

/// Create purchase request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseRequest {
    pub tenant_id: i64,
    pub requested_by: i64,
    pub department: Option<String>,
    pub priority: String,
    pub reason: Option<String>,
    pub lines: Vec<CreatePurchaseRequestLine>,
}

impl CreatePurchaseRequest {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.lines.is_empty() {
            errors.push("Request must have at least one line item".to_string());
        }
        if self.priority.trim().is_empty() {
            errors.push("Priority is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create purchase request line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseRequestLine {
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: f64,
    pub notes: Option<String>,
}

impl CreatePurchaseRequestLine {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.description.trim().is_empty() {
            errors.push("Description is required".to_string());
        }
        if self.quantity <= 0.0 {
            errors.push("Quantity must be positive".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update purchase request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePurchaseRequest {
    pub department: Option<String>,
    pub priority: Option<String>,
    pub reason: Option<String>,
    pub status: Option<PurchaseRequestStatus>,
}

/// Update purchase request line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePurchaseRequestLine {
    pub product_id: Option<i64>,
    pub description: Option<String>,
    pub quantity: Option<f64>,
    pub notes: Option<String>,
}

/// Purchase request response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseRequestResponse {
    pub id: i64,
    pub request_number: String,
    pub status: PurchaseRequestStatus,
    pub requested_by: i64,
    pub department: Option<String>,
    pub priority: String,
    pub reason: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub lines: Vec<PurchaseRequestLine>,
}

impl From<(PurchaseRequest, Vec<PurchaseRequestLine>)> for PurchaseRequestResponse {
    fn from((request, lines): (PurchaseRequest, Vec<PurchaseRequestLine>)) -> Self {
        Self {
            id: request.id,
            request_number: request.request_number,
            status: request.status,
            requested_by: request.requested_by,
            department: request.department,
            priority: request.priority,
            reason: request.reason,
            created_at: request.created_at,
            updated_at: request.updated_at,
            lines,
        }
    }
}

/// Create purchase order request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseOrder {
    pub tenant_id: i64,
    pub cari_id: i64,
    pub order_date: DateTime<Utc>,
    pub expected_delivery_date: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub lines: Vec<CreatePurchaseOrderLine>,
}

impl CreatePurchaseOrder {
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

/// Create purchase order line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseOrderLine {
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub tax_rate: f64,
    pub discount_rate: f64,
}

impl CreatePurchaseOrderLine {
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

/// Create goods receipt request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGoodsReceipt {
    pub tenant_id: i64,
    pub purchase_order_id: i64,
    pub receipt_date: DateTime<Utc>,
    pub notes: Option<String>,
    pub lines: Vec<CreateGoodsReceiptLine>,
}

impl CreateGoodsReceipt {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.lines.is_empty() {
            errors.push("Receipt must have at least one line item".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create goods receipt line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGoodsReceiptLine {
    pub order_line_id: i64,
    pub product_id: Option<i64>,
    pub quantity: f64,
    pub condition: String,
    pub notes: Option<String>,
}

impl CreateGoodsReceiptLine {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.quantity <= 0.0 {
            errors.push("Quantity must be positive".to_string());
        }
        if self.condition.trim().is_empty() {
            errors.push("Condition is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Purchase order response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderResponse {
    pub id: i64,
    pub order_number: String,
    pub cari_id: i64,
    pub status: PurchaseOrderStatus,
    pub order_date: DateTime<Utc>,
    pub expected_delivery_date: Option<DateTime<Utc>>,
    pub subtotal: f64,
    pub tax_amount: f64,
    pub discount_amount: f64,
    pub total_amount: f64,
    pub notes: Option<String>,
    pub lines: Vec<PurchaseOrderLine>,
}

impl From<(PurchaseOrder, Vec<PurchaseOrderLine>)> for PurchaseOrderResponse {
    fn from((order, lines): (PurchaseOrder, Vec<PurchaseOrderLine>)) -> Self {
        Self {
            id: order.id,
            order_number: order.order_number,
            cari_id: order.cari_id,
            status: order.status,
            order_date: order.order_date,
            expected_delivery_date: order.expected_delivery_date,
            subtotal: order.subtotal,
            tax_amount: order.tax_amount,
            discount_amount: order.discount_amount,
            total_amount: order.total_amount,
            notes: order.notes,
            lines,
        }
    }
}

/// Goods receipt response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodsReceiptResponse {
    pub id: i64,
    pub receipt_number: String,
    pub purchase_order_id: i64,
    pub status: GoodsReceiptStatus,
    pub receipt_date: DateTime<Utc>,
    pub notes: Option<String>,
    pub lines: Vec<GoodsReceiptLine>,
}

impl From<(GoodsReceipt, Vec<GoodsReceiptLine>)> for GoodsReceiptResponse {
    fn from((receipt, lines): (GoodsReceipt, Vec<GoodsReceiptLine>)) -> Self {
        Self {
            id: receipt.id,
            receipt_number: receipt.receipt_number,
            purchase_order_id: receipt.purchase_order_id,
            status: receipt.status,
            receipt_date: receipt.receipt_date,
            notes: receipt.notes,
            lines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_purchase_order_validation() {
        let valid = CreatePurchaseOrder {
            tenant_id: 1,
            cari_id: 1,
            order_date: Utc::now(),
            expected_delivery_date: Some(Utc::now() + chrono::Duration::days(7)),
            notes: None,
            lines: vec![CreatePurchaseOrderLine {
                product_id: Some(1),
                description: "Test".to_string(),
                quantity: 1.0,
                unit_price: 100.0,
                tax_rate: 18.0,
                discount_rate: 0.0,
            }],
        };
        assert!(valid.validate().is_ok());

        let invalid = CreatePurchaseOrder {
            tenant_id: 1,
            cari_id: 1,
            order_date: Utc::now(),
            expected_delivery_date: None,
            notes: None,
            lines: vec![],
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_goods_receipt_validation() {
        let valid = CreateGoodsReceipt {
            tenant_id: 1,
            purchase_order_id: 1,
            receipt_date: Utc::now(),
            notes: None,
            lines: vec![CreateGoodsReceiptLine {
                order_line_id: 1,
                product_id: Some(1),
                quantity: 10.0,
                condition: "Good".to_string(),
                notes: None,
            }],
        };
        assert!(valid.validate().is_ok());
    }
}
