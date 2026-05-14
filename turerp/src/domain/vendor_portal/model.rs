//! Vendor Portal domain models

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Vendor user status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum VendorUserStatus {
    #[default]
    Active,
    Passive,
    Blocked,
}

impl std::fmt::Display for VendorUserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VendorUserStatus::Active => write!(f, "active"),
            VendorUserStatus::Passive => write!(f, "passive"),
            VendorUserStatus::Blocked => write!(f, "blocked"),
        }
    }
}

impl std::str::FromStr for VendorUserStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(VendorUserStatus::Active),
            "passive" => Ok(VendorUserStatus::Passive),
            "blocked" => Ok(VendorUserStatus::Blocked),
            _ => Err(format!("Invalid vendor user status: {}", s)),
        }
    }
}

/// Delivery note status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryNoteStatus {
    Draft,
    Shipped,
    PartialReceived,
    Received,
    Cancelled,
}

impl std::fmt::Display for DeliveryNoteStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeliveryNoteStatus::Draft => write!(f, "draft"),
            DeliveryNoteStatus::Shipped => write!(f, "shipped"),
            DeliveryNoteStatus::PartialReceived => write!(f, "partialreceived"),
            DeliveryNoteStatus::Received => write!(f, "received"),
            DeliveryNoteStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for DeliveryNoteStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(DeliveryNoteStatus::Draft),
            "shipped" => Ok(DeliveryNoteStatus::Shipped),
            "partialreceived" => Ok(DeliveryNoteStatus::PartialReceived),
            "received" => Ok(DeliveryNoteStatus::Received),
            "cancelled" => Ok(DeliveryNoteStatus::Cancelled),
            _ => Err(format!("Invalid delivery note status: {}", s)),
        }
    }
}

/// Vendor user entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VendorUser {
    pub id: i64,
    pub tenant_id: i64,
    pub cari_id: i64,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub language: String,
    pub timezone: String,
    pub status: VendorUserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

/// Delivery note entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeliveryNote {
    pub id: i64,
    pub tenant_id: i64,
    pub vendor_user_id: i64,
    pub cari_id: i64,
    pub note_number: String,
    pub purchase_order_id: i64,
    pub description: String,
    pub status: DeliveryNoteStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub shipped_at: Option<DateTime<Utc>>,
}

/// Create vendor user request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateVendorUser {
    pub cari_id: i64,
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
}

/// Vendor login request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VendorLoginRequest {
    pub email: String,
    pub password: String,
}

/// Vendor auth response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VendorAuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub vendor_user: VendorUserProfile,
}

/// Vendor user profile (read-only view for auth response)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VendorUserProfile {
    pub id: i64,
    pub email: String,
    pub full_name: String,
    pub cari_id: i64,
    pub cari_name: String,
    pub language: String,
    pub timezone: String,
}

/// Create delivery note request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDeliveryNote {
    pub purchase_order_id: i64,
    pub description: String,
}

/// Vendor order view (read-only, mapped from PurchaseOrder)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VendorOrderView {
    pub id: i64,
    pub order_number: String,
    pub status: String,
    pub order_date: NaiveDate,
    pub expected_delivery_date: Option<NaiveDate>,
    pub total_amount: Decimal,
    pub currency: String,
    pub item_count: i64,
}

/// Vendor invoice view (read-only, mapped from Invoice)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VendorInvoiceView {
    pub id: i64,
    pub invoice_number: String,
    pub invoice_type: String,
    pub status: String,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub outstanding_amount: Decimal,
    pub currency: String,
}

/// Vendor payment view (read-only, mapped from Payment)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VendorPaymentView {
    pub id: i64,
    pub invoice_id: i64,
    pub invoice_number: String,
    pub amount: Decimal,
    pub payment_date: NaiveDate,
    pub payment_method: String,
    pub currency: String,
}

/// Pagination parameters for vendor portal endpoints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct VendorPaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl VendorPaginationParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }
}

/// Paginated response wrapper for vendor portal endpoints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VendorPaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

impl<T> VendorPaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, page: i64, per_page: i64) -> Self {
        let total_pages = if per_page > 0 {
            ((total + per_page - 1) / per_page).max(1)
        } else {
            1
        };
        Self {
            data,
            total,
            page,
            per_page,
            total_pages,
        }
    }
}
