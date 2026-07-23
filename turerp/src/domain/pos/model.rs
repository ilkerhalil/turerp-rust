//! POS domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::impl_soft_deletable;

/// POS terminal status
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum PosTerminalStatus {
    #[default]
    Active,
    Inactive,
    Offline,
    Syncing,
}

impl std::fmt::Display for PosTerminalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::Inactive => write!(f, "Inactive"),
            Self::Offline => write!(f, "Offline"),
            Self::Syncing => write!(f, "Syncing"),
        }
    }
}

impl std::str::FromStr for PosTerminalStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Active" => Ok(Self::Active),
            "Inactive" => Ok(Self::Inactive),
            "Offline" => Ok(Self::Offline),
            "Syncing" => Ok(Self::Syncing),
            _ => Err(format!("Invalid POS terminal status: {}", s)),
        }
    }
}

/// Z-report status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum ZReportStatus {
    Open,
    Closed,
    Reconciled,
}

impl std::fmt::Display for ZReportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "Open"),
            Self::Closed => write!(f, "Closed"),
            Self::Reconciled => write!(f, "Reconciled"),
        }
    }
}

impl std::str::FromStr for ZReportStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Open" => Ok(Self::Open),
            "Closed" => Ok(Self::Closed),
            "Reconciled" => Ok(Self::Reconciled),
            _ => Err(format!("Invalid Z-report status: {}", s)),
        }
    }
}

/// Sync queue status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum SyncQueueStatus {
    Pending,
    Synced,
    Failed,
}

impl std::fmt::Display for SyncQueueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Synced => write!(f, "Synced"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Payment method for POS sales
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum PaymentMethod {
    Cash,
    CreditCard,
    DebitCard,
    MobilePayment,
    Voucher,
    Other,
}

impl std::fmt::Display for PaymentMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cash => write!(f, "Cash"),
            Self::CreditCard => write!(f, "CreditCard"),
            Self::DebitCard => write!(f, "DebitCard"),
            Self::MobilePayment => write!(f, "MobilePayment"),
            Self::Voucher => write!(f, "Voucher"),
            Self::Other => write!(f, "Other"),
        }
    }
}

impl std::str::FromStr for PaymentMethod {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Cash" => Ok(Self::Cash),
            "CreditCard" => Ok(Self::CreditCard),
            "DebitCard" => Ok(Self::DebitCard),
            "MobilePayment" => Ok(Self::MobilePayment),
            "Voucher" => Ok(Self::Voucher),
            "Other" => Ok(Self::Other),
            _ => Err(format!("Invalid payment method: {}", s)),
        }
    }
}

/// POS terminal entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PosTerminal {
    pub id: i64,
    pub tenant_id: i64,
    pub terminal_code: String,
    pub name: String,
    pub warehouse_id: Option<i64>,
    pub status: PosTerminalStatus,
    pub store_name: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(PosTerminal);

/// POS sale entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PosSale {
    pub id: i64,
    pub tenant_id: i64,
    pub terminal_id: i64,
    pub sale_number: String,
    pub cari_id: Option<i64>,
    pub sale_date: DateTime<Utc>,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub payment_method: PaymentMethod,
    pub z_report_id: Option<i64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(PosSale);

/// POS sale line item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PosSaleLine {
    pub id: i64,
    pub tenant_id: i64,
    pub sale_id: i64,
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub discount_amount: Decimal,
    pub line_total: Decimal,
    pub created_at: DateTime<Utc>,
}

/// Z-report entity (end-of-day reconciliation)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ZReport {
    pub id: i64,
    pub tenant_id: i64,
    pub terminal_id: i64,
    pub report_number: String,
    pub report_date: DateTime<Utc>,
    pub status: ZReportStatus,
    pub total_sales: Decimal,
    pub total_cash: Decimal,
    pub total_card: Decimal,
    pub total_other: Decimal,
    pub total_tax: Decimal,
    pub total_discount: Decimal,
    pub transaction_count: u32,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Sync queue item (offline mode)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyncQueueItem {
    pub id: i64,
    pub tenant_id: i64,
    pub terminal_id: i64,
    pub payload: String,
    pub status: SyncQueueStatus,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub synced_at: Option<DateTime<Utc>>,
}

// --- DTOs ---

/// Create POS terminal request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePosTerminal {
    pub tenant_id: i64,
    pub terminal_code: String,
    pub name: String,
    pub warehouse_id: Option<i64>,
    pub store_name: Option<String>,
}

/// Update POS terminal request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdatePosTerminal {
    pub name: Option<String>,
    pub warehouse_id: Option<Option<i64>>,
    pub status: Option<PosTerminalStatus>,
    pub store_name: Option<String>,
}

/// Create POS sale request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePosSale {
    pub tenant_id: i64,
    pub terminal_id: i64,
    pub cari_id: Option<i64>,
    pub sale_date: DateTime<Utc>,
    pub payment_method: PaymentMethod,
    pub lines: Vec<CreatePosSaleLine>,
    pub discount_amount: Option<Decimal>,
    pub notes: Option<String>,
}

/// Create POS sale line request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatePosSaleLine {
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub discount_amount: Option<Decimal>,
}

/// Create Z-report request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateZReport {
    pub tenant_id: i64,
    pub terminal_id: i64,
}

// --- Response DTOs ---

/// POS terminal response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PosTerminalResponse {
    pub id: i64,
    pub terminal_code: String,
    pub name: String,
    pub warehouse_id: Option<i64>,
    pub status: String,
    pub store_name: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PosTerminal> for PosTerminalResponse {
    fn from(t: PosTerminal) -> Self {
        Self {
            id: t.id,
            terminal_code: t.terminal_code,
            name: t.name,
            warehouse_id: t.warehouse_id,
            status: t.status.to_string(),
            store_name: t.store_name,
            last_sync_at: t.last_sync_at,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

/// POS sale response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PosSaleResponse {
    pub id: i64,
    pub terminal_id: i64,
    pub sale_number: String,
    pub cari_id: Option<i64>,
    pub sale_date: DateTime<Utc>,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub payment_method: String,
    pub z_report_id: Option<i64>,
    pub notes: Option<String>,
    pub lines: Vec<PosSaleLineResponse>,
    pub created_at: DateTime<Utc>,
}

/// POS sale line response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PosSaleLineResponse {
    pub id: i64,
    pub product_id: Option<i64>,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub discount_amount: Decimal,
    pub line_total: Decimal,
}

/// Z-report response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ZReportResponse {
    pub id: i64,
    pub terminal_id: i64,
    pub report_number: String,
    pub report_date: DateTime<Utc>,
    pub status: String,
    pub total_sales: Decimal,
    pub total_cash: Decimal,
    pub total_card: Decimal,
    pub total_other: Decimal,
    pub total_tax: Decimal,
    pub total_discount: Decimal,
    pub transaction_count: u32,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<ZReport> for ZReportResponse {
    fn from(r: ZReport) -> Self {
        Self {
            id: r.id,
            terminal_id: r.terminal_id,
            report_number: r.report_number,
            report_date: r.report_date,
            status: r.status.to_string(),
            total_sales: r.total_sales,
            total_cash: r.total_cash,
            total_card: r.total_card,
            total_other: r.total_other,
            total_tax: r.total_tax,
            total_discount: r.total_discount,
            transaction_count: r.transaction_count,
            opened_at: r.opened_at,
            closed_at: r.closed_at,
            created_at: r.created_at,
        }
    }
}
