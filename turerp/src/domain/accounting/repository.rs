//! Accounting repository

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::domain::accounting::model::{
    Account, AccountSubType, AccountType, CreateAccount, CreateJournalEntry, CreateJournalLine,
    JournalEntry, JournalEntryStatus, JournalLine,
};
use crate::error::ApiError;

/// Repository trait for Account operations
#[async_trait]
pub trait AccountRepository: Send + Sync {
    async fn create(&self, account: CreateAccount) -> Result<Account, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Account>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Account>, ApiError>;
    async fn find_by_code(&self, tenant_id: i64, code: &str) -> Result<Option<Account>, ApiError>;
    async fn find_by_type(
        &self,
        tenant_id: i64,
        account_type: AccountType,
    ) -> Result<Vec<Account>, ApiError>;
    async fn update(
        &self,
        id: i64,
        name: Option<String>,
        is_active: Option<bool>,
    ) -> Result<Account, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for JournalEntry operations
#[async_trait]
pub trait JournalEntryRepository: Send + Sync {
    async fn create(&self, entry: CreateJournalEntry) -> Result<JournalEntry, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<JournalEntry>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<JournalEntry>, ApiError>;
    async fn find_by_date_range(
        &self,
        tenant_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<JournalEntry>, ApiError>;
    async fn post(&self, id: i64) -> Result<JournalEntry, ApiError>;
    async fn void(&self, id: i64) -> Result<JournalEntry, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for JournalLine operations
#[async_trait]
pub trait JournalLineRepository: Send + Sync {
    async fn create_many(
        &self,
        entry_id: i64,
        lines: Vec<CreateJournalLine>,
    ) -> Result<Vec<JournalLine>, ApiError>;
    async fn find_by_entry(&self, entry_id: i64) -> Result<Vec<JournalLine>, ApiError>;
    async fn delete_by_entry(&self, entry_id: i64) -> Result<(), ApiError>;
}

/// Type aliases
pub type BoxAccountRepository = Arc<dyn AccountRepository>;
pub type BoxJournalEntryRepository = Arc<dyn JournalEntryRepository>;
pub type BoxJournalLineRepository = Arc<dyn JournalLineRepository>;

fn generate_entry_number(count: i64) -> String {
    format!("JE-{:06}", count)
}

/// Inner state for InMemoryAccountRepository
struct InMemoryAccountInner {
    accounts: std::collections::HashMap<i64, Account>,
    next_id: i64,
}

/// In-memory account repository
pub struct InMemoryAccountRepository {
    inner: Mutex<InMemoryAccountInner>,
}

impl InMemoryAccountRepository {
    pub fn new() -> Self {
        let repo = Self {
            inner: Mutex::new(InMemoryAccountInner {
                accounts: std::collections::HashMap::new(),
                next_id: 1,
            }),
        };
        repo.seed_defaults();
        repo
    }

    fn seed_defaults(&self) {
        let defaults = vec![
            (
                1,
                "1000",
                "Cash",
                AccountType::Asset,
                AccountSubType::CurrentAsset,
            ),
            (
                2,
                "1100",
                "Accounts Receivable",
                AccountType::Asset,
                AccountSubType::CurrentAsset,
            ),
            (
                3,
                "1500",
                "Equipment",
                AccountType::Asset,
                AccountSubType::FixedAsset,
            ),
            (
                4,
                "2000",
                "Accounts Payable",
                AccountType::Liability,
                AccountSubType::CurrentLiability,
            ),
            (
                5,
                "2100",
                "Accrued Expenses",
                AccountType::Liability,
                AccountSubType::CurrentLiability,
            ),
            (
                6,
                "3000",
                "Common Stock",
                AccountType::Equity,
                AccountSubType::OwnersEquity,
            ),
            (
                7,
                "3100",
                "Retained Earnings",
                AccountType::Equity,
                AccountSubType::RetainedEarnings,
            ),
            (
                8,
                "4000",
                "Sales Revenue",
                AccountType::Revenue,
                AccountSubType::OperatingRevenue,
            ),
            (
                9,
                "4100",
                "Service Revenue",
                AccountType::Revenue,
                AccountSubType::OperatingRevenue,
            ),
            (
                10,
                "5000",
                "Cost of Goods Sold",
                AccountType::Expense,
                AccountSubType::OperatingExpense,
            ),
            (
                11,
                "5100",
                "Salaries Expense",
                AccountType::Expense,
                AccountSubType::OperatingExpense,
            ),
            (
                12,
                "5200",
                "Rent Expense",
                AccountType::Expense,
                AccountSubType::OperatingExpense,
            ),
        ];

        let mut inner = self.inner.lock();
        for (id, code, name, at, st) in defaults {
            inner.accounts.insert(
                id,
                Account {
                    id,
                    tenant_id: 1,
                    code: code.to_string(),
                    name: name.to_string(),
                    account_type: at,
                    sub_type: st,
                    parent_id: None,
                    is_active: true,
                    allow_transaction: true,
                    created_at: Utc::now(),
                },
            );
        }
        inner.next_id = 13;
    }
}

impl Default for InMemoryAccountRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AccountRepository for InMemoryAccountRepository {
    async fn create(&self, create: CreateAccount) -> Result<Account, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let account = Account {
            id,
            tenant_id: create.tenant_id,
            code: create.code,
            name: create.name,
            account_type: create.account_type,
            sub_type: create.sub_type,
            parent_id: create.parent_id,
            is_active: true,
            allow_transaction: create.allow_transaction,
            created_at: Utc::now(),
        };

        inner.accounts.insert(id, account.clone());
        Ok(account)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Account>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.accounts.get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Account>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .accounts
            .values()
            .filter(|a| a.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_code(&self, tenant_id: i64, code: &str) -> Result<Option<Account>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .accounts
            .values()
            .find(|a| a.tenant_id == tenant_id && a.code == code)
            .cloned())
    }

    async fn find_by_type(
        &self,
        tenant_id: i64,
        account_type: AccountType,
    ) -> Result<Vec<Account>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .accounts
            .values()
            .filter(|a| a.tenant_id == tenant_id && a.account_type == account_type)
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        id: i64,
        name: Option<String>,
        is_active: Option<bool>,
    ) -> Result<Account, ApiError> {
        let mut inner = self.inner.lock();
        let account = inner
            .accounts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Account {} not found", id)))?;
        if let Some(n) = name {
            account.name = n;
        }
        if let Some(active) = is_active {
            account.is_active = active;
        }
        Ok(account.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.accounts.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryJournalEntryRepository
struct InMemoryJournalEntryInner {
    entries: std::collections::HashMap<i64, JournalEntry>,
    next_id: i64,
}

/// In-memory journal entry repository
pub struct InMemoryJournalEntryRepository {
    inner: Mutex<InMemoryJournalEntryInner>,
}

impl InMemoryJournalEntryRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryJournalEntryInner {
                entries: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryJournalEntryRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JournalEntryRepository for InMemoryJournalEntryRepository {
    async fn create(&self, create: CreateJournalEntry) -> Result<JournalEntry, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let total_debit: Decimal = create.lines.iter().map(|l| l.debit).sum();
        let total_credit: Decimal = create.lines.iter().map(|l| l.credit).sum();

        let entry = JournalEntry {
            id,
            tenant_id: create.tenant_id,
            entry_number: generate_entry_number(id),
            date: create.date,
            description: create.description,
            reference: create.reference,
            status: JournalEntryStatus::Draft,
            total_debit,
            total_credit,
            created_by: create.created_by,
            created_at: Utc::now(),
            posted_at: None,
        };

        inner.entries.insert(id, entry.clone());
        Ok(entry)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<JournalEntry>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.entries.get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<JournalEntry>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .entries
            .values()
            .filter(|e| e.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_date_range(
        &self,
        tenant_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<JournalEntry>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .entries
            .values()
            .filter(|e| e.tenant_id == tenant_id && e.date >= start && e.date <= end)
            .cloned()
            .collect())
    }

    async fn post(&self, id: i64) -> Result<JournalEntry, ApiError> {
        let mut inner = self.inner.lock();
        let entry = inner
            .entries
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Journal entry {} not found", id)))?;
        if entry.status == JournalEntryStatus::Posted {
            return Err(ApiError::BadRequest("Entry already posted".to_string()));
        }
        entry.status = JournalEntryStatus::Posted;
        entry.posted_at = Some(Utc::now());
        Ok(entry.clone())
    }

    async fn void(&self, id: i64) -> Result<JournalEntry, ApiError> {
        let mut inner = self.inner.lock();
        let entry = inner
            .entries
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Journal entry {} not found", id)))?;
        entry.status = JournalEntryStatus::Voided;
        Ok(entry.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.entries.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryJournalLineRepository
struct InMemoryJournalLineInner {
    lines: std::collections::HashMap<i64, Vec<JournalLine>>,
    next_id: i64,
}

/// In-memory journal line repository
pub struct InMemoryJournalLineRepository {
    inner: Mutex<InMemoryJournalLineInner>,
}

impl InMemoryJournalLineRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryJournalLineInner {
                lines: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryJournalLineRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JournalLineRepository for InMemoryJournalLineRepository {
    async fn create_many(
        &self,
        entry_id: i64,
        create_lines: Vec<CreateJournalLine>,
    ) -> Result<Vec<JournalLine>, ApiError> {
        for line in &create_lines {
            line.validate()
                .map_err(|e| ApiError::Validation(e.join(", ")))?;
        }

        let mut inner = self.inner.lock();
        let mut lines = Vec::new();

        for create in create_lines {
            let id = inner.next_id;
            inner.next_id += 1;
            lines.push(JournalLine {
                id,
                entry_id,
                account_id: create.account_id,
                debit: create.debit,
                credit: create.credit,
                description: create.description,
                reference: create.reference,
            });
        }

        inner.lines.insert(entry_id, lines.clone());
        Ok(lines)
    }

    async fn find_by_entry(&self, entry_id: i64) -> Result<Vec<JournalLine>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.lines.get(&entry_id).cloned().unwrap_or_default())
    }

    async fn delete_by_entry(&self, entry_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.lines.remove(&entry_id);
        Ok(())
    }
}
