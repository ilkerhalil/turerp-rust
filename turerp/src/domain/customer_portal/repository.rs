//! Customer Portal repository

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::domain::customer_portal::model::{
    CreatePortalUser, CreateSupportTicket, PortalUser, PortalUserStatus, SupportTicket,
    SupportTicketStatus,
};
use crate::error::ApiError;

/// Repository trait for PortalUser operations
#[async_trait]
pub trait PortalUserRepository: Send + Sync {
    async fn create(
        &self,
        req: CreatePortalUser,
        password_hash: String,
        tenant_id: i64,
    ) -> Result<PortalUser, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<PortalUser>, ApiError>;
    async fn find_by_email(
        &self,
        email: &str,
        tenant_id: i64,
    ) -> Result<Option<PortalUser>, ApiError>;
    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Option<PortalUser>, ApiError>;
    async fn update_password(
        &self,
        id: i64,
        tenant_id: i64,
        password_hash: String,
    ) -> Result<PortalUser, ApiError>;
    async fn update_last_login(&self, id: i64, tenant_id: i64) -> Result<PortalUser, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for SupportTicket operations
#[async_trait]
pub trait SupportTicketRepository: Send + Sync {
    async fn create(
        &self,
        req: CreateSupportTicket,
        portal_user_id: i64,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<SupportTicket, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<SupportTicket>, ApiError>;
    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SupportTicket>, ApiError>;
    async fn find_by_portal_user(
        &self,
        portal_user_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SupportTicket>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<SupportTicket>, ApiError>;
    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: SupportTicketStatus,
    ) -> Result<SupportTicket, ApiError>;
}

pub type BoxPortalUserRepository = Arc<dyn PortalUserRepository>;
pub type BoxSupportTicketRepository = Arc<dyn SupportTicketRepository>;

struct PortalUserInner {
    users: HashMap<i64, PortalUser>,
    email_index: HashMap<String, i64>,
    next_id: AtomicI64,
}

pub struct InMemoryPortalUserRepository {
    inner: Mutex<PortalUserInner>,
}

impl Default for InMemoryPortalUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryPortalUserRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PortalUserInner {
                users: HashMap::new(),
                email_index: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

#[async_trait]
impl PortalUserRepository for InMemoryPortalUserRepository {
    async fn create(
        &self,
        req: CreatePortalUser,
        password_hash: String,
        tenant_id: i64,
    ) -> Result<PortalUser, ApiError> {
        let mut inner = self.inner.lock();
        if inner.email_index.contains_key(&req.email) {
            return Err(ApiError::Conflict(format!(
                "Portal user with email '{}' already exists",
                req.email
            )));
        }
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let user = PortalUser {
            id,
            tenant_id,
            cari_id: req.cari_id,
            email: req.email.clone(),
            password_hash,
            full_name: req.full_name,
            phone: req.phone,
            language: req.language.unwrap_or_else(|| "en".to_string()),
            timezone: req
                .timezone
                .unwrap_or_else(|| "Europe/Istanbul".to_string()),
            status: PortalUserStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_login_at: None,
        };
        inner.users.insert(id, user.clone());
        inner.email_index.insert(req.email, id);
        Ok(user)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<PortalUser>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .users
            .get(&id)
            .filter(|u| u.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_email(
        &self,
        email: &str,
        tenant_id: i64,
    ) -> Result<Option<PortalUser>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .email_index
            .get(email)
            .and_then(|id| inner.users.get(id))
            .filter(|u| u.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Option<PortalUser>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .users
            .values()
            .find(|u| u.cari_id == cari_id && u.tenant_id == tenant_id)
            .cloned())
    }

    async fn update_password(
        &self,
        id: i64,
        tenant_id: i64,
        password_hash: String,
    ) -> Result<PortalUser, ApiError> {
        let mut inner = self.inner.lock();
        let user = inner
            .users
            .get_mut(&id)
            .filter(|u| u.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Portal user {} not found", id)))?;
        user.password_hash = password_hash;
        user.updated_at = chrono::Utc::now();
        Ok(user.clone())
    }

    async fn update_last_login(&self, id: i64, tenant_id: i64) -> Result<PortalUser, ApiError> {
        let mut inner = self.inner.lock();
        let user = inner
            .users
            .get_mut(&id)
            .filter(|u| u.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Portal user {} not found", id)))?;
        user.last_login_at = Some(chrono::Utc::now());
        user.updated_at = chrono::Utc::now();
        Ok(user.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let user = inner
            .users
            .remove(&id)
            .filter(|u| u.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Portal user {} not found", id)))?;
        inner.email_index.remove(&user.email);
        Ok(())
    }
}

struct TicketInner {
    tickets: HashMap<i64, SupportTicket>,
    next_id: AtomicI64,
}

pub struct InMemorySupportTicketRepository {
    inner: Mutex<TicketInner>,
}

impl Default for InMemorySupportTicketRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemorySupportTicketRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(TicketInner {
                tickets: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

#[async_trait]
impl SupportTicketRepository for InMemorySupportTicketRepository {
    async fn create(
        &self,
        req: CreateSupportTicket,
        portal_user_id: i64,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<SupportTicket, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let ticket = SupportTicket {
            id,
            tenant_id,
            portal_user_id,
            cari_id,
            ticket_number: format!("TKT-{}", id),
            subject: req.subject,
            description: req.description,
            status: SupportTicketStatus::Open,
            priority: req.priority,
            category: req.category,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            resolved_at: None,
        };
        inner.tickets.insert(id, ticket.clone());
        Ok(ticket)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<SupportTicket>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tickets
            .get(&id)
            .filter(|t| t.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SupportTicket>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tickets
            .values()
            .filter(|t| t.cari_id == cari_id && t.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_portal_user(
        &self,
        portal_user_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SupportTicket>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tickets
            .values()
            .filter(|t| t.portal_user_id == portal_user_id && t.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<SupportTicket>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .tickets
            .values()
            .filter(|t| t.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: SupportTicketStatus,
    ) -> Result<SupportTicket, ApiError> {
        let mut inner = self.inner.lock();
        let ticket = inner
            .tickets
            .get_mut(&id)
            .filter(|t| t.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Support ticket {} not found", id)))?;
        ticket.status = status;
        ticket.updated_at = chrono::Utc::now();
        Ok(ticket.clone())
    }
}
