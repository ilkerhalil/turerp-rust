//! Chart of Accounts repository

use async_trait::async_trait;
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::common::SoftDeletable;
use crate::domain::chart_of_accounts::model::{
    AccountGroup, ChartAccount, CreateChartAccount, UpdateChartAccount,
};
use crate::error::ApiError;

/// Repository trait for Chart of Accounts operations
#[async_trait]
pub trait ChartAccountRepository: Send + Sync {
    /// Create a new chart account
    async fn create(
        &self,
        account: CreateChartAccount,
        tenant_id: i64,
    ) -> Result<ChartAccount, ApiError>;

    /// Find a chart account by code
    async fn find_by_code(
        &self,
        code: &str,
        tenant_id: i64,
    ) -> Result<Option<ChartAccount>, ApiError>;

    /// Find a chart account by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ChartAccount>, ApiError>;

    /// Find all chart accounts with optional group filter and pagination
    async fn find_all(
        &self,
        tenant_id: i64,
        group: Option<AccountGroup>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ChartAccount>, ApiError>;

    /// Find child accounts by parent code
    async fn find_children(
        &self,
        parent_code: &str,
        tenant_id: i64,
    ) -> Result<Vec<ChartAccount>, ApiError>;

    /// Update a chart account
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateChartAccount,
    ) -> Result<ChartAccount, ApiError>;

    /// Soft delete a chart account
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    /// Update the balance of a chart account
    async fn update_balance(
        &self,
        id: i64,
        tenant_id: i64,
        balance: Decimal,
    ) -> Result<(), ApiError>;
}

/// Type alias for boxed repository
pub type BoxChartAccountRepository = Arc<dyn ChartAccountRepository>;

/// Inner state for InMemoryChartAccountRepository
struct InMemoryChartAccountInner {
    accounts: HashMap<i64, ChartAccount>,
    next_id: i64,
}

/// In-memory chart account repository for testing
pub struct InMemoryChartAccountRepository {
    inner: Mutex<InMemoryChartAccountInner>,
}

impl InMemoryChartAccountRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryChartAccountInner {
                accounts: HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryChartAccountRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChartAccountRepository for InMemoryChartAccountRepository {
    async fn create(
        &self,
        create: CreateChartAccount,
        tenant_id: i64,
    ) -> Result<ChartAccount, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let now = chrono::Utc::now();

        let new_account = ChartAccount {
            id,
            tenant_id,
            code: create.code,
            name: create.name,
            group: create.group,
            parent_code: create.parent_code,
            level: 1, // Default level, can be calculated based on parent
            account_type: create.account_type,
            is_active: true,
            balance: Decimal::ZERO,
            allow_posting: create.allow_posting,
            created_at: now,
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };

        inner.accounts.insert(id, new_account.clone());
        Ok(new_account)
    }

    async fn find_by_code(
        &self,
        code: &str,
        tenant_id: i64,
    ) -> Result<Option<ChartAccount>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .accounts
            .values()
            .find(|a| a.code == code && a.tenant_id == tenant_id && !a.is_deleted())
            .cloned())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ChartAccount>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .accounts
            .get(&id)
            .filter(|a| a.tenant_id == tenant_id && !a.is_deleted())
            .cloned())
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        group: Option<AccountGroup>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ChartAccount>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .accounts
            .values()
            .filter(|a| {
                a.tenant_id == tenant_id && !a.is_deleted() && group.is_none_or(|g| a.group == g)
            })
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(params.offset() as usize)
            .take(params.per_page as usize)
            .collect();
        Ok(PaginatedResult::new(
            items,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn find_children(
        &self,
        parent_code: &str,
        tenant_id: i64,
    ) -> Result<Vec<ChartAccount>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .accounts
            .values()
            .filter(|a| {
                a.parent_code.as_deref() == Some(parent_code)
                    && a.tenant_id == tenant_id
                    && !a.is_deleted()
            })
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateChartAccount,
    ) -> Result<ChartAccount, ApiError> {
        let mut inner = self.inner.lock();

        let account = inner
            .accounts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Chart account {} not found", id)))?;

        if account.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!(
                "Chart account {} not found",
                id
            )));
        }

        if let Some(name) = update.name {
            account.name = name;
        }
        if let Some(group) = update.group {
            account.group = group;
        }
        if let Some(is_active) = update.is_active {
            account.is_active = is_active;
        }
        if let Some(allow_posting) = update.allow_posting {
            account.allow_posting = allow_posting;
        }

        account.updated_at = Some(chrono::Utc::now());

        Ok(account.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let account = inner
            .accounts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Chart account {} not found", id)))?;

        if account.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!(
                "Chart account {} not found",
                id
            )));
        }

        account.mark_deleted(deleted_by);
        Ok(())
    }

    async fn update_balance(
        &self,
        id: i64,
        tenant_id: i64,
        balance: Decimal,
    ) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let account = inner
            .accounts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Chart account {} not found", id)))?;

        if account.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!(
                "Chart account {} not found",
                id
            )));
        }

        account.balance = balance;
        account.updated_at = Some(chrono::Utc::now());

        Ok(())
    }
}
