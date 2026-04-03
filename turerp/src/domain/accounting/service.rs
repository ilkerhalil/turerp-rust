//! Accounting service for business logic
use chrono::Utc;
use rust_decimal::Decimal;

use crate::domain::accounting::model::{
    Account, AccountBalance, AccountType, CreateAccount, CreateJournalEntry, JournalEntry,
    JournalEntryStatus, TrialBalance,
};
use crate::domain::accounting::repository::{
    BoxAccountRepository, BoxJournalEntryRepository, BoxJournalLineRepository,
};
use crate::error::ApiError;

/// Accounting service
#[derive(Clone)]
pub struct AccountingService {
    account_repo: BoxAccountRepository,
    entry_repo: BoxJournalEntryRepository,
    line_repo: BoxJournalLineRepository,
}

impl AccountingService {
    pub fn new(
        account_repo: BoxAccountRepository,
        entry_repo: BoxJournalEntryRepository,
        line_repo: BoxJournalLineRepository,
    ) -> Self {
        Self {
            account_repo,
            entry_repo,
            line_repo,
        }
    }

    // Account operations
    pub async fn create_account(&self, create: CreateAccount) -> Result<Account, ApiError> {
        // Check if code exists
        if self
            .account_repo
            .find_by_code(create.tenant_id, &create.code)
            .await?
            .is_some()
        {
            return Err(ApiError::Conflict(format!(
                "Account code '{}' already exists",
                create.code
            )));
        }
        self.account_repo.create(create).await
    }

    pub async fn get_account(&self, id: i64) -> Result<Account, ApiError> {
        self.account_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Account {} not found", id)))
    }

