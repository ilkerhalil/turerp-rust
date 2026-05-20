//! Customer Portal domain models

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Portal user status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PortalUserStatus {
    #[default]
    Active,
    Passive,
    Blocked,
}

impl std::fmt::Display for PortalUserStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortalUserStatus::Active => write!(f, "active"),
            PortalUserStatus::Passive => write!(f, "passive"),
            PortalUserStatus::Blocked => write!(f, "blocked"),
        }
    }
}

impl std::str::FromStr for PortalUserStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(PortalUserStatus::Active),
            "passive" => Ok(PortalUserStatus::Passive),
            "blocked" => Ok(PortalUserStatus::Blocked),
            _ => Err(format!("Invalid portal user status: {}", s)),
        }
    }
}

/// Support ticket status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SupportTicketStatus {
    Open,
    InProgress,
    WaitingCustomer,
    Resolved,
    Closed,
}

impl std::fmt::Display for SupportTicketStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupportTicketStatus::Open => write!(f, "open"),
            SupportTicketStatus::InProgress => write!(f, "inprogress"),
            SupportTicketStatus::WaitingCustomer => write!(f, "waitingcustomer"),
            SupportTicketStatus::Resolved => write!(f, "resolved"),
            SupportTicketStatus::Closed => write!(f, "closed"),
        }
    }
}

impl std::str::FromStr for SupportTicketStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(SupportTicketStatus::Open),
            "inprogress" => Ok(SupportTicketStatus::InProgress),
            "waitingcustomer" => Ok(SupportTicketStatus::WaitingCustomer),
            "resolved" => Ok(SupportTicketStatus::Resolved),
            "closed" => Ok(SupportTicketStatus::Closed),
            _ => Err(format!("Invalid support ticket status: {}", s)),
        }
    }
}

/// Ticket priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for TicketPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TicketPriority::Low => write!(f, "low"),
            TicketPriority::Medium => write!(f, "medium"),
            TicketPriority::High => write!(f, "high"),
            TicketPriority::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for TicketPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(TicketPriority::Low),
            "medium" => Ok(TicketPriority::Medium),
            "high" => Ok(TicketPriority::High),
            "critical" => Ok(TicketPriority::Critical),
            _ => Err(format!("Invalid ticket priority: {}", s)),
        }
    }
}

/// Ticket category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TicketCategory {
    General,
    Order,
    Invoice,
    Payment,
    Technical,
    Complaint,
}

impl std::fmt::Display for TicketCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TicketCategory::General => write!(f, "general"),
            TicketCategory::Order => write!(f, "order"),
            TicketCategory::Invoice => write!(f, "invoice"),
            TicketCategory::Payment => write!(f, "payment"),
            TicketCategory::Technical => write!(f, "technical"),
            TicketCategory::Complaint => write!(f, "complaint"),
        }
    }
}

impl std::str::FromStr for TicketCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "general" => Ok(TicketCategory::General),
            "order" => Ok(TicketCategory::Order),
            "invoice" => Ok(TicketCategory::Invoice),
            "payment" => Ok(TicketCategory::Payment),
            "technical" => Ok(TicketCategory::Technical),
            "complaint" => Ok(TicketCategory::Complaint),
            _ => Err(format!("Invalid ticket category: {}", s)),
        }
    }
}

/// Portal user entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortalUser {
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
    pub status: PortalUserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

/// Support ticket entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SupportTicket {
    pub id: i64,
    pub tenant_id: i64,
    pub portal_user_id: i64,
    pub cari_id: i64,
    pub ticket_number: String,
    pub subject: String,
    pub description: String,
    pub status: SupportTicketStatus,
    pub priority: TicketPriority,
    pub category: TicketCategory,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Create portal user request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePortalUser {
    pub cari_id: i64,
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
}

/// Portal login request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortalLoginRequest {
    pub email: String,
    pub password: String,
}

/// Portal auth response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortalAuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub portal_user: PortalUserProfile,
}

/// Portal user profile (read-only view for auth response)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortalUserProfile {
    pub id: i64,
    pub email: String,
    pub full_name: String,
    pub cari_id: i64,
    pub cari_name: String,
    pub language: String,
    pub timezone: String,
}

/// Create support ticket request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSupportTicket {
    pub subject: String,
    pub description: String,
    pub priority: TicketPriority,
    pub category: TicketCategory,
}

/// Customer order view (read-only, mapped from SalesOrder)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CustomerOrderView {
    pub id: i64,
    pub order_number: String,
    pub status: String,
    pub order_date: NaiveDate,
    pub delivery_date: Option<NaiveDate>,
    pub total_amount: Decimal,
    pub currency: String,
    pub item_count: i64,
}

/// Customer invoice view (read-only, mapped from Invoice)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CustomerInvoiceView {
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

/// Customer payment view (read-only, mapped from Payment)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CustomerPaymentView {
    pub id: i64,
    pub invoice_id: i64,
    pub invoice_number: String,
    pub amount: Decimal,
    pub payment_date: NaiveDate,
    pub payment_method: String,
    pub currency: String,
}

/// Pagination parameters for portal endpoints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct PortalPaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl PortalPaginationParams {
    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }

    pub fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }
}

/// Paginated response wrapper for portal endpoints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortalPaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

impl<T> PortalPaginatedResponse<T> {
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
