//! Bank repository

use async_trait::async_trait;
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::common::SoftDeletable;
use crate::domain::bank::model::{
    BankAccount, BankStatement, BankTransaction, CreateBankAccount, CreateReconciliationRule,
    ImportBankStatement, MatchStatus, ReconciliationRule, UpdateBankAccount,
    UpdateReconciliationRule,
};
use crate::error::ApiError;

/// Repository trait for bank operations
#[async_trait]
pub trait BankRepository: Send + Sync {
    /// Create a new bank account
    async fn create_account(&self, account: CreateBankAccount) -> Result<BankAccount, ApiError>;

    /// Find bank account by ID
    async fn find_account_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<BankAccount>, ApiError>;

    /// Find all bank accounts for a tenant
    async fn find_accounts(&self, tenant_id: i64) -> Result<Vec<BankAccount>, ApiError>;

    /// Update a bank account
    async fn update_account(
        &self,
        id: i64,
        tenant_id: i64,
        account: UpdateBankAccount,
    ) -> Result<BankAccount, ApiError>;

    /// Soft delete a bank account
    async fn soft_delete_account(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError>;

    /// Restore a soft-deleted bank account
    async fn restore_account(&self, id: i64, tenant_id: i64) -> Result<BankAccount, ApiError>;

    /// Hard delete a bank account (admin only)
    async fn destroy_account(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Create a bank statement
    async fn create_statement(
        &self,
        tenant_id: i64,
        account_id: i64,
        import: ImportBankStatement,
    ) -> Result<BankStatement, ApiError>;

    /// Mark statement as processed
    async fn mark_statement_processed(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Create a bank transaction
    async fn create_transaction(
        &self,
        tenant_id: i64,
        account_id: i64,
        tx: crate::domain::bank::model::ParsedBankTransaction,
    ) -> Result<BankTransaction, ApiError>;

    /// Find transactions by account
    async fn find_transactions_by_account(
        &self,
        account_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError>;

    /// Find unmatched transactions by account
    async fn find_unmatched_transactions(
        &self,
        account_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError>;

    /// Find all unmatched transactions for a tenant
    async fn find_all_unmatched_transactions(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError>;

    /// Get transaction by ID
    async fn find_transaction_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<BankTransaction>, ApiError>;

    /// Update transaction match status
    async fn update_transaction_match(
        &self,
        id: i64,
        tenant_id: i64,
        invoice_id: Option<i64>,
        payment_id: Option<i64>,
        status: MatchStatus,
    ) -> Result<BankTransaction, ApiError>;

    /// Unmatch a transaction
    async fn unmatch_transaction(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<BankTransaction, ApiError>;

    /// Find transactions by reference number
    async fn find_transactions_by_reference(
        &self,
        reference: &str,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError>;

    /// Find transactions by amount and date range
    async fn find_transactions_by_amount_date(
        &self,
        amount: Decimal,
        date: chrono::NaiveDate,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError>;

    /// Get reconciliation summary for tenant
    async fn get_reconciliation_summary(
        &self,
        tenant_id: i64,
    ) -> Result<(i64, i64, i64, Decimal, Decimal), ApiError>;

    /// Create a reconciliation rule
    async fn create_rule(
        &self,
        rule: CreateReconciliationRule,
    ) -> Result<ReconciliationRule, ApiError>;

    /// Find all reconciliation rules for a tenant
    async fn find_rules(&self, tenant_id: i64) -> Result<Vec<ReconciliationRule>, ApiError>;

    /// Find active reconciliation rules for a tenant
    async fn find_active_rules(&self, tenant_id: i64) -> Result<Vec<ReconciliationRule>, ApiError>;

    /// Get reconciliation rule by ID
    async fn find_rule_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<ReconciliationRule>, ApiError>;

    /// Update a reconciliation rule
    async fn update_rule(
        &self,
        id: i64,
        tenant_id: i64,
        rule: UpdateReconciliationRule,
    ) -> Result<ReconciliationRule, ApiError>;

    /// Delete a reconciliation rule
    async fn delete_rule(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type alias for boxed repository
pub type BoxBankRepository = Arc<dyn BankRepository>;

struct InMemoryBankInner {
    accounts: std::collections::HashMap<i64, BankAccount>,
    statements: std::collections::HashMap<i64, BankStatement>,
    transactions: std::collections::HashMap<i64, BankTransaction>,
    rules: std::collections::HashMap<i64, ReconciliationRule>,
    next_account_id: i64,
    next_statement_id: i64,
    next_transaction_id: i64,
    next_rule_id: i64,
}

/// In-memory bank repository for testing
pub struct InMemoryBankRepository {
    inner: Mutex<InMemoryBankInner>,
}

impl InMemoryBankRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryBankInner {
                accounts: std::collections::HashMap::new(),
                statements: std::collections::HashMap::new(),
                transactions: std::collections::HashMap::new(),
                rules: std::collections::HashMap::new(),
                next_account_id: 1,
                next_statement_id: 1,
                next_transaction_id: 1,
                next_rule_id: 1,
            }),
        }
    }
}

impl Default for InMemoryBankRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BankRepository for InMemoryBankRepository {
    async fn create_account(&self, create: CreateBankAccount) -> Result<BankAccount, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_account_id;
        inner.next_account_id += 1;

        let bank_code = create
            .bank_code
            .parse()
            .map_err(|e: String| ApiError::Validation(format!("Invalid bank code: {}", e)))?;

        let account = BankAccount {
            id,
            tenant_id: create.tenant_id,
            company_id: create.company_id,
            bank_code,
            account_number: create.account_number,
            iban: create.iban,
            account_name: create.account_name,
            currency: create.currency,
            branch_code: create.branch_code,
            is_active: create.is_active,
            created_at: chrono::Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };

        inner.accounts.insert(id, account.clone());
        Ok(account)
    }

    async fn find_account_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<BankAccount>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .accounts
            .get(&id)
            .filter(|a| a.tenant_id == tenant_id && !a.is_deleted())
            .cloned())
    }

    async fn find_accounts(&self, tenant_id: i64) -> Result<Vec<BankAccount>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .accounts
            .values()
            .filter(|a| a.tenant_id == tenant_id && !a.is_deleted())
            .cloned()
            .collect())
    }

    async fn update_account(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateBankAccount,
    ) -> Result<BankAccount, ApiError> {
        let mut inner = self.inner.lock();
        let account = inner
            .accounts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Bank account {} not found", id)))?;

        if account.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Bank account {} not found", id)));
        }

        if let Some(v) = update.account_number {
            account.account_number = v;
        }
        if let Some(v) = update.account_name {
            account.account_name = v;
        }
        if let Some(v) = update.currency {
            account.currency = v;
        }
        if let Some(v) = update.iban {
            account.iban = Some(v);
        }
        if let Some(v) = update.branch_code {
            account.branch_code = Some(v);
        }
        if let Some(v) = update.is_active {
            account.is_active = v;
        }
        if let Some(v) = update.company_id {
            account.company_id = Some(v);
        }

        account.updated_at = Some(chrono::Utc::now());
        Ok(account.clone())
    }

    async fn soft_delete_account(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let account = inner
            .accounts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Bank account {} not found", id)))?;

        if account.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Bank account {} not found", id)));
        }

        account.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore_account(&self, id: i64, tenant_id: i64) -> Result<BankAccount, ApiError> {
        let mut inner = self.inner.lock();
        let account = inner
            .accounts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Bank account {} not found", id)))?;

        if account.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Bank account {} not found", id)));
        }

        account.restore();
        Ok(account.clone())
    }

