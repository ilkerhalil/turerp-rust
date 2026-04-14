//! User repository trait and implementations

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::domain::user::model::{CreateUser, UpdateUser, User};
use crate::error::ApiError;

/// Repository error
#[derive(Debug)]
pub struct RepositoryError(pub String);

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Repository error: {}", self.0)
    }
}

impl std::error::Error for RepositoryError {}

/// User repository trait
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Create a new user
    async fn create(&self, user: CreateUser, hashed_password: String) -> Result<User, ApiError>;

    /// Find user by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<User>, ApiError>;

    /// Find user by username
    async fn find_by_username(
        &self,
        username: &str,
        tenant_id: i64,
    ) -> Result<Option<User>, ApiError>;

    /// Find user by email
    async fn find_by_email(&self, email: &str, tenant_id: i64) -> Result<Option<User>, ApiError>;

    /// Find all users for a tenant
    async fn find_all(&self, tenant_id: i64) -> Result<Vec<User>, ApiError>;

    /// Find all users for a tenant with pagination
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<User>, ApiError>;

    /// Update a user
    async fn update(&self, id: i64, tenant_id: i64, user: UpdateUser) -> Result<User, ApiError>;

    /// Delete a user
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Check if username exists
    async fn username_exists(&self, username: &str, tenant_id: i64) -> Result<bool, ApiError>;

    /// Check if email exists
    async fn email_exists(&self, email: &str, tenant_id: i64) -> Result<bool, ApiError>;
}

/// Internal state for InMemoryUserRepository
struct InMemoryUserInner {
    users: Vec<User>,
    next_id: i64,
}

/// In-memory user repository for testing with thread-safe single mutex
pub struct InMemoryUserRepository {
    inner: Mutex<InMemoryUserInner>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryUserInner {
                users: Vec::new(),
                next_id: 1,
            }),
        }
    }

    pub fn with_users(users: Vec<User>) -> Self {
        let max_id = users.iter().map(|u| u.id).max().unwrap_or(0);
        Self {
            inner: Mutex::new(InMemoryUserInner {
                users,
                next_id: max_id + 1,
            }),
        }
    }
}

impl Default for InMemoryUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn create(&self, create: CreateUser, hashed_password: String) -> Result<User, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let user = User {
            id,
            username: create.username,
            email: create.email,
            full_name: create.full_name,
            hashed_password,
            tenant_id: create.tenant_id,
            role: create.role.unwrap_or_default(),
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: None,
        };

        inner.users.push(user.clone());
        Ok(user)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<User>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .users
            .iter()
            .find(|u| u.id == id && u.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_username(
        &self,
        username: &str,
        tenant_id: i64,
    ) -> Result<Option<User>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .users
            .iter()
            .find(|u| u.username == username && u.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_email(&self, email: &str, tenant_id: i64) -> Result<Option<User>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .users
            .iter()
            .find(|u| u.email == email && u.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_all(&self, tenant_id: i64) -> Result<Vec<User>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .users
            .iter()
            .filter(|u| u.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<User>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .users
            .iter()
            .filter(|u| u.tenant_id == tenant_id)
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(&self, id: i64, tenant_id: i64, update: UpdateUser) -> Result<User, ApiError> {
        let mut inner = self.inner.lock();
        let user = inner
            .users
            .iter_mut()
            .find(|u| u.id == id && u.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("User {} not found", id)))?;

        if let Some(username) = update.username {
            user.username = username;
        }
        if let Some(email) = update.email {
            user.email = email;
        }
        if let Some(full_name) = update.full_name {
            user.full_name = full_name;
        }
        if let Some(is_active) = update.is_active {
            user.is_active = is_active;
        }
        if let Some(role) = update.role {
            user.role = role;
        }
        user.updated_at = Some(chrono::Utc::now());

        Ok(user.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let len_before = inner.users.len();
        inner
            .users
            .retain(|u| !(u.id == id && u.tenant_id == tenant_id));

        if inner.users.len() == len_before {
            return Err(ApiError::NotFound(format!("User {} not found", id)));
        }
        Ok(())
    }

    async fn username_exists(&self, username: &str, tenant_id: i64) -> Result<bool, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .users
            .iter()
            .any(|u| u.username == username && u.tenant_id == tenant_id))
    }

    async fn email_exists(&self, email: &str, tenant_id: i64) -> Result<bool, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .users
            .iter()
            .any(|u| u.email == email && u.tenant_id == tenant_id))
    }
}

/// Type alias for a boxed user repository
pub type BoxUserRepository = Arc<dyn UserRepository>;