    pub async fn get_accounts_by_tenant(&self, tenant_id: i64) -> Result<Vec<Account>, ApiError> {
        self.account_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_accounts_by_type(
        &self,
        tenant_id: i64,
        account_type: AccountType,
    ) -> Result<Vec<Account>, ApiError> {
        self.account_repo
            .find_by_type(tenant_id, account_type)
            .await
    }

    // Journal entry operations
    pub async fn create_journal_entry(
        &self,
        create: CreateJournalEntry,
    ) -> Result<JournalEntry, ApiError> {
        let entry = self.entry_repo.create(create.clone()).await?;
        let _lines = self.line_repo.create_many(entry.id, create.lines).await?;
        Ok(entry)
    }

    pub async fn get_journal_entry(&self, id: i64) -> Result<JournalEntry, ApiError> {
        self.entry_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Journal entry {} not found", id)))
    }

    pub async fn get_journal_entries_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<JournalEntry>, ApiError> {
        self.entry_repo.find_by_tenant(tenant_id).await
    }

    pub async fn get_journal_entries_by_date_range(
        &self,
        tenant_id: i64,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<Vec<JournalEntry>, ApiError> {
        self.entry_repo
            .find_by_date_range(tenant_id, start, end)
            .await
    }

    pub async fn post_journal_entry(&self, id: i64) -> Result<JournalEntry, ApiError> {
        self.entry_repo.post(id).await
    }

    pub async fn void_journal_entry(&self, id: i64) -> Result<JournalEntry, ApiError> {
        self.entry_repo.void(id).await
    }

    // Reports
    pub async fn generate_trial_balance(
        &self,
        tenant_id: i64,
        period_start: chrono::DateTime<Utc>,
        period_end: chrono::DateTime<Utc>,
    ) -> Result<TrialBalance, ApiError> {
        // Get all accounts
        let accounts = self.account_repo.find_by_tenant(tenant_id).await?;

        // Get posted journal entries in period
        let entries = self
            .entry_repo
            .find_by_date_range(tenant_id, period_start, period_end)
            .await?;
        let posted_entries: Vec<_> = entries
            .into_iter()
            .filter(|e| e.status == JournalEntryStatus::Posted)
            .collect();

        // Calculate balances
        let mut account_balances: Vec<AccountBalance> = Vec::new();

        for account in accounts {
            let mut debit_total = Decimal::ZERO;
            let mut credit_total = Decimal::ZERO;

            for entry in &posted_entries {
                let lines = self.line_repo.find_by_entry(entry.id).await?;
                for line in lines {
                    if line.account_id == account.id {
                        debit_total += line.debit;
                        credit_total += line.credit;
                    }
                }
            }

            let balance = match account.account_type {
                AccountType::Asset | AccountType::Expense => debit_total - credit_total,
                AccountType::Liability | AccountType::Revenue | AccountType::Equity => {
                    credit_total - debit_total
                }
            };

            account_balances.push(AccountBalance {
                account_id: account.id,
                account_code: account.code.clone(),
                account_name: account.name.clone(),
                account_type: account.account_type.clone(),
                debit_balance: debit_total,
                credit_balance: credit_total,
                balance,
            });
        }

        let total_debits: Decimal = account_balances.iter().map(|b| b.debit_balance).sum();
        let total_credits: Decimal = account_balances.iter().map(|b| b.credit_balance).sum();

        Ok(TrialBalance {
            period_start,
            period_end,
            accounts: account_balances,
            total_debits,
            total_credits,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::accounting::model::AccountSubType;
    use crate::domain::accounting::repository::{
        InMemoryAccountRepository, InMemoryJournalEntryRepository, InMemoryJournalLineRepository,
    };
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_service() -> AccountingService {
        let account_repo = Arc::new(InMemoryAccountRepository::new()) as BoxAccountRepository;
        let entry_repo =
            Arc::new(InMemoryJournalEntryRepository::new()) as BoxJournalEntryRepository;
        let line_repo = Arc::new(InMemoryJournalLineRepository::new()) as BoxJournalLineRepository;
        AccountingService::new(account_repo, entry_repo, line_repo)
    }

    #[tokio::test]
    async fn test_create_account() {
        let service = create_service();
        let create = CreateAccount {
            tenant_id: 1,
            code: "1200".to_string(),
            name: "Inventory".to_string(),
            account_type: AccountType::Asset,
            sub_type: AccountSubType::CurrentAsset,
            parent_id: None,
            allow_transaction: true,
        };
        let result = service.create_account(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_journal_entry() {
        let service = create_service();
        let create = CreateJournalEntry {
            tenant_id: 1,
            date: Utc::now(),
            description: "Record sale".to_string(),
            reference: Some("INV001".to_string()),
            created_by: 1,
            lines: vec![
                crate::domain::accounting::model::CreateJournalLine {
                    account_id: 1,
                    debit: Decimal::ZERO,
                    credit: dec!(100.0),
                    description: None,
                    reference: None,
                },
                crate::domain::accounting::model::CreateJournalLine {
                    account_id: 8,
                    debit: dec!(100.0),
                    credit: Decimal::ZERO,
                    description: None,
                    reference: None,
                },
            ],
        };
        let result = service.create_journal_entry(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_journal_entry() {
        let service = create_service();
        let create = CreateJournalEntry {
            tenant_id: 1,
            date: Utc::now(),
            description: "Test".to_string(),
            reference: None,
            created_by: 1,
            lines: vec![
                crate::domain::accounting::model::CreateJournalLine {
                    account_id: 1,
                    debit: dec!(50.0),
                    credit: Decimal::ZERO,
                    description: None,
                    reference: None,
                },
                crate::domain::accounting::model::CreateJournalLine {
                    account_id: 11,
                    debit: Decimal::ZERO,
                    credit: dec!(50.0),
                    description: None,
                    reference: None,
                },
            ],
        };
        let entry = service.create_journal_entry(create).await.unwrap();
        let posted = service.post_journal_entry(entry.id).await.unwrap();
        assert_eq!(posted.status, JournalEntryStatus::Posted);
    }
}
