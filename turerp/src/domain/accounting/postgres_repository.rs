//! PostgreSQL accounting repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::accounting::model::{
    Account, AccountSubType, AccountType, CreateAccount, CreateJournalEntry, CreateJournalLine,
    JournalEntry, JournalEntryStatus, JournalLine,
};
use crate::domain::accounting::repository::{
    AccountRepository, BoxAccountRepository, BoxJournalEntryRepository, BoxJournalLineRepository,
    JournalEntryRepository, JournalLineRepository,
};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

// ---------------------------------------------------------------------------
// AccountRow / Account conversion
// ---------------------------------------------------------------------------

/// Database row representation for Account
#[derive(Debug, FromRow)]
struct AccountRow {
    id: i64,
    tenant_id: i64,
    code: String,
    name: String,
    account_type: String,
    sub_type: String,
    parent_id: Option<i64>,
    is_active: bool,
    allow_transaction: bool,
    created_at: DateTime<Utc>,
    total_count: Option<i64>,
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
        let account_type = row.account_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid account_type '{}' in database: {}, defaulting to Expense",
                row.account_type,
                e
            );
            AccountType::Expense
        });

        let sub_type = row.sub_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid sub_type '{}' in database: {}, defaulting to OperatingExpense",
                row.sub_type,
                e
            );
            AccountSubType::OperatingExpense
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            code: row.code,
            name: row.name,
            account_type,
            sub_type,
            parent_id: row.parent_id,
            is_active: row.is_active,
            allow_transaction: row.allow_transaction,
            created_at: row.created_at,
        }
    }
}

// ---------------------------------------------------------------------------
// JournalEntryRow / JournalEntry conversion
// ---------------------------------------------------------------------------

/// Database row representation for JournalEntry
#[derive(Debug, FromRow)]
struct JournalEntryRow {
    id: i64,
    tenant_id: i64,
    entry_number: String,
    date: DateTime<Utc>,
    description: String,
    reference: Option<String>,
    status: String,
    total_debit: Decimal,
    total_credit: Decimal,
    created_by: i64,
    created_at: DateTime<Utc>,
    posted_at: Option<DateTime<Utc>>,
    total_count: Option<i64>,
}

impl From<JournalEntryRow> for JournalEntry {
    fn from(row: JournalEntryRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            JournalEntryStatus::Draft
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            entry_number: row.entry_number,
            date: row.date,
            description: row.description,
            reference: row.reference,
            status,
            total_debit: row.total_debit,
            total_credit: row.total_credit,
            created_by: row.created_by,
            created_at: row.created_at,
            posted_at: row.posted_at,
        }
    }
}

// ---------------------------------------------------------------------------
// JournalLineRow / JournalLine conversion
// ---------------------------------------------------------------------------

/// Database row representation for JournalLine
#[derive(Debug, FromRow)]
struct JournalLineRow {
    id: i64,
    entry_id: i64,
    account_id: i64,
    debit: Decimal,
    credit: Decimal,
    description: Option<String>,
    reference: Option<String>,
}

impl From<JournalLineRow> for JournalLine {
    fn from(row: JournalLineRow) -> Self {
        Self {
            id: row.id,
            entry_id: row.entry_id,
            account_id: row.account_id,
            debit: row.debit,
            credit: row.credit,
            description: row.description,
            reference: row.reference,
        }
    }
}

// ===========================================================================
// PostgresAccountRepository
// ===========================================================================

/// PostgreSQL account repository
pub struct PostgresAccountRepository {
    pool: Arc<PgPool>,
}

impl PostgresAccountRepository {
    /// Create a new PostgreSQL account repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxAccountRepository {
        Arc::new(self) as BoxAccountRepository
    }
}

