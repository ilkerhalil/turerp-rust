//! e-Defter domain models
//!
//! Provides types for Turkish e-Defter (electronic ledger) integration
//! with GIB (Gelir İdaresi Başkanlığı), including Yevmiye defteri,
//! Büyük defter, and Berat signing structures.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// e-Defter document status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema)]
pub enum EDefterStatus {
    #[default]
    Draft,
    Signed,
    Sent,
    Accepted,
    Rejected,
    Cancelled,
}

impl std::fmt::Display for EDefterStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EDefterStatus::Draft => write!(f, "Draft"),
            EDefterStatus::Signed => write!(f, "Signed"),
            EDefterStatus::Sent => write!(f, "Sent"),
            EDefterStatus::Accepted => write!(f, "Accepted"),
            EDefterStatus::Rejected => write!(f, "Rejected"),
            EDefterStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl std::str::FromStr for EDefterStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(EDefterStatus::Draft),
            "Signed" => Ok(EDefterStatus::Signed),
            "Sent" => Ok(EDefterStatus::Sent),
            "Accepted" => Ok(EDefterStatus::Accepted),
            "Rejected" => Ok(EDefterStatus::Rejected),
            "Cancelled" => Ok(EDefterStatus::Cancelled),
            _ => Err(format!("Invalid e-Defter status: {}", s)),
        }
    }
}

/// Ledger type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum LedgerType {
    YevmiyeDefteri,
    BuyukDefter,
    KebirDefter,
}

impl std::fmt::Display for LedgerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerType::YevmiyeDefteri => write!(f, "YevmiyeDefteri"),
            LedgerType::BuyukDefter => write!(f, "BuyukDefter"),
            LedgerType::KebirDefter => write!(f, "KebirDefter"),
        }
    }
}

impl std::str::FromStr for LedgerType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "YevmiyeDefteri" => Ok(LedgerType::YevmiyeDefteri),
            "BuyukDefter" => Ok(LedgerType::BuyukDefter),
            "KebirDefter" => Ok(LedgerType::KebirDefter),
            _ => Err(format!("Invalid ledger type: {}", s)),
        }
    }
}

/// Ledger period (monthly)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LedgerPeriod {
    pub id: i64,
    pub tenant_id: i64,
    pub year: i32,
    pub month: u32,
    pub period_type: LedgerType,
    pub status: EDefterStatus,
    pub berat_signed_at: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Yevmiye (journal) entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct YevmiyeEntry {
    pub id: i64,
    pub period_id: i64,
    pub entry_number: i64,
    pub entry_date: NaiveDate,
    pub explanation: String,
    pub debit_total: Decimal,
    pub credit_total: Decimal,
    pub lines: Vec<YevmiyeLine>,
}

/// Yevmiye entry line
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct YevmiyeLine {
    pub account_code: String,
    pub account_name: String,
    pub debit: Decimal,
    pub credit: Decimal,
    pub explanation: String,
}

/// Balance check result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BalanceCheckResult {
    pub is_balanced: bool,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub difference: Decimal,
    pub errors: Vec<String>,
}

/// Berat (certificate) information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BeratInfo {
    pub period_id: i64,
    pub serial_number: String,
    pub sign_time: DateTime<Utc>,
    pub signer: String,
    pub digest_value: String,
    pub signature_value: String,
}

/// Create a new ledger period
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateLedgerPeriod {
    pub year: i32,
    pub month: u32,
    pub period_type: LedgerType,
}

