//! Bank integration domain model

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Turkish bank codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum BankCode {
    #[default]
    Halkbank,
    Ziraat,
    IsBankasi,
    Garanti,
    YapiKredi,
    Akbank,
}

impl std::fmt::Display for BankCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BankCode::Halkbank => write!(f, "halkbank"),
            BankCode::Ziraat => write!(f, "ziraat"),
            BankCode::IsBankasi => write!(f, "isbankasi"),
            BankCode::Garanti => write!(f, "garanti"),
            BankCode::YapiKredi => write!(f, "yapikredi"),
            BankCode::Akbank => write!(f, "akbank"),
        }
    }
}

impl std::str::FromStr for BankCode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "halkbank" => Ok(BankCode::Halkbank),
            "ziraat" => Ok(BankCode::Ziraat),
            "isbankasi" => Ok(BankCode::IsBankasi),
            "garanti" => Ok(BankCode::Garanti),
            "yapikredi" => Ok(BankCode::YapiKredi),
            "akbank" => Ok(BankCode::Akbank),
            _ => Err(format!("Invalid bank code: {}", s)),
        }
    }
}

/// Transaction match status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum MatchStatus {
    #[default]
    Unmatched,
    Matched,
    Manual,
}

impl std::fmt::Display for MatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchStatus::Unmatched => write!(f, "unmatched"),
            MatchStatus::Matched => write!(f, "matched"),
            MatchStatus::Manual => write!(f, "manual"),
        }
    }
}

impl std::str::FromStr for MatchStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unmatched" => Ok(MatchStatus::Unmatched),
            "matched" => Ok(MatchStatus::Matched),
            "manual" => Ok(MatchStatus::Manual),
            _ => Err(format!("Invalid match status: {}", s)),
        }
    }
}

/// Statement format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum StatementFormat {
    #[default]
    Mt940,
    Xml,
    Camt053,
}

impl std::fmt::Display for StatementFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatementFormat::Mt940 => write!(f, "mt940"),
            StatementFormat::Xml => write!(f, "xml"),
            StatementFormat::Camt053 => write!(f, "camt053"),
        }
    }
}

impl std::str::FromStr for StatementFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mt940" | "mt-940" => Ok(StatementFormat::Mt940),
            "xml" => Ok(StatementFormat::Xml),
            "camt053" | "camt.053" => Ok(StatementFormat::Camt053),
            _ => Err(format!("Invalid statement format: {}", s)),
        }
    }
}

/// Match field for reconciliation rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum MatchField {
    #[default]
    Description,
    Amount,
    Reference,
}

impl std::fmt::Display for MatchField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MatchField::Description => write!(f, "description"),
            MatchField::Amount => write!(f, "amount"),
            MatchField::Reference => write!(f, "reference"),
        }
    }
}

impl std::str::FromStr for MatchField {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "description" => Ok(MatchField::Description),
            "amount" => Ok(MatchField::Amount),
            "reference" => Ok(MatchField::Reference),
            _ => Err(format!("Invalid match field: {}", s)),
        }
    }
}

