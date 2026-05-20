//! PostgreSQL bank repository implementation

use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::bank::model::{
    BankAccount, BankStatement, BankTransaction, CreateBankAccount, CreateReconciliationRule,
    ImportBankStatement, MatchField, MatchStatus, ParsedBankTransaction, ReconciliationRule,
    StatementFormat, UpdateBankAccount, UpdateReconciliationRule,
};
use crate::domain::bank::repository::{BankRepository, BoxBankRepository};
use crate::error::ApiError;

/// Database row for bank accounts
#[derive(Debug, FromRow)]
struct BankAccountRow {
    id: i64,
    tenant_id: i64,
    company_id: Option<i64>,
    bank_code: String,
    account_number: String,
    iban: Option<String>,
    account_name: String,
    currency: String,
    branch_code: Option<String>,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<BankAccountRow> for BankAccount {
    fn from(row: BankAccountRow) -> Self {
        let bank_code = row.bank_code.parse().unwrap_or_else(|e| {
            tracing::warn!(
                error = %e,
                "Invalid bank_code in database, defaulting to Halkbank"
            );
            crate::domain::bank::model::BankCode::default()
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            company_id: row.company_id,
            bank_code,
            account_number: row.account_number,
            iban: row.iban,
            account_name: row.account_name,
            currency: row.currency,
            branch_code: row.branch_code,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// Database row for bank statements
#[derive(Debug, FromRow)]
struct BankStatementRow {
    id: i64,
    tenant_id: i64,
    account_id: i64,
    statement_date: NaiveDate,
    format: String,
    raw_data: String,
    processed: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<BankStatementRow> for BankStatement {
    fn from(row: BankStatementRow) -> Self {
        let format = row.format.parse().unwrap_or_else(|e| {
            tracing::warn!(
                error = %e,
                "Invalid format in database, defaulting to MT940"
            );
            StatementFormat::default()
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            account_id: row.account_id,
            statement_date: row.statement_date,
            format,
            raw_data: row.raw_data,
            processed: row.processed,
            created_at: row.created_at,
        }
    }
}

/// Database row for bank transactions
#[derive(Debug, FromRow)]
struct BankTransactionRow {
    id: i64,
    tenant_id: i64,
    account_id: i64,
    transaction_date: NaiveDate,
    description: String,
    amount: Decimal,
    currency: String,
    balance_after: Option<Decimal>,
    reference_no: Option<String>,
    matched_invoice_id: Option<i64>,
    matched_payment_id: Option<i64>,
    match_status: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<BankTransactionRow> for BankTransaction {
    fn from(row: BankTransactionRow) -> Self {
        let match_status = row.match_status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                error = %e,
                "Invalid match_status in database, defaulting to Unmatched"
            );
            MatchStatus::default()
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            account_id: row.account_id,
            transaction_date: row.transaction_date,
            description: row.description,
            amount: row.amount,
            currency: row.currency,
            balance_after: row.balance_after,
            reference_no: row.reference_no,
            matched_invoice_id: row.matched_invoice_id,
            matched_payment_id: row.matched_payment_id,
            match_status,
            created_at: row.created_at,
        }
    }
}

/// Database row for reconciliation rules
#[derive(Debug, FromRow)]
struct ReconciliationRuleRow {
    id: i64,
    tenant_id: i64,
    rule_name: String,
    match_field: String,
    match_pattern: String,
    auto_match: bool,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<ReconciliationRuleRow> for ReconciliationRule {
    fn from(row: ReconciliationRuleRow) -> Self {
        let match_field = row.match_field.parse().unwrap_or_else(|e| {
            tracing::warn!(
                error = %e,
                "Invalid match_field in database, defaulting to Description"
            );
            MatchField::default()
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            rule_name: row.rule_name,
            match_field,
            match_pattern: row.match_pattern,
            auto_match: row.auto_match,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL bank repository
pub struct PostgresBankRepository {
    pool: Arc<PgPool>,
}

impl PostgresBankRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxBankRepository {
        Arc::new(self) as BoxBankRepository
    }
}

#[async_trait]
impl BankRepository for PostgresBankRepository {
    async fn create_account(&self, create: CreateBankAccount) -> Result<BankAccount, ApiError> {
        let bank_code_str = create.bank_code.clone();

        let row: BankAccountRow = sqlx::query_as(
            r#"
            INSERT INTO bank_accounts (tenant_id, company_id, bank_code, account_number, iban,
                                       account_name, currency, branch_code, is_active, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            RETURNING id, tenant_id, company_id, bank_code, account_number, iban, account_name,
                      currency, branch_code, is_active, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(create.company_id)
        .bind(&bank_code_str)
        .bind(&create.account_number)
        .bind(&create.iban)
        .bind(&create.account_name)
        .bind(&create.currency)
        .bind(&create.branch_code)
        .bind(create.is_active)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Bank account"))?;

        Ok(row.into())
    }

    async fn find_account_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<BankAccount>, ApiError> {
        let result: Option<BankAccountRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, company_id, bank_code, account_number, iban, account_name,
                   currency, branch_code, is_active, created_at, updated_at, deleted_at, deleted_by
            FROM bank_accounts
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find bank account: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_accounts(&self, tenant_id: i64) -> Result<Vec<BankAccount>, ApiError> {
        let rows: Vec<BankAccountRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, company_id, bank_code, account_number, iban, account_name,
                   currency, branch_code, is_active, created_at, updated_at, deleted_at, deleted_by
            FROM bank_accounts
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find bank accounts: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_account(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateBankAccount,
    ) -> Result<BankAccount, ApiError> {
        let row: BankAccountRow = sqlx::query_as(
            r#"
            UPDATE bank_accounts
            SET
                account_number = COALESCE($1, account_number),
                account_name = COALESCE($2, account_name),
                currency = COALESCE($3, currency),
                iban = COALESCE($4, iban),
                branch_code = COALESCE($5, branch_code),
                is_active = COALESCE($6, is_active),
                company_id = COALESCE($7, company_id),
                updated_at = NOW()
            WHERE id = $8 AND tenant_id = $9 AND deleted_at IS NULL
            RETURNING id, tenant_id, company_id, bank_code, account_number, iban, account_name,
                      currency, branch_code, is_active, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(&update.account_number)
        .bind(&update.account_name)
        .bind(&update.currency)
        .bind(&update.iban)
        .bind(&update.branch_code)
        .bind(update.is_active)
        .bind(update.company_id)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Bank account"))?;

        Ok(row.into())
    }

    async fn soft_delete_account(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE bank_accounts
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete bank account: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Bank account not found".to_string()));
        }

        Ok(())
    }

    async fn restore_account(&self, id: i64, tenant_id: i64) -> Result<BankAccount, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE bank_accounts
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore bank account: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Bank account not found or not deleted".to_string(),
            ));
        }

        self.find_account_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Bank account not found".to_string()))
    }

    async fn destroy_account(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM bank_accounts
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy bank account: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Bank account not found".to_string()));
        }

        Ok(())
    }

    async fn create_statement(
        &self,
        tenant_id: i64,
        account_id: i64,
        import: ImportBankStatement,
    ) -> Result<BankStatement, ApiError> {
        let format_str = import.format.to_string();
        let statement_date = chrono::Utc::now().date_naive();

        let row: BankStatementRow = sqlx::query_as(
            r#"
            INSERT INTO bank_statements (tenant_id, account_id, statement_date, format, raw_data, processed, created_at)
            VALUES ($1, $2, $3, $4, $5, FALSE, NOW())
            RETURNING id, tenant_id, account_id, statement_date, format, raw_data, processed, created_at
            "#,
        )
        .bind(tenant_id)
        .bind(account_id)
        .bind(statement_date)
        .bind(&format_str)
        .bind(&import.data)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Bank statement"))?;

        Ok(row.into())
    }

    async fn mark_statement_processed(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE bank_statements
            SET processed = TRUE
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to mark statement processed: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Bank statement not found".to_string()));
        }

        Ok(())
    }

    async fn create_transaction(
        &self,
        tenant_id: i64,
        account_id: i64,
        tx: ParsedBankTransaction,
    ) -> Result<BankTransaction, ApiError> {
        let match_status = MatchStatus::Unmatched.to_string();

        let row: BankTransactionRow = sqlx::query_as(
            r#"
            INSERT INTO bank_transactions (tenant_id, account_id, transaction_date, description, amount,
                                           currency, balance_after, reference_no, match_status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            RETURNING id, tenant_id, account_id, transaction_date, description, amount, currency,
                      balance_after, reference_no, matched_invoice_id, matched_payment_id, match_status, created_at
            "#,
        )
        .bind(tenant_id)
        .bind(account_id)
        .bind(tx.transaction_date)
        .bind(&tx.description)
        .bind(tx.amount)
        .bind(&tx.currency)
        .bind(tx.balance_after)
        .bind(&tx.reference_no)
        .bind(&match_status)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Bank transaction"))?;

        Ok(row.into())
    }

    async fn find_transactions_by_account(
        &self,
        account_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError> {
        let rows: Vec<BankTransactionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, account_id, transaction_date, description, amount, currency,
                   balance_after, reference_no, matched_invoice_id, matched_payment_id, match_status, created_at
            FROM bank_transactions
            WHERE account_id = $1 AND tenant_id = $2
            ORDER BY transaction_date DESC, id DESC
            "#,
        )
        .bind(account_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find transactions: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_unmatched_transactions(
        &self,
        account_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError> {
        let rows: Vec<BankTransactionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, account_id, transaction_date, description, amount, currency,
                   balance_after, reference_no, matched_invoice_id, matched_payment_id, match_status, created_at
            FROM bank_transactions
            WHERE account_id = $1 AND tenant_id = $2 AND match_status = 'unmatched'
            ORDER BY transaction_date DESC, id DESC
            "#,
        )
        .bind(account_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find unmatched transactions: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_all_unmatched_transactions(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError> {
        let rows: Vec<BankTransactionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, account_id, transaction_date, description, amount, currency,
                   balance_after, reference_no, matched_invoice_id, matched_payment_id, match_status, created_at
            FROM bank_transactions
            WHERE tenant_id = $1 AND match_status = 'unmatched'
            ORDER BY transaction_date DESC, id DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find unmatched transactions: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_transaction_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<BankTransaction>, ApiError> {
        let result: Option<BankTransactionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, account_id, transaction_date, description, amount, currency,
                   balance_after, reference_no, matched_invoice_id, matched_payment_id, match_status, created_at
            FROM bank_transactions
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find transaction: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn update_transaction_match(
        &self,
        id: i64,
        tenant_id: i64,
        invoice_id: Option<i64>,
        payment_id: Option<i64>,
        status: MatchStatus,
    ) -> Result<BankTransaction, ApiError> {
        let status_str = status.to_string();

        let row: BankTransactionRow = sqlx::query_as(
            r#"
            UPDATE bank_transactions
            SET matched_invoice_id = $1, matched_payment_id = $2, match_status = $3
            WHERE id = $4 AND tenant_id = $5
            RETURNING id, tenant_id, account_id, transaction_date, description, amount, currency,
                      balance_after, reference_no, matched_invoice_id, matched_payment_id, match_status, created_at
            "#,
        )
        .bind(invoice_id)
        .bind(payment_id)
        .bind(&status_str)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Bank transaction"))?;

        Ok(row.into())
    }

    async fn unmatch_transaction(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<BankTransaction, ApiError> {
        let row: BankTransactionRow = sqlx::query_as(
            r#"
            UPDATE bank_transactions
            SET matched_invoice_id = NULL, matched_payment_id = NULL, match_status = 'unmatched'
            WHERE id = $1 AND tenant_id = $2
            RETURNING id, tenant_id, account_id, transaction_date, description, amount, currency,
                      balance_after, reference_no, matched_invoice_id, matched_payment_id, match_status, created_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Bank transaction"))?;

        Ok(row.into())
    }

    async fn find_transactions_by_reference(
        &self,
        reference: &str,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError> {
        let rows: Vec<BankTransactionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, account_id, transaction_date, description, amount, currency,
                   balance_after, reference_no, matched_invoice_id, matched_payment_id, match_status, created_at
            FROM bank_transactions
            WHERE tenant_id = $1 AND reference_no = $2 AND match_status = 'unmatched'
            ORDER BY transaction_date DESC
            "#,
        )
        .bind(tenant_id)
        .bind(reference)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find transactions by reference: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_transactions_by_amount_date(
        &self,
        amount: Decimal,
        date: NaiveDate,
        tenant_id: i64,
    ) -> Result<Vec<BankTransaction>, ApiError> {
        let rows: Vec<BankTransactionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, account_id, transaction_date, description, amount, currency,
                   balance_after, reference_no, matched_invoice_id, matched_payment_id, match_status, created_at
            FROM bank_transactions
            WHERE tenant_id = $1 AND amount = $2 AND transaction_date BETWEEN $3 - INTERVAL '1 day' AND $3 + INTERVAL '1 day'
              AND match_status = 'unmatched'
            ORDER BY ABS(EXTRACT(EPOCH FROM (transaction_date - $3)))
            "#,
        )
        .bind(tenant_id)
        .bind(amount)
        .bind(date)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find transactions by amount/date: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_reconciliation_summary(
        &self,
        tenant_id: i64,
    ) -> Result<(i64, i64, i64, Decimal, Decimal), ApiError> {
        let row: (i64, i64, i64, Decimal, Decimal) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE match_status = 'matched') as matched,
                COUNT(*) FILTER (WHERE match_status = 'unmatched') as unmatched,
                COALESCE(SUM(amount), 0) as total_amount,
                COALESCE(SUM(amount) FILTER (WHERE match_status = 'matched'), 0) as matched_amount
            FROM bank_transactions
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get reconciliation summary: {}", e)))?;

        Ok(row)
    }

    async fn create_rule(
        &self,
        create: CreateReconciliationRule,
    ) -> Result<ReconciliationRule, ApiError> {
        let match_field_str = create.match_field.to_string();

        let row: ReconciliationRuleRow = sqlx::query_as(
            r#"
            INSERT INTO reconciliation_rules (tenant_id, rule_name, match_field, match_pattern, auto_match, is_active, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            RETURNING id, tenant_id, rule_name, match_field, match_pattern, auto_match, is_active, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.rule_name)
        .bind(&match_field_str)
        .bind(&create.match_pattern)
        .bind(create.auto_match)
        .bind(create.is_active)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Reconciliation rule"))?;

        Ok(row.into())
    }

    async fn find_rules(&self, tenant_id: i64) -> Result<Vec<ReconciliationRule>, ApiError> {
        let rows: Vec<ReconciliationRuleRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, rule_name, match_field, match_pattern, auto_match, is_active, created_at, updated_at, deleted_at, deleted_by
            FROM reconciliation_rules
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find rules: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_active_rules(&self, tenant_id: i64) -> Result<Vec<ReconciliationRule>, ApiError> {
        let rows: Vec<ReconciliationRuleRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, rule_name, match_field, match_pattern, auto_match, is_active, created_at, updated_at, deleted_at, deleted_by
            FROM reconciliation_rules
            WHERE tenant_id = $1 AND is_active = TRUE AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find active rules: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_rule_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<ReconciliationRule>, ApiError> {
        let result: Option<ReconciliationRuleRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, rule_name, match_field, match_pattern, auto_match, is_active, created_at, updated_at, deleted_at, deleted_by
            FROM reconciliation_rules
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find rule: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn update_rule(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateReconciliationRule,
    ) -> Result<ReconciliationRule, ApiError> {
        let match_field_str = update.match_field.map(|f| f.to_string());

        let row: ReconciliationRuleRow = sqlx::query_as(
            r#"
            UPDATE reconciliation_rules
            SET
                rule_name = COALESCE($1, rule_name),
                match_field = COALESCE($2, match_field),
                match_pattern = COALESCE($3, match_pattern),
                auto_match = COALESCE($4, auto_match),
                is_active = COALESCE($5, is_active),
                updated_at = NOW()
            WHERE id = $6 AND tenant_id = $7 AND deleted_at IS NULL
            RETURNING id, tenant_id, rule_name, match_field, match_pattern, auto_match, is_active, created_at, updated_at, deleted_at, deleted_by
            "#,
        )
        .bind(&update.rule_name)
        .bind(&match_field_str)
        .bind(&update.match_pattern)
        .bind(update.auto_match)
        .bind(update.is_active)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Reconciliation rule"))?;

        Ok(row.into())
    }

    async fn delete_rule(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE reconciliation_rules
            SET deleted_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete rule: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Reconciliation rule not found".to_string(),
            ));
        }

        Ok(())
    }
}
