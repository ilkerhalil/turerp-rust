//! Accounting domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Account type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountType::Asset => write!(f, "Asset"),
            AccountType::Liability => write!(f, "Liability"),
            AccountType::Equity => write!(f, "Equity"),
            AccountType::Revenue => write!(f, "Revenue"),
            AccountType::Expense => write!(f, "Expense"),
        }
    }
}

impl std::str::FromStr for AccountType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Asset" => Ok(AccountType::Asset),
            "Liability" => Ok(AccountType::Liability),
            "Equity" => Ok(AccountType::Equity),
            "Revenue" => Ok(AccountType::Revenue),
            "Expense" => Ok(AccountType::Expense),
            _ => Err(format!("Invalid account type: {}", s)),
        }
    }
}

/// Account subtype
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccountSubType {
    // Asset
    CurrentAsset,
    FixedAsset,
    // Liability
    CurrentLiability,
    LongTermLiability,
    // Equity
    OwnersEquity,
    RetainedEarnings,
    // Revenue
    OperatingRevenue,
    NonOperatingRevenue,
    // Expense
    OperatingExpense,
    NonOperatingExpense,
}

impl std::fmt::Display for AccountSubType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountSubType::CurrentAsset => write!(f, "CurrentAsset"),
            AccountSubType::FixedAsset => write!(f, "FixedAsset"),
            AccountSubType::CurrentLiability => write!(f, "CurrentLiability"),
            AccountSubType::LongTermLiability => write!(f, "LongTermLiability"),
            AccountSubType::OwnersEquity => write!(f, "OwnersEquity"),
            AccountSubType::RetainedEarnings => write!(f, "RetainedEarnings"),
            AccountSubType::OperatingRevenue => write!(f, "OperatingRevenue"),
            AccountSubType::NonOperatingRevenue => write!(f, "NonOperatingRevenue"),
            AccountSubType::OperatingExpense => write!(f, "OperatingExpense"),
            AccountSubType::NonOperatingExpense => write!(f, "NonOperatingExpense"),
        }
    }
}

impl std::str::FromStr for AccountSubType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CurrentAsset" => Ok(AccountSubType::CurrentAsset),
            "FixedAsset" => Ok(AccountSubType::FixedAsset),
            "CurrentLiability" => Ok(AccountSubType::CurrentLiability),
            "LongTermLiability" => Ok(AccountSubType::LongTermLiability),
            "OwnersEquity" => Ok(AccountSubType::OwnersEquity),
            "RetainedEarnings" => Ok(AccountSubType::RetainedEarnings),
            "OperatingRevenue" => Ok(AccountSubType::OperatingRevenue),
            "NonOperatingRevenue" => Ok(AccountSubType::NonOperatingRevenue),
            "OperatingExpense" => Ok(AccountSubType::OperatingExpense),
            "NonOperatingExpense" => Ok(AccountSubType::NonOperatingExpense),
            _ => Err(format!("Invalid account sub type: {}", s)),
        }
    }
}

/// Account entity (Chart of Accounts)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub sub_type: AccountSubType,
    pub parent_id: Option<i64>,
    pub is_active: bool,
    pub allow_transaction: bool,
    pub created_at: DateTime<Utc>,
}

/// Journal entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: i64,
    pub tenant_id: i64,
    pub entry_number: String,
    pub date: DateTime<Utc>,
    pub description: String,
    pub reference: Option<String>,
    pub status: JournalEntryStatus,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub posted_at: Option<DateTime<Utc>>,
}

/// Journal entry status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JournalEntryStatus {
    Draft,
    Posted,
    Voided,
}

impl std::fmt::Display for JournalEntryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JournalEntryStatus::Draft => write!(f, "Draft"),
            JournalEntryStatus::Posted => write!(f, "Posted"),
            JournalEntryStatus::Voided => write!(f, "Voided"),
        }
    }
}

impl std::str::FromStr for JournalEntryStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(JournalEntryStatus::Draft),
            "Posted" => Ok(JournalEntryStatus::Posted),
            "Voided" => Ok(JournalEntryStatus::Voided),
            _ => Err(format!("Invalid journal entry status: {}", s)),
        }
    }
}

/// Journal line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalLine {
    pub id: i64,
    pub entry_id: i64,
    pub account_id: i64,
    pub debit: Decimal,
    pub credit: Decimal,
    pub description: Option<String>,
    pub reference: Option<String>,
}

/// Account balance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub account_id: i64,
    pub account_code: String,
    pub account_name: String,
    pub account_type: AccountType,
    pub debit_balance: Decimal,
    pub credit_balance: Decimal,
    pub balance: Decimal,
}

/// Trial balance report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialBalance {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub accounts: Vec<AccountBalance>,
    pub total_debits: Decimal,
    pub total_credits: Decimal,
}

/// Create account request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccount {
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub sub_type: AccountSubType,
    pub parent_id: Option<i64>,
    pub allow_transaction: bool,
}