/// Bank account entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BankAccount {
    pub id: i64,
    pub tenant_id: i64,
    pub company_id: Option<i64>,
    pub bank_code: BankCode,
    pub account_number: String,
    pub iban: Option<String>,
    pub account_name: String,
    pub currency: String,
    pub branch_code: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for BankAccount {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

/// Bank account response (without sensitive internal data)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BankAccountResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub company_id: Option<i64>,
    pub bank_code: BankCode,
    pub account_number: String,
    pub iban: Option<String>,
    pub account_name: String,
    pub currency: String,
    pub branch_code: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<BankAccount> for BankAccountResponse {
    fn from(account: BankAccount) -> Self {
        Self {
            id: account.id,
            tenant_id: account.tenant_id,
            company_id: account.company_id,
            bank_code: account.bank_code,
            account_number: account.account_number,
            iban: account.iban,
            account_name: account.account_name,
            currency: account.currency,
            branch_code: account.branch_code,
            is_active: account.is_active,
            created_at: account.created_at,
            updated_at: account.updated_at,
        }
    }
}

/// Data for creating a new bank account
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateBankAccount {
    #[validate(length(min = 1, max = 50))]
    pub bank_code: String,

    #[validate(length(min = 1, max = 100))]
    pub account_number: String,

    #[validate(length(min = 1, max = 200))]
    pub account_name: String,

    #[validate(length(min = 3, max = 3))]
    #[serde(default = "default_currency")]
    pub currency: String,

    #[validate(length(min = 1, max = 34))]
    #[serde(default)]
    pub iban: Option<String>,

    #[validate(length(max = 50))]
    #[serde(default)]
    pub branch_code: Option<String>,

    #[serde(default = "default_is_active")]
    pub is_active: bool,

    pub tenant_id: i64,
    pub company_id: Option<i64>,
}

fn default_currency() -> String {
    "TRY".to_string()
}

fn default_is_active() -> bool {
    true
}

/// Data for updating an existing bank account
#[derive(Debug, Clone, Deserialize, Serialize, Default, Validate, ToSchema)]
pub struct UpdateBankAccount {
    #[validate(length(min = 1, max = 100))]
    #[serde(default)]
    pub account_number: Option<String>,

    #[validate(length(min = 1, max = 200))]
    #[serde(default)]
    pub account_name: Option<String>,

    #[validate(length(min = 3, max = 3))]
    #[serde(default)]
    pub currency: Option<String>,

    #[validate(length(min = 1, max = 34))]
    #[serde(default)]
    pub iban: Option<String>,

    #[validate(length(max = 50))]
    #[serde(default)]
    pub branch_code: Option<String>,

    #[serde(default)]
    pub is_active: Option<bool>,

    #[serde(default)]
    pub company_id: Option<i64>,
}

/// Bank statement entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BankStatement {
    pub id: i64,
    pub tenant_id: i64,
    pub account_id: i64,
    pub statement_date: NaiveDate,
    pub format: StatementFormat,
    pub raw_data: String,
    pub processed: bool,
    pub created_at: DateTime<Utc>,
}

/// Data for importing a bank statement
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct ImportBankStatement {
    #[serde(default)]
    pub format: StatementFormat,
    pub data: String,
}

/// Bank transaction entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BankTransaction {
    pub id: i64,
    pub tenant_id: i64,
    pub account_id: i64,
    pub transaction_date: NaiveDate,
    pub description: String,
    pub amount: Decimal,
    pub currency: String,
    pub balance_after: Option<Decimal>,
    pub reference_no: Option<String>,
    pub matched_invoice_id: Option<i64>,
    pub matched_payment_id: Option<i64>,
    pub match_status: MatchStatus,
    pub created_at: DateTime<Utc>,
}

/// Bank transaction response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BankTransactionResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub account_id: i64,
    pub transaction_date: NaiveDate,
    pub description: String,
    pub amount: Decimal,
    pub currency: String,
    pub balance_after: Option<Decimal>,
    pub reference_no: Option<String>,
    pub matched_invoice_id: Option<i64>,
    pub matched_payment_id: Option<i64>,
    pub match_status: MatchStatus,
    pub created_at: DateTime<Utc>,
}

impl From<BankTransaction> for BankTransactionResponse {
    fn from(tx: BankTransaction) -> Self {
        Self {
            id: tx.id,
            tenant_id: tx.tenant_id,
            account_id: tx.account_id,
            transaction_date: tx.transaction_date,
            description: tx.description,
            amount: tx.amount,
            currency: tx.currency,
            balance_after: tx.balance_after,
            reference_no: tx.reference_no,
            matched_invoice_id: tx.matched_invoice_id,
            matched_payment_id: tx.matched_payment_id,
            match_status: tx.match_status,
            created_at: tx.created_at,
        }
    }
}

/// Data for manually matching a transaction
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct MatchTransaction {
    pub invoice_id: Option<i64>,
    pub payment_id: Option<i64>,
}