/// Ledger period response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LedgerPeriodResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub year: i32,
    pub month: u32,
    pub period_type: LedgerType,
    pub status: EDefterStatus,
    pub berat_signed_at: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<LedgerPeriod> for LedgerPeriodResponse {
    fn from(p: LedgerPeriod) -> Self {
        Self {
            id: p.id,
            tenant_id: p.tenant_id,
            year: p.year,
            month: p.month,
            period_type: p.period_type,
            status: p.status,
            berat_signed_at: p.berat_signed_at,
            sent_at: p.sent_at,
            created_at: p.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edefter_status_display() {
        assert_eq!(EDefterStatus::Draft.to_string(), "Draft");
        assert_eq!(EDefterStatus::Signed.to_string(), "Signed");
        assert_eq!(EDefterStatus::Sent.to_string(), "Sent");
        assert_eq!(EDefterStatus::Accepted.to_string(), "Accepted");
        assert_eq!(EDefterStatus::Rejected.to_string(), "Rejected");
        assert_eq!(EDefterStatus::Cancelled.to_string(), "Cancelled");
    }

    #[test]
    fn test_edefter_status_from_str() {
        assert_eq!(
            "Draft".parse::<EDefterStatus>().unwrap(),
            EDefterStatus::Draft
        );
        assert_eq!(
            "Signed".parse::<EDefterStatus>().unwrap(),
            EDefterStatus::Signed
        );
        assert_eq!(
            "Sent".parse::<EDefterStatus>().unwrap(),
            EDefterStatus::Sent
        );
        assert_eq!(
            "Accepted".parse::<EDefterStatus>().unwrap(),
            EDefterStatus::Accepted
        );
        assert_eq!(
            "Rejected".parse::<EDefterStatus>().unwrap(),
            EDefterStatus::Rejected
        );
        assert_eq!(
            "Cancelled".parse::<EDefterStatus>().unwrap(),
            EDefterStatus::Cancelled
        );
        assert!("INVALID".parse::<EDefterStatus>().is_err());
    }

    #[test]
    fn test_ledger_type_display() {
        assert_eq!(LedgerType::YevmiyeDefteri.to_string(), "YevmiyeDefteri");
        assert_eq!(LedgerType::BuyukDefter.to_string(), "BuyukDefter");
        assert_eq!(LedgerType::KebirDefter.to_string(), "KebirDefter");
    }

    #[test]
    fn test_ledger_type_from_str() {
        assert_eq!(
            "YevmiyeDefteri".parse::<LedgerType>().unwrap(),
            LedgerType::YevmiyeDefteri
        );
        assert_eq!(
            "BuyukDefter".parse::<LedgerType>().unwrap(),
            LedgerType::BuyukDefter
        );
        assert_eq!(
            "KebirDefter".parse::<LedgerType>().unwrap(),
            LedgerType::KebirDefter
        );
        assert!("INVALID".parse::<LedgerType>().is_err());
    }

    #[test]
    fn test_ledger_period_response_from_ledger_period() {
        let now = Utc::now();
        let period = LedgerPeriod {
            id: 1,
            tenant_id: 100,
            year: 2024,
            month: 6,
            period_type: LedgerType::YevmiyeDefteri,
            status: EDefterStatus::Draft,
            berat_signed_at: None,
            sent_at: None,
            created_at: now,
        };

        let response = LedgerPeriodResponse::from(period);
        assert_eq!(response.id, 1);
        assert_eq!(response.tenant_id, 100);
        assert_eq!(response.year, 2024);
        assert_eq!(response.month, 6);
        assert_eq!(response.period_type, LedgerType::YevmiyeDefteri);
        assert_eq!(response.status, EDefterStatus::Draft);
        assert!(response.berat_signed_at.is_none());
        assert!(response.sent_at.is_none());
    }

    #[test]
    fn test_balance_check_result_balanced() {
        let result = BalanceCheckResult {
            is_balanced: true,
            total_debit: Decimal::new(10000, 2),
            total_credit: Decimal::new(10000, 2),
            difference: Decimal::ZERO,
            errors: vec![],
        };
        assert!(result.is_balanced);
        assert_eq!(result.difference, Decimal::ZERO);
    }

    #[test]
    fn test_balance_check_result_unbalanced() {
        let result = BalanceCheckResult {
            is_balanced: false,
            total_debit: Decimal::new(10000, 2),
            total_credit: Decimal::new(9000, 2),
            difference: Decimal::new(1000, 2),
            errors: vec!["Debit does not equal credit".to_string()],
        };
        assert!(!result.is_balanced);
        assert!(!result.errors.is_empty());
    }
}
