//! Accounting service for business logic
use chrono::Utc;
use rust_decimal::Decimal;

use crate::common::pagination::PaginatedResult;
use crate::domain::accounting::model::{
    Account, AccountBalance, AccountType, CreateAccount, CreateJournalEntry, JournalEntry,
    JournalEntryStatus, TrialBalance,
};
use crate::domain::accounting::repository::{
    BoxAccountRepository, BoxJournalEntryRepository, BoxJournalLineRepository,
};
use crate::domain::company::service::ensure_company_owned;
use crate::domain::company::BoxCompanyRepository;
use crate::error::ApiError;

/// Accounting service
#[derive(Clone)]
pub struct AccountingService {
    account_repo: BoxAccountRepository,
    entry_repo: BoxJournalEntryRepository,
    line_repo: BoxJournalLineRepository,
    company_repo: BoxCompanyRepository,
}

impl AccountingService {
    pub fn new(
        account_repo: BoxAccountRepository,
        entry_repo: BoxJournalEntryRepository,
        line_repo: BoxJournalLineRepository,
        company_repo: BoxCompanyRepository,
    ) -> Self {
        Self {
            account_repo,
            entry_repo,
            line_repo,
            company_repo,
        }
    }

    // Account operations
    #[tracing::instrument(skip(self))]
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
        // Parent-ownership precheck: body company_id must belong to the caller's
        // tenant (legacy `1` sentinel skipped for backward compat).
        ensure_company_owned(&self.company_repo, create.company_id, create.tenant_id).await?;
        self.account_repo.create(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_account(&self, id: i64, tenant_id: i64) -> Result<Account, ApiError> {
        self.account_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Account {} not found", id)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_accounts_by_tenant(&self, tenant_id: i64) -> Result<Vec<Account>, ApiError> {
        self.account_repo.find_by_tenant(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_accounts_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Account>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.account_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self))]
    pub async fn create_journal_entry(
        &self,
        create: CreateJournalEntry,
    ) -> Result<JournalEntry, ApiError> {
        // Parent-ownership precheck: body company_id must belong to the caller's
        // tenant (legacy `1` sentinel skipped for backward compat).
        ensure_company_owned(&self.company_repo, create.company_id, create.tenant_id).await?;
        let entry = self.entry_repo.create(create.clone()).await?;
        let _lines = self
            .line_repo
            .create_many(entry.id, create.lines, create.tenant_id)
            .await?;
        Ok(entry)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_journal_entry(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<JournalEntry, ApiError> {
        self.entry_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Journal entry {} not found", id)))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_journal_entries_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<JournalEntry>, ApiError> {
        self.entry_repo.find_by_tenant(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_journal_entries_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<JournalEntry>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.entry_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
    }

    #[tracing::instrument(skip(self))]
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

    #[tracing::instrument(skip(self))]
    pub async fn post_journal_entry(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<JournalEntry, ApiError> {
        self.entry_repo.post(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn void_journal_entry(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<JournalEntry, ApiError> {
        self.entry_repo.void(id, tenant_id).await
    }

    // Soft delete operations
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_account(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.account_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_account(&self, id: i64, tenant_id: i64) -> Result<Account, ApiError> {
        self.account_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_accounts(&self, tenant_id: i64) -> Result<Vec<Account>, ApiError> {
        self.account_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_account(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.account_repo.destroy(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_journal_entry(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.entry_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_journal_entry(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<JournalEntry, ApiError> {
        self.entry_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_journal_entries(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<JournalEntry>, ApiError> {
        self.entry_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_journal_entry(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.entry_repo.destroy(id, tenant_id).await
    }

    // Reports
    #[tracing::instrument(skip(self))]
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
                let lines = self.line_repo.find_by_entry(entry.id, tenant_id).await?;
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
    use crate::domain::accounting::model::{AccountSubType, CreateJournalLine};
    use crate::domain::accounting::repository::{
        InMemoryAccountRepository, InMemoryJournalEntryRepository, InMemoryJournalLineRepository,
    };
    use crate::domain::company::repository::InMemoryCompanyRepository;
    use crate::domain::company::service::LEGACY_COMPANY_ID;
    use crate::domain::company::CreateCompany;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    async fn create_service() -> AccountingService {
        let account_repo = Arc::new(InMemoryAccountRepository::new()) as BoxAccountRepository;
        let entry_repo =
            Arc::new(InMemoryJournalEntryRepository::new()) as BoxJournalEntryRepository;
        let line_repo = Arc::new(InMemoryJournalLineRepository::new()) as BoxJournalLineRepository;
        let company_repo = Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository;
        // Seed a company for each tenant so the InMemory auto-id counter yields
        // id=1 for tenant-1 (the LEGACY_COMPANY_ID sentinel, skipped by the
        // precheck) and id=2 for tenant-2 (a non-sentinel foreign company the
        // reject tests target).
        for tenant in [1, 2] {
            company_repo
                .create(CreateCompany {
                    code: format!("CO{}", tenant),
                    name: format!("Tenant {} Co", tenant),
                    tax_number: None,
                    address: None,
                    city: None,
                    country: None,
                    currency: "TRY".to_string(),
                    tenant_id: tenant,
                })
                .await
                .expect("seed company");
        }
        AccountingService::new(account_repo, entry_repo, line_repo, company_repo)
    }

    /// Returns the tenant-2 company id (a non-sentinel foreign company) for the
    /// reject tests, guarding that the seeded id is not the LEGACY sentinel.
    async fn foreign_company_id(service: &AccountingService) -> i64 {
        let id = service
            .company_repo
            .find_by_tenant(2)
            .await
            .expect("list tenant-2 companies")
            .into_iter()
            .map(|c| c.id)
            .next()
            .expect("tenant-2 company seeded");
        assert_ne!(id, LEGACY_COMPANY_ID);
        id
    }

    /// Compact balanced two-line journal entry (debit/credit) for the tests.
    fn jl(account_id: i64, debit: Decimal, credit: Decimal) -> CreateJournalLine {
        CreateJournalLine {
            account_id,
            cost_center_id: None,
            debit,
            credit,
            description: None,
            reference: None,
        }
    }

    #[tokio::test]
    async fn test_create_account() {
        let service = create_service().await;
        let create = CreateAccount {
            tenant_id: 1,
            company_id: 1,
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
        let service = create_service().await;
        let create = CreateJournalEntry {
            tenant_id: 1,
            company_id: 1,
            date: Utc::now(),
            description: "Record sale".to_string(),
            reference: Some("INV001".to_string()),
            created_by: 1,
            lines: vec![
                jl(1, Decimal::ZERO, dec!(100.0)),
                jl(8, dec!(100.0), Decimal::ZERO),
            ],
        };
        let result = service.create_journal_entry(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_post_journal_entry() {
        let service = create_service().await;
        let create = CreateJournalEntry {
            tenant_id: 1,
            company_id: 1,
            date: Utc::now(),
            description: "Test".to_string(),
            reference: None,
            created_by: 1,
            lines: vec![
                jl(1, dec!(50.0), Decimal::ZERO),
                jl(11, Decimal::ZERO, dec!(50.0)),
            ],
        };
        let entry = service.create_journal_entry(create).await.unwrap();
        let posted = service.post_journal_entry(entry.id, 1).await.unwrap();
        assert_eq!(posted.status, JournalEntryStatus::Posted);
    }

    /// Rejects a chart-of-accounts entry stamped onto a foreign-tenant company.
    #[tokio::test]
    async fn test_create_account_rejects_foreign_company() {
        let service = create_service().await;
        let foreign = foreign_company_id(&service).await;
        let create = CreateAccount {
            tenant_id: 1,
            company_id: foreign,
            code: "1300".to_string(),
            name: "Foreign-stamped account".to_string(),
            account_type: AccountType::Asset,
            sub_type: AccountSubType::CurrentAsset,
            parent_id: None,
            allow_transaction: true,
        };
        let result = service.create_account(create).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "expected NotFound for foreign company_id, got {:?}",
            result
        );
    }

    /// Rejects a journal entry stamped onto a foreign-tenant company.
    #[tokio::test]
    async fn test_create_journal_entry_rejects_foreign_company() {
        let service = create_service().await;
        let foreign = foreign_company_id(&service).await;
        let create = CreateJournalEntry {
            tenant_id: 1,
            company_id: foreign,
            date: Utc::now(),
            description: "Foreign-stamped entry".to_string(),
            reference: None,
            created_by: 1,
            lines: vec![
                jl(1, dec!(100.0), Decimal::ZERO),
                jl(8, Decimal::ZERO, dec!(100.0)),
            ],
        };
        let result = service.create_journal_entry(create).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "expected NotFound for foreign company_id, got {:?}",
            result
        );
    }
}