/// Reconciliation rule entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReconciliationRule {
    pub id: i64,
    pub tenant_id: i64,
    pub rule_name: String,
    pub match_field: MatchField,
    pub match_pattern: String,
    pub auto_match: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for ReconciliationRule {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

/// Data for creating a reconciliation rule
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateReconciliationRule {
    #[validate(length(min = 1, max = 200))]
    pub rule_name: String,

    pub match_field: MatchField,

    #[validate(length(min = 1, max = 500))]
    pub match_pattern: String,

    #[serde(default)]
    pub auto_match: bool,

    #[serde(default = "default_is_active")]
    pub is_active: bool,

    pub tenant_id: i64,
}

/// Data for updating a reconciliation rule
#[derive(Debug, Clone, Deserialize, Serialize, Default, Validate, ToSchema)]
pub struct UpdateReconciliationRule {
    #[validate(length(min = 1, max = 200))]
    #[serde(default)]
    pub rule_name: Option<String>,

    #[serde(default)]
    pub match_field: Option<MatchField>,

    #[validate(length(min = 1, max = 500))]
    #[serde(default)]
    pub match_pattern: Option<String>,

    #[serde(default)]
    pub auto_match: Option<bool>,

    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Reconciliation report summary
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReconciliationReport {
    pub tenant_id: i64,
    pub total_transactions: i64,
    pub matched_count: i64,
    pub unmatched_count: i64,
    pub manual_count: i64,
    pub total_amount: Decimal,
    pub matched_amount: Decimal,
    pub unmatched_amount: Decimal,
}

/// Parsed bank transaction (used by parsers before persistence)
#[derive(Debug, Clone, Default)]
pub struct ParsedBankTransaction {
    pub transaction_date: NaiveDate,
    pub description: String,
    pub amount: Decimal,
    pub currency: String,
    pub balance_after: Option<Decimal>,
    pub reference_no: Option<String>,
}

/// Payment type for Turkish banking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PaymentType {
    #[default]
    Havale,
    Eft,
    Fast,
}

impl std::fmt::Display for PaymentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentType::Havale => write!(f, "havale"),
            PaymentType::Eft => write!(f, "eft"),
            PaymentType::Fast => write!(f, "fast"),
        }
    }
}

impl std::str::FromStr for PaymentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "havale" => Ok(PaymentType::Havale),
            "eft" => Ok(PaymentType::Eft),
            "fast" => Ok(PaymentType::Fast),
            _ => Err(format!("Invalid payment type: {}", s)),
        }
    }
}

/// Payment status in bank processing lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PaymentStatus {
    #[default]
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Rejected,
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PaymentStatus::Pending => write!(f, "pending"),
            PaymentStatus::Processing => write!(f, "processing"),
            PaymentStatus::Completed => write!(f, "completed"),
            PaymentStatus::Failed => write!(f, "failed"),
            PaymentStatus::Cancelled => write!(f, "cancelled"),
            PaymentStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl std::str::FromStr for PaymentStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(PaymentStatus::Pending),
            "processing" => Ok(PaymentStatus::Processing),
            "completed" => Ok(PaymentStatus::Completed),
            "failed" => Ok(PaymentStatus::Failed),
            "cancelled" => Ok(PaymentStatus::Cancelled),
            "rejected" => Ok(PaymentStatus::Rejected),
            _ => Err(format!("Invalid payment status: {}", s)),
        }
    }
}

/// Bank API connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum BankConnectionStatus {
    #[default]
    Disconnected,
    Connected,
    Error,
}

/// Data for initiating a payment through bank API
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct PaymentInitiation {
    pub source_account_id: i64,

    #[validate(length(min = 1, max = 34))]
    pub destination_iban: Option<String>,

    #[validate(length(min = 1, max = 100))]
    pub destination_account_no: Option<String>,

    #[validate(length(min = 1, max = 200))]
    pub beneficiary_name: String,

    pub amount: Decimal,

    #[validate(length(min = 3, max = 3))]
    #[serde(default = "default_currency")]
    pub currency: String,

    #[validate(length(max = 500))]
    #[serde(default)]
    pub description: Option<String>,

    pub payment_type: PaymentType,

    pub tenant_id: i64,
}

