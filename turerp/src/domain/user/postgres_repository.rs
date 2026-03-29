//! PostgreSQL user repository implementation

use async_trait::async_trait;
use sqlx::PgPool;
use std::sync::Arc;

use crate::domain::user::model::{CreateUser, UpdateUser, User};
use crate::domain::user::repository::{BoxUserRepository, UserRepository};
use crate::error::ApiError;

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

        let row = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (username, email, full_name, password, tenant_id, role, is_active, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING id, username, email, full_name, password as hashed_password, tenant_id,
                      role, is_active, created_at, updated_at
            "#,
            create.username,
            create.email,
            create.full_name,
            hashed_password,
            create.tenant_id,
            role_str,
            true
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                ApiError::Conflict("Username or email already exists".to_string())
            } else {
                ApiError::Database(format!("Failed to create user: {}", e))
            }
        })?;

        Ok(row)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<User>, ApiError> {
        let result = sqlx::query_as!(
            User,
            r#"
            SELECT id, username, email, full_name, password as hashed_password, tenant_id,
                   role, is_active, created_at, updated_at
            FROM users
            WHERE id = $1 AND tenant_id = $2
            "#,
            id,
            tenant_id
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find user by id: {}", e)))?;

        Ok(result)
    }

    async fn find_by_username(
        &self,
        username: &str,
        tenant_id: i64,
    ) -> Result<Option<User>, ApiError> {
        let result = sqlx::query_as!(
            User,
            r#"
            SELECT id, username, email, full_name, password as hashed_password, tenant_id,
                   role, is_active, created_at, updated_at
            FROM users
            WHERE username = $1 AND tenant_id = $2
            "#,
            username,
            tenant_id
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find user by username: {}", e)))?;

        Ok(result)
    }

    async fn find_by_email(&self, email: &str, tenant_id: i64) -> Result<Option<User>, ApiError> {
        let result = sqlx::query_as!(
            User,
            r#"
            SELECT id, username, email, full_name, password as hashed_password, tenant_id,
                   role, is_active, created_at, updated_at
            FROM users
            WHERE email = $1 AND tenant_id = $2
            "#,
            email,
            tenant_id
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find user by email: {}", e)))?;

        Ok(result)
    }

    async fn find_all(&self, tenant_id: i64) -> Result<Vec<User>, ApiError> {
        let users = sqlx::query_as!(
            User,
            r#"
            SELECT id, username, email, full_name, password as hashed_password, tenant_id,
                   role, is_active, created_at, updated_at
            FROM users
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
            tenant_id
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find all users: {}", e)))?;

        Ok(users)
    }

    async fn update(&self, id: i64, tenant_id: i64, update: UpdateUser) -> Result<User, ApiError> {
        // Build dynamic update query
        let user = sqlx::query_as!(
            User,
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
            RETURNING id, username, email, full_name, password as hashed_password, tenant_id,
                      role, is_active, created_at, updated_at
            "#,
            update.username,
            update.email,
            update.full_name,
            update.is_active,
            update.role.map(|r| r.to_string()),
            id,
            tenant_id
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("no rows") {
                ApiError::NotFound("User not found".to_string())
            } else {
                ApiError::Database(format!("Failed to update user: {}", e))
            }
        })?;

        Ok(user)
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1 AND tenant_id = $2
            "#,
            id,
            tenant_id
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete user: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("User not found".to_string()));
        }

        Ok(())
    }

    async fn username_exists(&self, username: &str, tenant_id: i64) -> Result<bool, ApiError> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 AND tenant_id = $2) as exists
            "#,
            username,
            tenant_id
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to check username: {}", e)))?;

        Ok(result.exists.unwrap_or(false))
    }

    async fn email_exists(&self, email: &str, tenant_id: i64) -> Result<bool, ApiError> {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS(SELECT 1 FROM users WHERE email = $1 AND tenant_id = $2) as exists
            "#,
            email,
            tenant_id
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to check email: {}", e)))?;

        Ok(result.exists.unwrap_or(false))
    }
}
