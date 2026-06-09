//! User service for business logic

use std::sync::Arc;

use validator::Validate;

use crate::cache::{cache_get, cache_key, cache_set, CacheService};
use crate::common::pagination::PaginatedResult;
use crate::domain::user::model::{CreateUser, Role, UpdateUser, User, UserResponse};
use crate::domain::user::repository::BoxUserRepository;
use crate::error::ApiError;
use serde::{Deserialize, Serialize};

/// Permissions derived from a user's role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissions {
    pub user_id: i64,
    pub tenant_id: i64,
    pub role: Role,
    pub can_read: bool,
    pub can_write: bool,
    pub can_delete: bool,
    pub can_admin: bool,
}

impl UserPermissions {
    /// Derive permissions from a user
    pub fn from_user(user: &User) -> Self {
        let (can_write, can_delete, can_admin) = match user.role {
            Role::Admin => (true, true, true),
            Role::User => (true, false, false),
            Role::Viewer => (false, false, false),
        };
        Self {
            user_id: user.id,
            tenant_id: user.tenant_id,
            role: user.role,
            can_read: true,
            can_write,
            can_delete,
            can_admin,
        }
    }
}

/// User service
#[derive(Clone)]
pub struct UserService {
    repo: BoxUserRepository,
    cache: Option<Arc<dyn CacheService>>,
}

/// TTL for permissions cache entries (seconds)
const PERMISSIONS_TTL: u64 = 300;

impl UserService {
    pub fn new(repo: BoxUserRepository) -> Self {
        Self { repo, cache: None }
    }

    /// Attach a cache service for permissions caching
    pub fn with_cache(mut self, cache: Arc<dyn CacheService>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Invalidate cached permissions for a user
    async fn invalidate_permissions_cache(&self, tenant_id: i64, user_id: i64) {
        if let Some(ref cache) = self.cache {
            let key = cache_key(tenant_id, "permissions", &user_id.to_string());
            cache.delete(&key).await.ok();
        }
    }

    /// Create a new user
    #[tracing::instrument(skip(self))]
    pub async fn create_user(&self, create: CreateUser) -> Result<UserResponse, ApiError> {
        // Validate password complexity first
        create.validate_password().map_err(ApiError::Validation)?;

        // Validate other input fields
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;

        // Check if username exists
        if self
            .repo
            .username_exists(&create.username, create.tenant_id)
            .await?
        {
            return Err(ApiError::Conflict("Username already exists".to_string()));
        }

        // Check if email exists
        if self
            .repo
            .email_exists(&create.email, create.tenant_id)
            .await?
        {
            return Err(ApiError::Conflict("Email already exists".to_string()));
        }

        // Hash password
        let hashed_password = crate::utils::password::hash_password(&create.password)
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        // Create user
        let user = self.repo.create(create, hashed_password).await?;

        // Invalidate any cached permissions for this tenant (new user may affect lists)
        self.invalidate_permissions_cache(user.tenant_id, user.id)
            .await;

        Ok(user.into())
    }

    /// Get user by ID
    #[tracing::instrument(skip(self))]
    pub async fn get_user(&self, id: i64, tenant_id: i64) -> Result<UserResponse, ApiError> {
        let user = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("User {} not found", id)))?;

