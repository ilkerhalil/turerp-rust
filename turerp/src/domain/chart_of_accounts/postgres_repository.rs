//! PostgreSQL chart of accounts repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::db::error::map_sqlx_error;
use crate::domain::accounting::model::AccountType;
use crate::domain::chart_of_accounts::model::{
    AccountGroup, ChartAccount, CreateChartAccount, UpdateChartAccount,
};
use crate::domain::chart_of_accounts::repository::{
    BoxChartAccountRepository, ChartAccountRepository,
};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// ChartAccountRow / ChartAccount conversion
// ---------------------------------------------------------------------------

/// Database row representation for ChartAccount
#[derive(Debug, FromRow)]
struct ChartAccountRow {
    id: i64,
    tenant_id: i64,
    code: String,
    name: String,
    group_name: String,
    parent_code: Option<String>,
    level: i16,
    account_type: String,
    is_active: bool,
    balance: Decimal,
    allow_posting: bool,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
    total_count: Option<i64>,
}

impl From<ChartAccountRow> for ChartAccount {
    fn from(row: ChartAccountRow) -> Self {
        let group = row.group_name.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid group_name '{}' in database: {}, defaulting to DonenVarliklar",
                row.group_name,
                e
            );
            AccountGroup::DonenVarliklar
        });

        let account_type = row.account_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid account_type '{}' in database: {}, defaulting to Expense",
                row.account_type,
                e
            );
            AccountType::Expense
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            code: row.code,
            name: row.name,
            group,
            parent_code: row.parent_code,
            level: row.level as u8,
            account_type,
            is_active: row.is_active,
            balance: row.balance,
            allow_posting: row.allow_posting,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

// ===========================================================================
// PostgresChartAccountRepository
// ===========================================================================

/// PostgreSQL chart account repository
pub struct PostgresChartAccountRepository {
    pool: Arc<PgPool>,
}

impl PostgresChartAccountRepository {
    /// Create a new PostgreSQL chart account repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxChartAccountRepository {
        Arc::new(self) as BoxChartAccountRepository
    }
}

/// Common column list for SELECT queries
const CHART_ACCOUNT_COLUMNS: &str = r#"
    id, tenant_id, code, name, group_name, parent_code, level,
    account_type, is_active, balance, allow_posting,
    created_at, updated_at, deleted_at, deleted_by
"#;

#[async_trait]
impl ChartAccountRepository for PostgresChartAccountRepository {
    async fn create(
        &self,
        create: CreateChartAccount,
        tenant_id: i64,
    ) -> Result<ChartAccount, ApiError> {
        let group_name = create.group.to_string();
        let account_type = create.account_type.to_string();

        let row: ChartAccountRow = sqlx::query_as(&format!(
            r#"
            INSERT INTO chart_accounts (tenant_id, code, name, group_name, parent_code, level,
                                         account_type, is_active, balance, allow_posting,
                                         created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 1, $6, true, 0, $7, NOW(), NOW())
            RETURNING {CHART_ACCOUNT_COLUMNS}
            "#,
        ))
        .bind(tenant_id)
        .bind(&create.code)
        .bind(&create.name)
        .bind(&group_name)
        .bind(&create.parent_code)
        .bind(&account_type)
        .bind(create.allow_posting)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ChartAccount"))?;

        Ok(row.into())
    }

    async fn find_by_code(
        &self,
        code: &str,
        tenant_id: i64,
    ) -> Result<Option<ChartAccount>, ApiError> {
        let result: Option<ChartAccountRow> = sqlx::query_as(&format!(
            r#"
            SELECT {CHART_ACCOUNT_COLUMNS}
            FROM chart_accounts
            WHERE tenant_id = $1 AND code = $2 AND deleted_at IS NULL
            "#,
        ))
        .bind(tenant_id)
        .bind(code)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find chart account by code: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ChartAccount>, ApiError> {
        let result: Option<ChartAccountRow> = sqlx::query_as(&format!(
            r#"
            SELECT {CHART_ACCOUNT_COLUMNS}
            FROM chart_accounts
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find chart account by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        group: Option<AccountGroup>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ChartAccount>, ApiError> {
        let offset = params.offset() as i64;
        let per_page = params.per_page as i64;

        match group {
            Some(g) => {
                let group_name = g.to_string();
                let rows: Vec<ChartAccountRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {CHART_ACCOUNT_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM chart_accounts
                    WHERE tenant_id = $1 AND group_name = $2 AND deleted_at IS NULL
                    ORDER BY code ASC
                    LIMIT $3 OFFSET $4
                    "#,
                ))
                .bind(tenant_id)
                .bind(&group_name)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "ChartAccount"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<ChartAccount> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
            None => {
                let rows: Vec<ChartAccountRow> = sqlx::query_as(&format!(
                    r#"
                    SELECT {CHART_ACCOUNT_COLUMNS},
                           COUNT(*) OVER() as total_count
                    FROM chart_accounts
                    WHERE tenant_id = $1 AND deleted_at IS NULL
                    ORDER BY code ASC
                    LIMIT $2 OFFSET $3
                    "#,
                ))
                .bind(tenant_id)
                .bind(per_page)
                .bind(offset)
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| map_sqlx_error(e, "ChartAccount"))?;

                let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
                let items: Vec<ChartAccount> = rows.into_iter().map(|r| r.into()).collect();
                Ok(PaginatedResult::new(
                    items,
                    params.page,
                    params.per_page,
                    total,
                ))
            }
        }
    }

    async fn find_children(
        &self,
        parent_code: &str,
        tenant_id: i64,
    ) -> Result<Vec<ChartAccount>, ApiError> {
        let rows: Vec<ChartAccountRow> = sqlx::query_as(&format!(
            r#"
            SELECT {CHART_ACCOUNT_COLUMNS}
            FROM chart_accounts
            WHERE tenant_id = $1 AND parent_code = $2 AND deleted_at IS NULL
            ORDER BY code ASC
            "#,
        ))
        .bind(tenant_id)
        .bind(parent_code)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find chart account children: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateChartAccount,
    ) -> Result<ChartAccount, ApiError> {
        let group_name = update.group.map(|g| g.to_string());

        let row: ChartAccountRow = sqlx::query_as(&format!(
            r#"
            UPDATE chart_accounts
            SET
                name = COALESCE($1, name),
                group_name = COALESCE($2, group_name),
                is_active = COALESCE($3, is_active),
                allow_posting = COALESCE($4, allow_posting),
                updated_at = NOW()
            WHERE id = $5 AND tenant_id = $6 AND deleted_at IS NULL
            RETURNING {CHART_ACCOUNT_COLUMNS}
            "#,
        ))
        .bind(&update.name)
        .bind(&group_name)
        .bind(update.is_active)
        .bind(update.allow_posting)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ChartAccount"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE chart_accounts
            SET deleted_at = NOW(), deleted_by = $3, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete chart account: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Chart account not found".to_string()));
        }

        Ok(())
    }

    async fn update_balance(
        &self,
        id: i64,
        tenant_id: i64,
        balance: Decimal,
    ) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE chart_accounts
            SET balance = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3 AND deleted_at IS NULL
            "#,
        )
        .bind(balance)
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to update chart account balance: {}", e))
        })?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Chart account not found".to_string()));
        }

        Ok(())
    }
}
