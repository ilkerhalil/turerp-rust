//! Chart of Accounts service for business logic

use rust_decimal::Decimal;
use std::collections::HashMap;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::chart_of_accounts::model::{
    AccountGroup, AccountTreeNode, ChartAccount, ChartAccountResponse, CreateChartAccount,
    TrialBalanceEntry, UpdateChartAccount,
};
use crate::domain::chart_of_accounts::repository::BoxChartAccountRepository;
use crate::error::ApiError;

/// Chart of Accounts service
#[derive(Clone)]
pub struct ChartOfAccountsService {
    repo: BoxChartAccountRepository,
}

impl ChartOfAccountsService {
    pub fn new(repo: BoxChartAccountRepository) -> Self {
        Self { repo }
    }

    /// Create a new chart account
    pub async fn create_account(
        &self,
        create: CreateChartAccount,
        tenant_id: i64,
    ) -> Result<ChartAccountResponse, ApiError> {
        // Check if code already exists
        if self
            .repo
            .find_by_code(&create.code, tenant_id)
            .await?
            .is_some()
        {
            return Err(ApiError::Conflict(format!(
                "Account code '{}' already exists",
                create.code
            )));
        }

        // Validate parent code if provided
        if let Some(ref parent_code) = create.parent_code {
            if self
                .repo
                .find_by_code(parent_code, tenant_id)
                .await?
                .is_none()
            {
                return Err(ApiError::BadRequest(format!(
                    "Parent account code '{}' not found",
                    parent_code
                )));
            }
        }

        let account = self.repo.create(create, tenant_id).await?;
        Ok(account.into())
    }