        Ok(user.into())
    }

    /// Get cached permissions for a user
    #[tracing::instrument(skip(self))]
    pub async fn get_user_permissions(
        &self,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<UserPermissions, ApiError> {
        let cache_key = cache_key(tenant_id, "permissions", &user_id.to_string());

        // Try cache first
        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache_get::<UserPermissions>(&**cache, &cache_key).await? {
                return Ok(cached);
            }
        }

        let user = self
            .repo
            .find_by_id(user_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("User {} not found", user_id)))?;

        let permissions = UserPermissions::from_user(&user);

        // Store in cache
        if let Some(ref cache) = self.cache {
            cache_set(&**cache, &cache_key, &permissions, Some(PERMISSIONS_TTL))
                .await
                .ok();
        }

        Ok(permissions)
    }

    /// Get user by username
    #[tracing::instrument(skip(self))]
    pub async fn get_user_by_username(
        &self,
        username: &str,
        tenant_id: i64,
    ) -> Result<User, ApiError> {
        self.repo
            .find_by_username(username, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("User {} not found", username)))
    }

    /// Get all users for a tenant
    #[tracing::instrument(skip(self))]
    pub async fn get_all_users(&self, tenant_id: i64) -> Result<Vec<UserResponse>, ApiError> {
        let users = self.repo.find_all(tenant_id).await?;
        Ok(users.into_iter().map(|u| u.into()).collect())
    }

    /// Get all users for a tenant with pagination
    #[tracing::instrument(skip(self))]
    pub async fn get_all_users_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<UserResponse>, ApiError> {
        let result = self
            .repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await?;
        Ok(PaginatedResult::new(
            result.items.into_iter().map(|u| u.into()).collect(),
            result.page,
            result.per_page,
            result.total,
        ))
    }

    /// Update a user
    #[tracing::instrument(skip(self))]
    pub async fn update_user(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateUser,
    ) -> Result<UserResponse, ApiError> {
        // Validate input
        update
            .validate()
            .map_err(|e: validator::ValidationErrors| ApiError::Validation(e.to_string()))?;

        // Check if username changed and exists
        if let Some(ref username) = update.username {
            let existing = self.repo.find_by_username(username, tenant_id).await?;
            if let Some(u) = existing {
                if u.id != id {
                    return Err(ApiError::Conflict("Username already exists".to_string()));
                }
            }
        }

        // Check if email changed and exists
        if let Some(ref email) = update.email {
            let existing = self.repo.find_by_email(email, tenant_id).await?;
            if let Some(u) = existing {
                if u.id != id {
                    return Err(ApiError::Conflict("Email already exists".to_string()));
                }
            }
        }

        let user = self.repo.update(id, tenant_id, update).await?;

        // Invalidate cached permissions since role may have changed
        self.invalidate_permissions_cache(tenant_id, id).await;

        Ok(user.into())
    }

    /// Delete a user
    #[tracing::instrument(skip(self))]
    pub async fn delete_user(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.delete(id, tenant_id).await?;
        self.invalidate_permissions_cache(tenant_id, id).await;
        Ok(())
    }

    /// Soft delete a user
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_user(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete(id, tenant_id, deleted_by).await?;
        self.invalidate_permissions_cache(tenant_id, id).await;
        Ok(())
    }

    /// Restore a soft-deleted user
    #[tracing::instrument(skip(self))]
    pub async fn restore_user(&self, id: i64, tenant_id: i64) -> Result<UserResponse, ApiError> {
        let user = self.repo.restore(id, tenant_id).await?;
        self.invalidate_permissions_cache(tenant_id, id).await;
        Ok(user.into())
    }

    /// List all deleted users for a tenant
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_users(&self, tenant_id: i64) -> Result<Vec<UserResponse>, ApiError> {
        let users = self.repo.find_deleted(tenant_id).await?;
        Ok(users.into_iter().map(|u| u.into()).collect())
    }

    /// Permanently destroy a user
    #[tracing::instrument(skip(self))]
    pub async fn destroy_user(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id, tenant_id).await?;
        self.invalidate_permissions_cache(tenant_id, id).await;
        Ok(())
    }

    /// Verify user credentials
    #[tracing::instrument(skip(self))]
    pub async fn verify_credentials(
        &self,
        username: &str,
        password: &str,
        tenant_id: i64,
    ) -> Result<User, ApiError> {
        let user = self
            .repo
            .find_by_username(username, tenant_id)
            .await?
            .ok_or(ApiError::InvalidCredentials)?;

        if !user.is_active {
            return Err(ApiError::InvalidCredentials);
        }

        crate::utils::password::verify_password(password, &user.hashed_password)
            .map_err(|_| ApiError::InvalidCredentials)
            .and_then(|valid| {
                if valid {
                    Ok(())
                } else {
                    Err(ApiError::InvalidCredentials)
                }
            })?;

        Ok(user)
    }
}