impl CreateAccount {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.code.trim().is_empty() {
            errors.push("Account code is required".to_string());
        }
        if self.name.trim().is_empty() {
            errors.push("Account name is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create journal entry request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJournalEntry {
    pub tenant_id: i64,
    pub date: DateTime<Utc>,
    pub description: String,
    pub reference: Option<String>,
    pub lines: Vec<CreateJournalLine>,
    pub created_by: i64,
}

impl CreateJournalEntry {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.lines.is_empty() {
            errors.push("Journal entry must have at least one line".to_string());
        }
        let total_debit: Decimal = self.lines.iter().map(|l| l.debit).sum();
        let total_credit: Decimal = self.lines.iter().map(|l| l.credit).sum();
        if total_debit != total_credit {
            errors.push("Total debits must equal total credits".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create journal line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJournalLine {
    pub account_id: i64,
    pub debit: Decimal,
    pub credit: Decimal,
    pub description: Option<String>,
    pub reference: Option<String>,
}

impl CreateJournalLine {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.debit < Decimal::ZERO || self.credit < Decimal::ZERO {
            errors.push("Amounts cannot be negative".to_string());
        }
        if self.debit > Decimal::ZERO && self.credit > Decimal::ZERO {
            errors.push("Line cannot have both debit and credit".to_string());
        }
        if self.debit == Decimal::ZERO && self.credit == Decimal::ZERO {
            errors.push("Line must have either debit or credit".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_create_account_validation() {
        let valid = CreateAccount {
            tenant_id: 1,
            code: "1000".to_string(),
            name: "Cash".to_string(),
            account_type: AccountType::Asset,
            sub_type: AccountSubType::CurrentAsset,
            parent_id: None,
            allow_transaction: true,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateAccount {
            tenant_id: 1,
            code: "".to_string(),
            name: "".to_string(),
            account_type: AccountType::Asset,
            sub_type: AccountSubType::CurrentAsset,
            parent_id: None,
            allow_transaction: true,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_journal_entry_validation() {
        let valid = CreateJournalEntry {
            tenant_id: 1,
            date: Utc::now(),
            description: "Test entry".to_string(),
            reference: Some("REF001".to_string()),
            created_by: 1,
            lines: vec![
                CreateJournalLine {
                    account_id: 1,
                    debit: dec!(100.0),
                    credit: Decimal::ZERO,
                    description: None,
                    reference: None,
                },
                CreateJournalLine {
                    account_id: 2,
                    debit: Decimal::ZERO,
                    credit: dec!(100.0),
                    description: None,
                    reference: None,
                },
            ],
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateJournalEntry {
            tenant_id: 1,
            date: Utc::now(),
            description: "Test".to_string(),
            reference: None,
            created_by: 1,
            lines: vec![],
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_account_type_display() {
        assert_eq!(AccountType::Asset.to_string(), "Asset");
        assert_eq!(AccountType::Liability.to_string(), "Liability");
        assert_eq!(AccountType::Equity.to_string(), "Equity");
        assert_eq!(AccountType::Revenue.to_string(), "Revenue");
        assert_eq!(AccountType::Expense.to_string(), "Expense");
    }

    #[test]
    fn test_account_type_from_str() {
        use std::str::FromStr;
        assert_eq!(AccountType::from_str("Asset").unwrap(), AccountType::Asset);
        assert_eq!(
            AccountType::from_str("Liability").unwrap(),
            AccountType::Liability
        );
        assert!(AccountType::from_str("Invalid").is_err());
    }

    #[test]
    fn test_account_sub_type_display() {
        assert_eq!(AccountSubType::CurrentAsset.to_string(), "CurrentAsset");
        assert_eq!(AccountSubType::FixedAsset.to_string(), "FixedAsset");
        assert_eq!(
            AccountSubType::CurrentLiability.to_string(),
            "CurrentLiability"
        );
        assert_eq!(
            AccountSubType::LongTermLiability.to_string(),
            "LongTermLiability"
        );
        assert_eq!(AccountSubType::OwnersEquity.to_string(), "OwnersEquity");
        assert_eq!(
            AccountSubType::RetainedEarnings.to_string(),
            "RetainedEarnings"
        );
        assert_eq!(
            AccountSubType::OperatingRevenue.to_string(),
            "OperatingRevenue"
        );
        assert_eq!(
            AccountSubType::NonOperatingRevenue.to_string(),
            "NonOperatingRevenue"
        );
        assert_eq!(
            AccountSubType::OperatingExpense.to_string(),
            "OperatingExpense"
        );
        assert_eq!(
            AccountSubType::NonOperatingExpense.to_string(),
            "NonOperatingExpense"
        );
    }

    #[test]
    fn test_account_sub_type_from_str() {
        use std::str::FromStr;
        assert_eq!(
            AccountSubType::from_str("CurrentAsset").unwrap(),
            AccountSubType::CurrentAsset
        );
        assert_eq!(
            AccountSubType::from_str("FixedAsset").unwrap(),
            AccountSubType::FixedAsset
        );
        assert!(AccountSubType::from_str("Invalid").is_err());
    }

    #[test]
    fn test_journal_entry_status_display() {
        assert_eq!(JournalEntryStatus::Draft.to_string(), "Draft");
        assert_eq!(JournalEntryStatus::Posted.to_string(), "Posted");
        assert_eq!(JournalEntryStatus::Voided.to_string(), "Voided");
    }

    #[test]
    fn test_journal_entry_status_from_str() {
        use std::str::FromStr;
        assert_eq!(
            JournalEntryStatus::from_str("Draft").unwrap(),
            JournalEntryStatus::Draft
        );
        assert_eq!(
            JournalEntryStatus::from_str("Posted").unwrap(),
            JournalEntryStatus::Posted
        );
        assert_eq!(
            JournalEntryStatus::from_str("Voided").unwrap(),
            JournalEntryStatus::Voided
        );
        assert!(JournalEntryStatus::from_str("Invalid").is_err());
    }
}
