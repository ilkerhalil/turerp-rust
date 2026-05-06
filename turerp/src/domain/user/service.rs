//! User service for business logic

use validator::Validate;

use crate::common::pagination::PaginatedResult;
use crate::domain::user::model::{CreateUser, UpdateUser, User, UserResponse};
use crate::domain::user::repository::BoxUserRepository;
use crate::error::ApiError;

/// User service
#[derive(Clone)]
pub struct UserService {
    repo: BoxUserRepository,
}

impl UserService {
    pub fn new(repo: BoxUserRepository) -> Self {
        Self { repo }
    }

    /// Create a new user
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

        Ok(user.into())
    }

    /// Get user by ID
    pub async fn get_user(&self, id: i64, tenant_id: i64) -> Result<UserResponse, ApiError> {
        let user = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("User {} not found", id)))?;

        Ok(user.into())
    }

    /// Get user by username
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
    pub async fn get_all_users(&self, tenant_id: i64) -> Result<Vec<UserResponse>, ApiError> {
        let users = self.repo.find_all(tenant_id).await?;
        Ok(users.into_iter().map(|u| u.into()).collect())
    }

    /// Get all users for a tenant with pagination
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
        Ok(user.into())
    }

    /// Delete a user
    pub async fn delete_user(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.delete(id, tenant_id).await
    }

    /// Soft delete a user
    pub async fn soft_delete_user(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Restore a soft-deleted user
    pub async fn restore_user(&self, id: i64, tenant_id: i64) -> Result<UserResponse, ApiError> {
        let user = self.repo.restore(id, tenant_id).await?;
        Ok(user.into())
    }

    /// List all deleted users for a tenant
    pub async fn list_deleted_users(&self, tenant_id: i64) -> Result<Vec<UserResponse>, ApiError> {
        let users = self.repo.find_deleted(tenant_id).await?;
        Ok(users.into_iter().map(|u| u.into()).collect())
    }

    /// Permanently destroy a user
    pub async fn destroy_user(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id, tenant_id).await
    }

    /// Verify user credentials
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
            .map_err(|_| ApiError::InvalidCredentials)?;

        Ok(user)
    }
}