    /// Get a chart account by ID
    pub async fn get_account(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<ChartAccountResponse, ApiError> {
        let account = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Chart account {} not found", id)))?;

        Ok(account.into())
    }

    /// List chart accounts with optional group filter and pagination
    pub async fn list_accounts(
        &self,
        tenant_id: i64,
        group: Option<AccountGroup>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ChartAccountResponse>, ApiError> {
        let result = self.repo.find_all(tenant_id, group, params).await?;
        Ok(result.map(|a| a.into()))
    }

    /// Update a chart account
    pub async fn update_account(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateChartAccount,
    ) -> Result<ChartAccountResponse, ApiError> {
        // Verify account exists
        let _ = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Chart account {} not found", id)))?;

        let account = self.repo.update(id, tenant_id, update).await?;
        Ok(account.into())
    }

    /// Delete a chart account (soft delete)
    pub async fn delete_account(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        // Verify account exists
        let _ = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Chart account {} not found", id)))?;

        self.repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Get the hierarchical tree of accounts
    pub async fn get_tree(&self, tenant_id: i64) -> Result<Vec<AccountTreeNode>, ApiError> {
        let all_accounts = self
            .repo
            .find_all(
                tenant_id,
                None,
                PaginationParams {
                    page: 1,
                    per_page: 10000, // Get all accounts for tree building
                },
            )
            .await?;

        let accounts: Vec<ChartAccount> = all_accounts.items;

        // Build a map of code -> ChartAccount for lookup
        let account_map: HashMap<&str, &ChartAccount> =
            accounts.iter().map(|a| (a.code.as_str(), a)).collect();

        // Build a map of parent_code -> children
        let mut children_map: HashMap<Option<&str>, Vec<&ChartAccount>> = HashMap::new();
        for account in &accounts {
            children_map
                .entry(account.parent_code.as_deref())
                .or_default()
                .push(account);
        }

        // Build tree starting from root accounts (no parent)
        let root_accounts = children_map.get(&None).cloned().unwrap_or_default();

        fn build_tree(
            account: &ChartAccount,
            children_map: &HashMap<Option<&str>, Vec<&ChartAccount>>,
            _account_map: &HashMap<&str, &ChartAccount>,
        ) -> AccountTreeNode {
            let children = children_map
                .get(&Some(account.code.as_str()))
                .cloned()
                .unwrap_or_default();

            let child_nodes: Vec<AccountTreeNode> = children
                .iter()
                .map(|c| build_tree(c, children_map, _account_map))
                .collect();

            // Calculate balance: if account allows posting, use its own balance
            // otherwise sum children balances
            let balance = if account.allow_posting {
                account.balance
            } else {
                child_nodes.iter().map(|c| c.balance).sum()
            };

            AccountTreeNode {
                code: account.code.clone(),
                name: account.name.clone(),
                group: account.group,
                balance,
                children: child_nodes,
            }
        }

        let tree: Vec<AccountTreeNode> = root_accounts
            .iter()
            .map(|a| build_tree(a, &children_map, &account_map))
            .collect();

        Ok(tree)
    }

    /// Get children of an account by parent code
    pub async fn get_children(
        &self,
        parent_code: &str,
        tenant_id: i64,
    ) -> Result<Vec<ChartAccountResponse>, ApiError> {
        let children = self.repo.find_children(parent_code, tenant_id).await?;
        Ok(children.into_iter().map(|c| c.into()).collect())
    }

    /// Recalculate the balance of a parent account by summing all child balances
    pub async fn recalculate_balance(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<ChartAccountResponse, ApiError> {
        let account = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Chart account {} not found", id)))?;

        let children = self.repo.find_children(&account.code, tenant_id).await?;

        let total_balance: Decimal = children.iter().map(|c| c.balance).sum();

        self.repo
            .update_balance(id, tenant_id, total_balance)
            .await?;

        // Re-fetch to get updated values
        let updated = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Chart account {} not found", id)))?;

        Ok(updated.into())
    }

    /// Get trial balance: all accounts with debit/credit balances
    pub async fn get_trial_balance(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<TrialBalanceEntry>, ApiError> {
        let all_accounts = self
            .repo
            .find_all(
                tenant_id,
                None,
                PaginationParams {
                    page: 1,
                    per_page: 10000,
                },
            )
            .await?;

        let entries: Vec<TrialBalanceEntry> = all_accounts
            .items
            .into_iter()
            .filter(|a| a.allow_posting && a.is_active)
            .map(|a| {
                let (debit_balance, credit_balance) = if a.balance >= Decimal::ZERO {
                    (a.balance, Decimal::ZERO)
                } else {
                    (Decimal::ZERO, -a.balance)
                };

                TrialBalanceEntry {
                    account_code: a.code,
                    account_name: a.name,
                    debit_balance,
                    credit_balance,
                }
            })
            .collect();

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::accounting::model::AccountType;
    use crate::domain::chart_of_accounts::model::{AccountGroup, CreateChartAccount};
    use crate::domain::chart_of_accounts::repository::InMemoryChartAccountRepository;
    use std::sync::Arc;

    fn create_service() -> ChartOfAccountsService {
        let repo = Arc::new(InMemoryChartAccountRepository::new()) as BoxChartAccountRepository;
        ChartOfAccountsService::new(repo)
    }

    #[tokio::test]
    async fn test_create_account_success() {
        let service = create_service();

        let create = CreateChartAccount {
            code: "100".to_string(),
            name: "Cash".to_string(),
            group: AccountGroup::DonenVarliklar,
            parent_code: None,
            account_type: AccountType::Asset,
            allow_posting: true,
        };

        let result = service.create_account(create, 1).await;
        assert!(result.is_ok());
        let account = result.unwrap();
        assert_eq!(account.code, "100");
        assert_eq!(account.name, "Cash");
        assert_eq!(account.group, AccountGroup::DonenVarliklar);
    }

    #[tokio::test]
    async fn test_create_account_duplicate_code() {
        let service = create_service();

        let create = CreateChartAccount {
            code: "100".to_string(),
            name: "Cash".to_string(),
            group: AccountGroup::DonenVarliklar,
            parent_code: None,
            account_type: AccountType::Asset,
            allow_posting: true,
        };

        service.create_account(create.clone(), 1).await.unwrap();
        let result = service.create_account(create, 1).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_get_account_by_id() {
        let service = create_service();

        let create = CreateChartAccount {
            code: "100".to_string(),
            name: "Cash".to_string(),
            group: AccountGroup::DonenVarliklar,
            parent_code: None,
            account_type: AccountType::Asset,
            allow_posting: true,
        };

        let created = service.create_account(create, 1).await.unwrap();
        let result = service.get_account(created.id, 1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().code, "100");
    }

    #[tokio::test]
    async fn test_get_account_not_found() {
        let service = create_service();
        let result = service.get_account(999, 1).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_update_account() {
        let service = create_service();

        let create = CreateChartAccount {
            code: "100".to_string(),
            name: "Cash".to_string(),
            group: AccountGroup::DonenVarliklar,
            parent_code: None,
            account_type: AccountType::Asset,
            allow_posting: true,
        };

        let created = service.create_account(create, 1).await.unwrap();

        let update = UpdateChartAccount {
            name: Some("Cash & Bank".to_string()),
            ..Default::default()
        };

        let result = service.update_account(created.id, 1, update).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Cash & Bank");
    }

    #[tokio::test]
    async fn test_delete_account() {
        let service = create_service();

        let create = CreateChartAccount {
            code: "100".to_string(),
            name: "Cash".to_string(),
            group: AccountGroup::DonenVarliklar,
            parent_code: None,
            account_type: AccountType::Asset,
            allow_posting: true,
        };

        let created = service.create_account(create, 1).await.unwrap();
        let result = service.delete_account(created.id, 1, 1).await;
        assert!(result.is_ok());

        // Verify deleted
        let result = service.get_account(created.id, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_trial_balance() {
        let service = create_service();

        let create1 = CreateChartAccount {
            code: "100".to_string(),
            name: "Cash".to_string(),
            group: AccountGroup::DonenVarliklar,
            parent_code: None,
            account_type: AccountType::Asset,
            allow_posting: true,
        };

        let create2 = CreateChartAccount {
            code: "400".to_string(),
            name: "Sales Revenue".to_string(),
            group: AccountGroup::GelirTablosu,
            parent_code: None,
            account_type: AccountType::Revenue,
            allow_posting: true,
        };

        service.create_account(create1, 1).await.unwrap();
        service.create_account(create2, 1).await.unwrap();

        let result = service.get_trial_balance(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }
}