/// Response after initiating a payment
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentInitiationResponse {
    pub tracking_id: String,
    pub status: PaymentStatus,
    pub bank_reference: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Bank API credentials for connecting to a bank
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BankApiCredentials {
    pub bank_code: BankCode,
    pub api_key: String,
    pub api_secret: String,
    pub base_url: String,
    pub client_id: Option<String>,
}

/// Parsed CAMT.053 statement
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CamtStatement {
    pub statement_id: String,
    pub creation_date: DateTime<Utc>,
    pub account_iban: String,
    pub entries: Vec<CamtEntry>,
}

/// Individual entry in a CAMT.053 statement
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CamtEntry {
    pub entry_date: NaiveDate,
    pub amount: Decimal,
    pub currency: String,
    pub credit_debit: String,
    pub reference: Option<String>,
    pub description: Option<String>,
    pub counterparty_name: Option<String>,
    pub counterparty_iban: Option<String>,
}

/// Data for checking payment status
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CheckPaymentStatus {
    pub tracking_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::SoftDeletable;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    #[test]
    fn test_bank_code_display() {
        assert_eq!(BankCode::Halkbank.to_string(), "halkbank");
        assert_eq!(BankCode::Ziraat.to_string(), "ziraat");
        assert_eq!(BankCode::IsBankasi.to_string(), "isbankasi");
    }

    #[test]
    fn test_bank_code_from_str() {
        assert_eq!(BankCode::from_str("halkbank").unwrap(), BankCode::Halkbank);
        assert_eq!(BankCode::from_str("GARANTI").unwrap(), BankCode::Garanti);
        assert!(BankCode::from_str("unknown").is_err());
    }

    #[test]
    fn test_match_status_display() {
        assert_eq!(MatchStatus::Unmatched.to_string(), "unmatched");
        assert_eq!(MatchStatus::Matched.to_string(), "matched");
        assert_eq!(MatchStatus::Manual.to_string(), "manual");
    }

    #[test]
    fn test_statement_format_from_str() {
        assert_eq!(
            StatementFormat::from_str("mt940").unwrap(),
            StatementFormat::Mt940
        );
        assert_eq!(
            StatementFormat::from_str("camt053").unwrap(),
            StatementFormat::Camt053
        );
        assert_eq!(
            StatementFormat::from_str("xml").unwrap(),
            StatementFormat::Xml
        );
    }

    #[test]
    fn test_create_bank_account_validation() {
        let create = CreateBankAccount {
            bank_code: "garanti".to_string(),
            account_number: "12345678".to_string(),
            account_name: "Main Account".to_string(),
            currency: "TRY".to_string(),
            iban: Some("TR000123456789012345678901".to_string()),
            branch_code: Some("001".to_string()),
            is_active: true,
            tenant_id: 1,
            company_id: None,
        };

        assert!(create.validate().is_ok());
    }

    #[test]
    fn test_payment_type_from_str() {
        assert_eq!(
            PaymentType::from_str("havale").unwrap(),
            PaymentType::Havale
        );
        assert_eq!(PaymentType::from_str("EFT").unwrap(), PaymentType::Eft);
        assert_eq!(PaymentType::from_str("fast").unwrap(), PaymentType::Fast);
        assert!(PaymentType::from_str("unknown").is_err());
    }

    #[test]
    fn test_payment_status_from_str() {
        assert_eq!(
            PaymentStatus::from_str("pending").unwrap(),
            PaymentStatus::Pending
        );
        assert_eq!(
            PaymentStatus::from_str("completed").unwrap(),
            PaymentStatus::Completed
        );
        assert_eq!(
            PaymentStatus::from_str("rejected").unwrap(),
            PaymentStatus::Rejected
        );
        assert!(PaymentStatus::from_str("unknown").is_err());
    }

    #[test]
    fn test_payment_initiation_validation() {
        let initiation = PaymentInitiation {
            source_account_id: 1,
            destination_iban: Some("TR000123456789012345678901".to_string()),
            destination_account_no: None,
            beneficiary_name: "Test Recipient".to_string(),
            amount: dec!(1000.00),
            currency: "TRY".to_string(),
            description: Some("Test payment".to_string()),
            payment_type: PaymentType::Havale,
            tenant_id: 1,
        };
        assert!(initiation.validate().is_ok());
    }

    #[test]
    fn test_camt_statement_serialization() {
        let statement = CamtStatement {
            statement_id: "stmt-001".to_string(),
            creation_date: Utc::now(),
            account_iban: "TR000123456789012345678901".to_string(),
            entries: vec![CamtEntry {
                entry_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
                amount: dec!(500.00),
                currency: "TRY".to_string(),
                credit_debit: "CRDT".to_string(),
                reference: Some("REF-001".to_string()),
                description: Some("Invoice payment".to_string()),
                counterparty_name: Some("ABC Ltd".to_string()),
                counterparty_iban: None,
            }],
        };

        let json = serde_json::to_string(&statement).unwrap();
        assert!(json.contains("stmt-001"));
        assert!(json.contains("TR000123456789012345678901"));
    }

    #[test]
    fn test_bank_account_soft_delete() {
        let mut account = BankAccount {
            id: 1,
            tenant_id: 1,
            company_id: None,
            bank_code: BankCode::Ziraat,
            account_number: "12345678".to_string(),
            iban: None,
            account_name: "Test".to_string(),
            currency: "TRY".to_string(),
            branch_code: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };

        assert!(!account.is_deleted());
        account.mark_deleted(1);
        assert!(account.is_deleted());
        account.restore();
        assert!(!account.is_deleted());
    }
}