#[async_trait]
impl AccountRepository for PostgresAccountRepository {
    async fn create(&self, create: CreateAccount) -> Result<Account, ApiError> {
        let account_type = create.account_type.to_string();
        let sub_type = create.sub_type.to_string();

        let row: AccountRow = sqlx::query_as(
            r#"
            INSERT INTO accounts (tenant_id, code, name, account_type, sub_type, parent_id, is_active, allow_transaction, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            RETURNING id, tenant_id, code, name, account_type, sub_type, parent_id, is_active, allow_transaction, created_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.code)
        .bind(&create.name)
        .bind(&account_type)
        .bind(&sub_type)
        .bind(create.parent_id)
        .bind(true) // is_active defaults to true
        .bind(create.allow_transaction)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Account"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Account>, ApiError> {
        let result: Option<AccountRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, account_type, sub_type, parent_id, is_active, allow_transaction, created_at
            FROM accounts
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find account by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Account>, ApiError> {
        let rows: Vec<AccountRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, account_type, sub_type, parent_id, is_active, allow_transaction, created_at
            FROM accounts
            WHERE tenant_id = $1
            ORDER BY code ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find accounts by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Account>, ApiError> {
        let offset = (page.saturating_sub(1)) * per_page;
        let rows: Vec<AccountRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, account_type, sub_type, parent_id, is_active, allow_transaction, created_at,
                   COUNT(*) OVER() as total_count
            FROM accounts
            WHERE tenant_id = $1
            ORDER BY id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Account"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<Account> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_code(&self, tenant_id: i64, code: &str) -> Result<Option<Account>, ApiError> {
        let result: Option<AccountRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, account_type, sub_type, parent_id, is_active, allow_transaction, created_at
            FROM accounts
            WHERE tenant_id = $1 AND code = $2
            "#,
        )
        .bind(tenant_id)
        .bind(code)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find account by code: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_type(
        &self,
        tenant_id: i64,
        account_type: AccountType,
    ) -> Result<Vec<Account>, ApiError> {
        let account_type_str = account_type.to_string();

        let rows: Vec<AccountRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, account_type, sub_type, parent_id, is_active, allow_transaction, created_at
            FROM accounts
            WHERE tenant_id = $1 AND account_type = $2
            ORDER BY code ASC
            "#,
        )
        .bind(tenant_id)
        .bind(&account_type_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find accounts by type: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update(
        &self,
        id: i64,
        name: Option<String>,
        is_active: Option<bool>,
    ) -> Result<Account, ApiError> {
        let row: AccountRow = sqlx::query_as(
            r#"
            UPDATE accounts
            SET
                name = COALESCE($1, name),
                is_active = COALESCE($2, is_active)
            WHERE id = $3
            RETURNING id, tenant_id, code, name, account_type, sub_type, parent_id, is_active, allow_transaction, created_at
            "#,
        )
        .bind(&name)
        .bind(is_active)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Account"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM accounts
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete account: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Account not found".to_string()));
        }

        Ok(())
    }
}

// ===========================================================================
// PostgresJournalEntryRepository
// ===========================================================================

/// PostgreSQL journal entry repository
pub struct PostgresJournalEntryRepository {
    pool: Arc<PgPool>,
}

impl PostgresJournalEntryRepository {
    /// Create a new PostgreSQL journal entry repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxJournalEntryRepository {
        Arc::new(self) as BoxJournalEntryRepository
    }
}

#[async_trait]
impl JournalEntryRepository for PostgresJournalEntryRepository {
    async fn create(&self, create: CreateJournalEntry) -> Result<JournalEntry, ApiError> {
        // Calculate totals from lines
        let total_debit: Decimal = create.lines.iter().map(|l| l.debit).sum();
        let total_credit: Decimal = create.lines.iter().map(|l| l.credit).sum();

        // Generate entry number using a sequence-like pattern
        let count: (i64,) =
            sqlx::query_as(r#"SELECT COUNT(*) as count FROM journal_entries WHERE tenant_id = $1"#)
                .bind(create.tenant_id)
                .fetch_one(&*self.pool)
                .await
                .map_err(|e| {
                    ApiError::Database(format!("Failed to count journal entries: {}", e))
                })?;

        let entry_number = format!("JE-{:06}", count.0 + 1);

        let row: JournalEntryRow = sqlx::query_as(
            r#"
            INSERT INTO journal_entries (tenant_id, entry_number, date, description, reference, status, total_debit, total_credit, created_by, created_at)
            VALUES ($1, $2, $3, $4, $5, 'Draft', $6, $7, $8, NOW())
            RETURNING id, tenant_id, entry_number, date, description, reference, status, total_debit, total_credit, created_by, created_at, posted_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&entry_number)
        .bind(create.date)
        .bind(&create.description)
        .bind(&create.reference)
        .bind(total_debit)
        .bind(total_credit)
        .bind(create.created_by)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JournalEntry"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<JournalEntry>, ApiError> {
        let result: Option<JournalEntryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, entry_number, date, description, reference, status, total_debit, total_credit, created_by, created_at, posted_at
            FROM journal_entries
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find journal entry by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<JournalEntry>, ApiError> {
        let rows: Vec<JournalEntryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, entry_number, date, description, reference, status, total_debit, total_credit, created_by, created_at, posted_at
            FROM journal_entries
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find journal entries by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<JournalEntry>, ApiError> {
        let offset = (page.saturating_sub(1)) * per_page;
        let rows: Vec<JournalEntryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, entry_number, date, description, reference, status, total_debit, total_credit, created_by, created_at, posted_at,
                   COUNT(*) OVER() as total_count
            FROM journal_entries
            WHERE tenant_id = $1
            ORDER BY id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JournalEntry"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<JournalEntry> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_date_range(
        &self,
        tenant_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<JournalEntry>, ApiError> {
        let rows: Vec<JournalEntryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, entry_number, date, description, reference, status, total_debit, total_credit, created_by, created_at, posted_at
            FROM journal_entries
            WHERE tenant_id = $1 AND date >= $2 AND date <= $3
            ORDER BY date ASC
            "#,
        )
        .bind(tenant_id)
        .bind(start)
        .bind(end)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find journal entries by date range: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn post(&self, id: i64) -> Result<JournalEntry, ApiError> {
        let row: JournalEntryRow = sqlx::query_as(
            r#"
            UPDATE journal_entries
            SET status = 'Posted', posted_at = NOW()
            WHERE id = $1
            RETURNING id, tenant_id, entry_number, date, description, reference, status, total_debit, total_credit, created_by, created_at, posted_at
            "#,
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JournalEntry"))?;

        Ok(row.into())
    }

    async fn void(&self, id: i64) -> Result<JournalEntry, ApiError> {
        let row: JournalEntryRow = sqlx::query_as(
            r#"
            UPDATE journal_entries
            SET status = 'Voided'
            WHERE id = $1
            RETURNING id, tenant_id, entry_number, date, description, reference, status, total_debit, total_credit, created_by, created_at, posted_at
            "#,
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "JournalEntry"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM journal_entries
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete journal entry: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Journal entry not found".to_string()));
        }

        Ok(())
    }
}

// ===========================================================================
// PostgresJournalLineRepository
// ===========================================================================

/// PostgreSQL journal line repository
pub struct PostgresJournalLineRepository {
    pool: Arc<PgPool>,
}

impl PostgresJournalLineRepository {
    /// Create a new PostgreSQL journal line repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxJournalLineRepository {
        Arc::new(self) as BoxJournalLineRepository
    }
}

#[async_trait]
impl JournalLineRepository for PostgresJournalLineRepository {
    async fn create_many(
        &self,
        entry_id: i64,
        lines: Vec<CreateJournalLine>,
    ) -> Result<Vec<JournalLine>, ApiError> {
        if lines.is_empty() {
            return Ok(Vec::new());
        }

        let mut results = Vec::with_capacity(lines.len());

        for line in lines {
            let row: JournalLineRow = sqlx::query_as(
                r#"
                INSERT INTO journal_lines (entry_id, account_id, debit, credit, description, reference)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id, entry_id, account_id, debit, credit, description, reference
                "#,
            )
            .bind(entry_id)
            .bind(line.account_id)
            .bind(line.debit)
            .bind(line.credit)
            .bind(&line.description)
            .bind(&line.reference)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "JournalLine"))?;

            results.push(row.into());
        }

        Ok(results)
    }

    async fn find_by_entry(&self, entry_id: i64) -> Result<Vec<JournalLine>, ApiError> {
        let rows: Vec<JournalLineRow> = sqlx::query_as(
            r#"
            SELECT id, entry_id, account_id, debit, credit, description, reference
            FROM journal_lines
            WHERE entry_id = $1
            ORDER BY id ASC
            "#,
        )
        .bind(entry_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find journal lines by entry: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete_by_entry(&self, entry_id: i64) -> Result<(), ApiError> {
        sqlx::query(
            r#"
            DELETE FROM journal_lines
            WHERE entry_id = $1
            "#,
        )
        .bind(entry_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to delete journal lines by entry: {}", e))
        })?;

        Ok(())
    }
}
