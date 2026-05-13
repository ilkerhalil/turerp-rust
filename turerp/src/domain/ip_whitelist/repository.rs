//! IP whitelist repository

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::ip_whitelist::model::{
    CreateIpWhitelistEntry, IpWhitelistEntry, UpdateIpWhitelistEntry,
};
use crate::error::ApiError;

/// Repository trait for IP whitelist operations
#[async_trait]
pub trait IpWhitelistRepository: Send + Sync {
    /// Find all active whitelist entries for a tenant
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<IpWhitelistEntry>, ApiError>;

    /// Create a new whitelist entry
    async fn create(&self, entry: CreateIpWhitelistEntry) -> Result<IpWhitelistEntry, ApiError>;

    /// Update an existing whitelist entry
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateIpWhitelistEntry,
    ) -> Result<IpWhitelistEntry, ApiError>;

    /// Delete a whitelist entry
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Check if a whitelist entry exists for a tenant
    async fn exists(&self, id: i64, tenant_id: i64) -> Result<bool, ApiError>;

    /// Find a single entry by ID and tenant
    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<IpWhitelistEntry>, ApiError>;
}

/// Type alias for boxed IpWhitelistRepository
pub type BoxIpWhitelistRepository = Arc<dyn IpWhitelistRepository>;

struct InMemoryInner {
    entries: HashMap<i64, IpWhitelistEntry>,
    next_id: i64,
    tenant_entries: HashMap<i64, Vec<i64>>,
}

/// In-memory IP whitelist repository for testing and development
pub struct InMemoryIpWhitelistRepository {
    inner: Mutex<InMemoryInner>,
}

impl InMemoryIpWhitelistRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryInner {
                entries: HashMap::new(),
                next_id: 1,
                tenant_entries: HashMap::new(),
            }),
        }
    }
}

impl Default for InMemoryIpWhitelistRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IpWhitelistRepository for InMemoryIpWhitelistRepository {
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<IpWhitelistEntry>, ApiError> {
        let inner = self.inner.lock();
        let entries: Vec<IpWhitelistEntry> = inner
            .tenant_entries
            .get(&tenant_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| inner.entries.get(id))
                    .filter(|e| e.is_active)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        Ok(entries)
    }

    async fn create(&self, create: CreateIpWhitelistEntry) -> Result<IpWhitelistEntry, ApiError> {
        let mut inner = self.inner.lock();

        let id = inner.next_id;
        inner.next_id += 1;
        let now = chrono::Utc::now();

        let entry = IpWhitelistEntry {
            id,
            tenant_id: create.tenant_id,
            ip_address: create.ip_address,
            description: create.description,
            is_active: create.is_active,
            created_at: now,
        };

        inner.entries.insert(id, entry.clone());
        inner
            .tenant_entries
            .entry(create.tenant_id)
            .or_default()
            .push(id);

        Ok(entry)
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateIpWhitelistEntry,
    ) -> Result<IpWhitelistEntry, ApiError> {
        let mut inner = self.inner.lock();

        let entry = inner
            .entries
            .get_mut(&id)
            .filter(|e| e.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("IP whitelist entry {} not found", id)))?;

        if let Some(ip) = update.ip_address {
            entry.ip_address = ip;
        }
        if let Some(desc) = update.description {
            entry.description = desc;
        }
        if let Some(active) = update.is_active {
            entry.is_active = active;
        }

        Ok(entry.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let entry = inner
            .entries
            .get(&id)
            .filter(|e| e.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("IP whitelist entry {} not found", id)))?;

        let tid = entry.tenant_id;
        inner.entries.remove(&id);
        if let Some(ids) = inner.tenant_entries.get_mut(&tid) {
            ids.retain(|&x| x != id);
        }

        Ok(())
    }

    async fn exists(&self, id: i64, tenant_id: i64) -> Result<bool, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .entries
            .get(&id)
            .map(|e| e.tenant_id == tenant_id)
            .unwrap_or(false))
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<IpWhitelistEntry>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .entries
            .get(&id)
            .filter(|e| e.tenant_id == tenant_id)
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_find() {
        let repo = InMemoryIpWhitelistRepository::new();
        let create = CreateIpWhitelistEntry {
            tenant_id: 1,
            ip_address: "192.168.1.0/24".to_string(),
            description: Some("Office".to_string()),
            is_active: true,
        };

        let entry = repo.create(create).await.unwrap();
        assert_eq!(entry.id, 1);
        assert_eq!(entry.tenant_id, 1);

        let found = repo.find_by_id(1, 1).await.unwrap();
        assert!(found.is_some());

        let entries = repo.find_by_tenant(1).await.unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_find_by_tenant_only_active() {
        let repo = InMemoryIpWhitelistRepository::new();
        repo.create(CreateIpWhitelistEntry {
            tenant_id: 1,
            ip_address: "192.168.1.1".to_string(),
            description: None,
            is_active: true,
        })
        .await
        .unwrap();
        repo.create(CreateIpWhitelistEntry {
            tenant_id: 1,
            ip_address: "192.168.1.2".to_string(),
            description: None,
            is_active: false,
        })
        .await
        .unwrap();

        let entries = repo.find_by_tenant(1).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].ip_address, "192.168.1.1");
    }

    #[tokio::test]
    async fn test_update() {
        let repo = InMemoryIpWhitelistRepository::new();
        let entry = repo
            .create(CreateIpWhitelistEntry {
                tenant_id: 1,
                ip_address: "10.0.0.0/8".to_string(),
                description: None,
                is_active: true,
            })
            .await
            .unwrap();

        let updated = repo
            .update(
                entry.id,
                1,
                UpdateIpWhitelistEntry {
                    ip_address: Some("172.16.0.0/12".to_string()),
                    description: None,
                    is_active: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.ip_address, "172.16.0.0/12");
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryIpWhitelistRepository::new();
        let entry = repo
            .create(CreateIpWhitelistEntry {
                tenant_id: 1,
                ip_address: "192.168.1.1".to_string(),
                description: None,
                is_active: true,
            })
            .await
            .unwrap();

        repo.delete(entry.id, 1).await.unwrap();
        assert!(repo.find_by_id(entry.id, 1).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let repo = InMemoryIpWhitelistRepository::new();
        repo.create(CreateIpWhitelistEntry {
            tenant_id: 1,
            ip_address: "192.168.1.1".to_string(),
            description: None,
            is_active: true,
        })
        .await
        .unwrap();

        let entries = repo.find_by_tenant(2).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_exists() {
        let repo = InMemoryIpWhitelistRepository::new();
        let entry = repo
            .create(CreateIpWhitelistEntry {
                tenant_id: 1,
                ip_address: "192.168.1.1".to_string(),
                description: None,
                is_active: true,
            })
            .await
            .unwrap();

        assert!(repo.exists(entry.id, 1).await.unwrap());
        assert!(!repo.exists(entry.id, 2).await.unwrap());
        assert!(!repo.exists(999, 1).await.unwrap());
    }
}
