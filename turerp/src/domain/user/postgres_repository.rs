//! PostgreSQL user repository implementation

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::user::model::{CreateUser, Role, UpdateUser, User};
use crate::domain::user::repository::{BoxUserRepository, UserRepository};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

/// Database row representation for User
#[derive(Debug, FromRow)]
struct UserRow {
    id: i64,
    username: String,
    email: String,
    full_name: String,
    password: String,
    tenant_id: i64,
    role: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        // Parse role with warning for invalid values
        let role = row.role.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid role '{}' in database: {}, defaulting to User",
                row.role,
                e
            );
            Role::default()
        });

        Self {
            id: row.id,
            username: row.username,
            email: row.email,
            full_name: row.full_name,
            hashed_password: row.password,
            tenant_id: row.tenant_id,
            role,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Database row representation for paginated user queries with total count
#[derive(Debug, FromRow)]
struct UserRowWithTotal {
    id: i64,
    username: String,
    email: String,
    full_name: String,
    password: String,
    tenant_id: i64,
    role: String,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    total_count: i64,
}

impl From<UserRowWithTotal> for (User, i64) {
    fn from(row: UserRowWithTotal) -> (User, i64) {
        let role = row.role.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid role '{}' in database: {}, defaulting to User",
                row.role,
                e
            );
            Role::default()
        });

        let user = User {
            id: row.id,
            username: row.username,
            email: row.email,
            full_name: row.full_name,
            hashed_password: row.password,
            tenant_id: row.tenant_id,
            role,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        };
        (user, row.total_count)
    }
}

/// PostgreSQL user repository
pub struct PostgresUserRepository {
    pool: Arc<PgPool>,
}

impl PostgresUserRepository {
    /// Create a new PostgreSQL user repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxUserRepository {
        Arc::new(self) as BoxUserRepository
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn create(&self, create: CreateUser, hashed_password: String) -> Result<User, ApiError> {
        let role = create.role.unwrap_or_default();
        let role_str = role.to_string();

        let row: UserRow = sqlx::query_as(
            r#"
            INSERT INTO users (username, email, full_name, password, tenant_id, role, is_active, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING id, username, email, full_name, password, tenant_id, role, is_active, created_at, updated_at
            "#,
        )
        .bind(&create.username)
        .bind(&create.email)
        .bind(&create.full_name)
        .bind(&hashed_password)
        .bind(create.tenant_id)
        .bind(&role_str)
        .bind(true)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "User"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<User>, ApiError> {
        let result: Option<UserRow> = sqlx::query_as(
            r#"
            SELECT id, username, email, full_name, password, tenant_id, role, is_active, created_at, updated_at
            FROM users
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find user by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_username(
        &self,
        username: &str,
        tenant_id: i64,
    ) -> Result<Option<User>, ApiError> {
        let result: Option<UserRow> = sqlx::query_as(
            r#"
            SELECT id, username, email, full_name, password, tenant_id, role, is_active, created_at, updated_at
            FROM users
            WHERE username = $1 AND tenant_id = $2
            "#,
        )
        .bind(username)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find user by username: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_email(&self, email: &str, tenant_id: i64) -> Result<Option<User>, ApiError> {
        let result: Option<UserRow> = sqlx::query_as(
            r#"
            SELECT id, username, email, full_name, password, tenant_id, role, is_active, created_at, updated_at
            FROM users
            WHERE email = $1 AND tenant_id = $2
            "#,
        )
        .bind(email)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find user by email: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_all(&self, tenant_id: i64) -> Result<Vec<User>, ApiError> {
        let rows: Vec<UserRow> = sqlx::query_as(
            r#"
            SELECT id, username, email, full_name, password, tenant_id, role, is_active, created_at, updated_at
            FROM users
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find all users: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<User>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;

        let rows: Vec<UserRowWithTotal> = sqlx::query_as(
            r#"
            SELECT id, username, email, full_name, password, tenant_id, role, is_active, created_at, updated_at,
                   COUNT(*) OVER() as total_count
            FROM users
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
        .map_err(|e| map_sqlx_error(e, "User"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<User> = rows
            .into_iter()
            .map(|r| r.into())
            .map(|(user, _)| user)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(&self, id: i64, tenant_id: i64, update: UpdateUser) -> Result<User, ApiError> {
        let role_str = update.role.map(|r| r.to_string());

        let row: UserRow = sqlx::query_as(
            r#"
            UPDATE users
            SET
                username = COALESCE($1, username),
                email = COALESCE($2, email),
                full_name = COALESCE($3, full_name),
                is_active = COALESCE($4, is_active),
                role = COALESCE($5, role),
                updated_at = NOW()
            WHERE id = $6 AND tenant_id = $7
            RETURNING id, username, email, full_name, password, tenant_id, role, is_active, created_at, updated_at
            "#,
        )
        .bind(&update.username)
        .bind(&update.email)
        .bind(&update.full_name)
        .bind(update.is_active)
        .bind(&role_str)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "User"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM users
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete user: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("User not found".to_string()));
        }

        Ok(())
    }

    async fn username_exists(&self, username: &str, tenant_id: i64) -> Result<bool, ApiError> {
        let result: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 AND tenant_id = $2)
            "#,
        )
        .bind(username)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to check username: {}", e)))?;

        Ok(result.0)
    }

    async fn email_exists(&self, email: &str, tenant_id: i64) -> Result<bool, ApiError> {
        let result: (bool,) = sqlx::query_as(
            r#"
            SELECT EXISTS(SELECT 1 FROM users WHERE email = $1 AND tenant_id = $2)
            "#,
        )
        .bind(email)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to check email: {}", e)))?;

        Ok(result.0)
    }
}
