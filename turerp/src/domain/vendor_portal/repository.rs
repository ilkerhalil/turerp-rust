//! Vendor Portal repository

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::domain::vendor_portal::model::{
    CreateDeliveryNote, CreateVendorUser, DeliveryNote, DeliveryNoteStatus, VendorUser,
    VendorUserStatus,
};
use crate::error::ApiError;

/// Repository trait for VendorUser operations
#[async_trait]
pub trait VendorUserRepository: Send + Sync {
    async fn create(
        &self,
        req: CreateVendorUser,
        password_hash: String,
        tenant_id: i64,
    ) -> Result<VendorUser, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<VendorUser>, ApiError>;
    async fn find_by_email(
        &self,
        email: &str,
        tenant_id: i64,
    ) -> Result<Option<VendorUser>, ApiError>;
    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Option<VendorUser>, ApiError>;
    async fn update_password(
        &self,
        id: i64,
        tenant_id: i64,
        password_hash: String,
    ) -> Result<VendorUser, ApiError>;
    async fn update_last_login(&self, id: i64, tenant_id: i64) -> Result<VendorUser, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for DeliveryNote operations
#[async_trait]
pub trait DeliveryNoteRepository: Send + Sync {
    async fn create(
        &self,
        req: CreateDeliveryNote,
        vendor_user_id: i64,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<DeliveryNote, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<DeliveryNote>, ApiError>;
    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DeliveryNote>, ApiError>;
    async fn find_by_vendor_user(
        &self,
        vendor_user_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DeliveryNote>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<DeliveryNote>, ApiError>;
    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: DeliveryNoteStatus,
    ) -> Result<DeliveryNote, ApiError>;
}

pub type BoxVendorUserRepository = Arc<dyn VendorUserRepository>;
pub type BoxDeliveryNoteRepository = Arc<dyn DeliveryNoteRepository>;

struct VendorUserInner {
    users: HashMap<i64, VendorUser>,
    email_index: HashMap<(i64, String), i64>,
    next_id: AtomicI64,
}

pub struct InMemoryVendorUserRepository {
    inner: Mutex<VendorUserInner>,
}

impl Default for InMemoryVendorUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryVendorUserRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(VendorUserInner {
                users: HashMap::new(),
                email_index: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

#[async_trait]
impl VendorUserRepository for InMemoryVendorUserRepository {
    async fn create(
        &self,
        req: CreateVendorUser,
        password_hash: String,
        tenant_id: i64,
    ) -> Result<VendorUser, ApiError> {
        let mut inner = self.inner.lock();
        if inner
            .email_index
            .contains_key(&(tenant_id, req.email.clone()))
        {
            return Err(ApiError::Conflict(format!(
                "Vendor user with email '{}' already exists",
                req.email
            )));
        }
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let user = VendorUser {
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
            status: VendorUserStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_login_at: None,
        };
        inner.users.insert(id, user.clone());
        inner.email_index.insert((tenant_id, req.email), id);
        Ok(user)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<VendorUser>, ApiError> {
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
    ) -> Result<Option<VendorUser>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .email_index
            .get(&(tenant_id, email.to_string()))
            .and_then(|id| inner.users.get(id))
            .filter(|u| u.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Option<VendorUser>, ApiError> {
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
    ) -> Result<VendorUser, ApiError> {
        let mut inner = self.inner.lock();
        let user = inner
            .users
            .get_mut(&id)
            .filter(|u| u.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Vendor user {} not found", id)))?;
        user.password_hash = password_hash;
        user.updated_at = chrono::Utc::now();
        Ok(user.clone())
    }

    async fn update_last_login(&self, id: i64, tenant_id: i64) -> Result<VendorUser, ApiError> {
        let mut inner = self.inner.lock();
        let user = inner
            .users
            .get_mut(&id)
            .filter(|u| u.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Vendor user {} not found", id)))?;
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
            .ok_or_else(|| ApiError::NotFound(format!("Vendor user {} not found", id)))?;
        inner.email_index.remove(&(tenant_id, user.email.clone()));
        Ok(())
    }
}

struct DeliveryNoteInner {
    notes: HashMap<i64, DeliveryNote>,
    next_id: AtomicI64,
}

pub struct InMemoryDeliveryNoteRepository {
    inner: Mutex<DeliveryNoteInner>,
}

impl Default for InMemoryDeliveryNoteRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryDeliveryNoteRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(DeliveryNoteInner {
                notes: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

#[async_trait]
impl DeliveryNoteRepository for InMemoryDeliveryNoteRepository {
    async fn create(
        &self,
        req: CreateDeliveryNote,
        vendor_user_id: i64,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<DeliveryNote, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let note = DeliveryNote {
            id,
            tenant_id,
            vendor_user_id,
            cari_id,
            note_number: format!("DN-{}", id),
            purchase_order_id: req.purchase_order_id,
            description: req.description,
            status: DeliveryNoteStatus::Draft,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            shipped_at: None,
        };
        inner.notes.insert(id, note.clone());
        Ok(note)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<DeliveryNote>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .notes
            .get(&id)
            .filter(|n| n.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DeliveryNote>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .notes
            .values()
            .filter(|n| n.cari_id == cari_id && n.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_vendor_user(
        &self,
        vendor_user_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DeliveryNote>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .notes
            .values()
            .filter(|n| n.vendor_user_id == vendor_user_id && n.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<DeliveryNote>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .notes
            .values()
            .filter(|n| n.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: DeliveryNoteStatus,
    ) -> Result<DeliveryNote, ApiError> {
        let mut inner = self.inner.lock();
        let note = inner
            .notes
            .get_mut(&id)
            .filter(|n| n.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Delivery note {} not found", id)))?;
        note.status = status;
        note.updated_at = chrono::Utc::now();
        if status == DeliveryNoteStatus::Shipped && note.shipped_at.is_none() {
            note.shipped_at = Some(chrono::Utc::now());
        }
        Ok(note.clone())
    }
}
