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
}