    async fn destroy_account(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let account = inner
            .accounts
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Bank account {} not found", id)))?;

        if account.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Bank account {} not found", id)));
        }

        inner.accounts.remove(&id);
        Ok(())
    }

    async fn create_statement(
        &self,
        tenant_id: i64,
        account_id: i64,
        import: ImportBankStatement,
    ) -> Result<BankStatement, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_statement_id;
        inner.next_statement_id += 1;

        let statement = BankStatement {
            id,
            tenant_id,
            account_id,
            statement_date: chrono::Utc::now().date_naive(),
            format: import.format,
            raw_data: import.data,
            processed: false,
            created_at: chrono::Utc::now(),
        };

        inner.statements.insert(id, statement.clone());
        Ok(statement)
    }

    async fn mark_statement_processed(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let statement = inner
            .statements
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Statement {} not found", id)))?;

        if statement.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Statement {} not found", id)));
        }

        statement.processed = true;
        Ok(())
    }

    async fn create_transaction(
        &self,
        tenant_id: i64,
        account_id: i64,
        tx: crate::domain::bank::model::ParsedBankTransaction,
    ) -> Result<BankTransaction, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_transaction_id;
        inner.next_transaction_id += 1;

        let transaction = BankTransaction {
            id,
            tenant_id,
            account_id,
            transaction_date: tx.transaction_date,
            description: tx.description,
            amount: tx.amount,
            currency: tx.currency,
            balance_after: tx.balance_after,
            reference_no: tx.reference_no,
            matched_invoice_id: None,
            matched_payment_id: None,
            match_status: MatchStatus::Unmatched,
            created_at: chrono::Utc::now(),
        };

        inner.transactions.insert(id, transaction.clone());
        Ok(transaction)
    }

    async fn find_transactions_by_account(
        &self,
        account_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .transactions
            .values()
            .filter(|t| t.account_id == account_id && t.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_unmatched_transactions(
        &self,
        account_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .transactions
            .values()
            .filter(|t| {
                t.account_id == account_id
                    && t.tenant_id == tenant_id
                    && t.match_status == MatchStatus::Unmatched
            })
            .cloned()
            .collect())
    }

    async fn find_all_unmatched_transactions(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .transactions
            .values()
            .filter(|t| t.tenant_id == tenant_id && t.match_status == MatchStatus::Unmatched)
            .cloned()
            .collect())
    }

    async fn find_transaction_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<BankTransaction>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .transactions
            .get(&id)
            .filter(|t| t.tenant_id == tenant_id)
            .cloned())
    }

    async fn update_transaction_match(
        &self,
        id: i64,
        tenant_id: i64,
        invoice_id: Option<i64>,
        payment_id: Option<i64>,
        status: MatchStatus,
    ) -> Result<BankTransaction, ApiError> {
        let mut inner = self.inner.lock();
        let tx = inner
            .transactions
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Transaction {} not found", id)))?;

        if tx.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Transaction {} not found", id)));
        }

        tx.matched_invoice_id = invoice_id;
        tx.matched_payment_id = payment_id;
        tx.match_status = status;
        Ok(tx.clone())
    }

    async fn unmatch_transaction(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<BankTransaction, ApiError> {
        let mut inner = self.inner.lock();
        let tx = inner
            .transactions
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Transaction {} not found", id)))?;

        if tx.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Transaction {} not found", id)));
        }

        tx.matched_invoice_id = None;
        tx.matched_payment_id = None;
        tx.match_status = MatchStatus::Unmatched;
        Ok(tx.clone())
    }

    async fn find_transactions_by_reference(
        &self,
        reference: &str,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .transactions
            .values()
            .filter(|t| {
                t.tenant_id == tenant_id
                    && t.reference_no
                        .as_ref()
                        .map(|r| r == reference)
                        .unwrap_or(false)
            })
            .cloned()
            .collect())
    }

    async fn find_transactions_by_amount_date(
        &self,
        amount: Decimal,
        date: chrono::NaiveDate,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .transactions
            .values()
            .filter(|t| {
                t.tenant_id == tenant_id && t.amount == amount && t.transaction_date == date
            })
            .cloned()
            .collect())
    }

    async fn get_reconciliation_summary(
        &self,
        tenant_id: i64,
    ) -> Result<(i64, i64, i64, Decimal, Decimal), ApiError> {
        let inner = self.inner.lock();
        let transactions: Vec<_> = inner
            .transactions
            .values()
            .filter(|t| t.tenant_id == tenant_id)
            .cloned()
            .collect();

        let total = transactions.len() as i64;
        let matched = transactions
            .iter()
            .filter(|t| t.match_status == MatchStatus::Matched)
            .count() as i64;
        let unmatched = transactions
            .iter()
            .filter(|t| t.match_status == MatchStatus::Unmatched)
            .count() as i64;
        let total_amount: Decimal = transactions.iter().map(|t| t.amount).sum();
        let matched_amount: Decimal = transactions
            .iter()
            .filter(|t| t.match_status == MatchStatus::Matched)
            .map(|t| t.amount)
            .sum();

        Ok((total, matched, unmatched, total_amount, matched_amount))
    }

    async fn create_rule(
        &self,
        create: CreateReconciliationRule,
    ) -> Result<ReconciliationRule, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_rule_id;
        inner.next_rule_id += 1;

        let rule = ReconciliationRule {
            id,
            tenant_id: create.tenant_id,
            rule_name: create.rule_name,
            match_field: create.match_field,
            match_pattern: create.match_pattern,
            auto_match: create.auto_match,
            is_active: create.is_active,
            created_at: chrono::Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };

        inner.rules.insert(id, rule.clone());
        Ok(rule)
    }

    async fn find_rules(&self, tenant_id: i64) -> Result<Vec<ReconciliationRule>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .rules
            .values()
            .filter(|r| r.tenant_id == tenant_id && !r.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_active_rules(&self, tenant_id: i64) -> Result<Vec<ReconciliationRule>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .rules
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.is_active && !r.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_rule_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<ReconciliationRule>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .rules
            .get(&id)
            .filter(|r| r.tenant_id == tenant_id && !r.is_deleted())
            .cloned())
    }

    async fn update_rule(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateReconciliationRule,
    ) -> Result<ReconciliationRule, ApiError> {
        let mut inner = self.inner.lock();
        let rule = inner
            .rules
            .get_mut(&id)
            .filter(|r| !r.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Rule {} not found", id)))?;

        if rule.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Rule {} not found", id)));
        }

        if let Some(v) = update.rule_name {
            rule.rule_name = v;
        }
        if let Some(v) = update.match_field {
            rule.match_field = v;
        }
        if let Some(v) = update.match_pattern {
            rule.match_pattern = v;
        }
        if let Some(v) = update.auto_match {
            rule.auto_match = v;
        }
        if let Some(v) = update.is_active {
            rule.is_active = v;
        }

        rule.updated_at = Some(chrono::Utc::now());
        Ok(rule.clone())
    }

    async fn delete_rule(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let rule = inner
            .rules
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Rule {} not found", id)))?;

        if rule.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Rule {} not found", id)));
        }

        rule.mark_deleted(0); // system delete in in-memory repo
        Ok(())
    }
}
