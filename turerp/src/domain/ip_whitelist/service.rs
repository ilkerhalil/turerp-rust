//! IP whitelist service

use std::net::IpAddr;

use crate::domain::ip_whitelist::model::{
    ip_matches_entry, CreateIpWhitelistEntry, IpWhitelistCheckResult, IpWhitelistEntry,
    UpdateIpWhitelistEntry,
};
use crate::domain::ip_whitelist::repository::BoxIpWhitelistRepository;
use crate::error::ApiError;

/// Service for managing tenant-scoped IP whitelists
#[derive(Clone)]
pub struct IpWhitelistService {
    repo: BoxIpWhitelistRepository,
}

impl IpWhitelistService {
    pub fn new(repo: BoxIpWhitelistRepository) -> Self {
        Self { repo }
    }

    /// Check if an IP is allowed for a tenant
    ///
    /// If the tenant has no whitelist entries, all IPs are allowed (opt-in).
    #[tracing::instrument(skip(self))]
    pub async fn is_ip_allowed(&self, tenant_id: i64, ip: &str) -> bool {
        let entries = match self.repo.find_by_tenant(tenant_id).await {
            Ok(e) => e,
            Err(_) => return true,
        };

        // No whitelist entries = allow all (opt-in feature)
        if entries.is_empty() {
            return true;
        }

        let client_ip = match ip.parse::<IpAddr>() {
            Ok(ip) => ip,
            Err(_) => return false,
        };

        entries
            .iter()
            .any(|entry| ip_matches_entry(&client_ip, &entry.ip_address))
    }

    /// Check if an IP is allowed, returning detailed result
    #[tracing::instrument(skip(self))]
    pub async fn check_ip(
        &self,
        tenant_id: i64,
        ip: &str,
    ) -> Result<IpWhitelistCheckResult, ApiError> {
        let entries = self.repo.find_by_tenant(tenant_id).await?;

        if entries.is_empty() {
            return Ok(IpWhitelistCheckResult {
                allowed: true,
                matched_entry: None,
            });
        }

        let client_ip = ip
            .parse::<IpAddr>()
            .map_err(|e| ApiError::BadRequest(format!("Invalid IP address: {}", e)))?;

        let matched = entries
            .iter()
            .find(|entry| ip_matches_entry(&client_ip, &entry.ip_address))
            .cloned();

        Ok(IpWhitelistCheckResult {
            allowed: matched.is_some(),
            matched_entry: matched,
        })
    }

    /// Add a new whitelist entry
    #[tracing::instrument(skip(self))]
    pub async fn add_entry(
        &self,
        tenant_id: i64,
        mut entry: CreateIpWhitelistEntry,
    ) -> Result<IpWhitelistEntry, ApiError> {
        entry.tenant_id = tenant_id;
        entry
            .validate()
            .map_err(|errors| ApiError::Validation(errors.join("; ")))?;
        self.repo.create(entry).await
    }

    /// Remove a whitelist entry
    #[tracing::instrument(skip(self))]
    pub async fn remove_entry(&self, tenant_id: i64, id: i64) -> Result<(), ApiError> {
        self.repo.delete(id, tenant_id).await
    }

    /// List all whitelist entries for a tenant
    #[tracing::instrument(skip(self))]
    pub async fn list_entries(&self, tenant_id: i64) -> Result<Vec<IpWhitelistEntry>, ApiError> {
        self.repo.find_by_tenant(tenant_id).await
    }

    /// Update a whitelist entry
    #[tracing::instrument(skip(self))]
    pub async fn update_entry(
        &self,
        tenant_id: i64,
        id: i64,
        update: UpdateIpWhitelistEntry,
    ) -> Result<IpWhitelistEntry, ApiError> {
        if let Some(ref ip) = update.ip_address {
            if !crate::domain::ip_whitelist::model::is_valid_ip_or_cidr(ip) {
                return Err(ApiError::Validation(format!(
                    "'{}' is not a valid IP address or CIDR range",
                    ip
                )));
            }
        }
        self.repo.update(id, tenant_id, update).await
    }

    /// Get a single entry by ID
    #[tracing::instrument(skip(self))]
    pub async fn get_entry(&self, tenant_id: i64, id: i64) -> Result<IpWhitelistEntry, ApiError> {
        self.repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("IP whitelist entry {} not found", id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ip_whitelist::repository::InMemoryIpWhitelistRepository;
    use std::sync::Arc;

    fn setup_service() -> IpWhitelistService {
        let repo = Arc::new(InMemoryIpWhitelistRepository::new()) as BoxIpWhitelistRepository;
        IpWhitelistService::new(repo)
    }

    #[tokio::test]
    async fn test_is_ip_allowed_empty_whitelist() {
        let svc = setup_service();
        // No entries = allow all
        assert!(svc.is_ip_allowed(1, "192.168.1.1").await);
        assert!(svc.is_ip_allowed(1, "10.0.0.1").await);
    }

    #[tokio::test]
    async fn test_is_ip_allowed_with_entries() {
        let svc = setup_service();
        svc.add_entry(
            1,
            CreateIpWhitelistEntry {
                tenant_id: 1,
                ip_address: "192.168.1.0/24".to_string(),
                description: None,
                is_active: true,
            },
        )
        .await
        .unwrap();

        assert!(svc.is_ip_allowed(1, "192.168.1.1").await);
        assert!(svc.is_ip_allowed(1, "192.168.1.100").await);
        assert!(!svc.is_ip_allowed(1, "10.0.0.1").await);
    }

    #[tokio::test]
    async fn test_is_ip_allowed_tenant_isolation() {
        let svc = setup_service();
        svc.add_entry(
            1,
            CreateIpWhitelistEntry {
                tenant_id: 1,
                ip_address: "192.168.1.1".to_string(),
                description: None,
                is_active: true,
            },
        )
        .await
        .unwrap();

        // Tenant 1 has whitelist, so 10.0.0.1 is blocked
        assert!(!svc.is_ip_allowed(1, "10.0.0.1").await);
        // Tenant 2 has no whitelist, so 10.0.0.1 is allowed
        assert!(svc.is_ip_allowed(2, "10.0.0.1").await);
    }

    #[tokio::test]
    async fn test_is_ip_allowed_invalid_ip() {
        let svc = setup_service();
        svc.add_entry(
            1,
            CreateIpWhitelistEntry {
                tenant_id: 1,
                ip_address: "192.168.1.1".to_string(),
                description: None,
                is_active: true,
            },
        )
        .await
        .unwrap();

        // Invalid IP string should be denied when whitelist exists
        assert!(!svc.is_ip_allowed(1, "not-an-ip").await);
    }

    #[tokio::test]
    async fn test_add_entry_validation() {
        let svc = setup_service();
        let result = svc
            .add_entry(
                1,
                CreateIpWhitelistEntry {
                    tenant_id: 1,
                    ip_address: "invalid".to_string(),
                    description: None,
                    is_active: true,
                },
            )
            .await;
        assert!(matches!(result, Err(ApiError::Validation(_))));
    }

    #[tokio::test]
    async fn test_remove_entry() {
        let svc = setup_service();
        let entry = svc
            .add_entry(
                1,
                CreateIpWhitelistEntry {
                    tenant_id: 1,
                    ip_address: "192.168.1.1".to_string(),
                    description: None,
                    is_active: true,
                },
            )
            .await
            .unwrap();

        svc.remove_entry(1, entry.id).await.unwrap();
        let entries = svc.list_entries(1).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_list_entries() {
        let svc = setup_service();
        svc.add_entry(
            1,
            CreateIpWhitelistEntry {
                tenant_id: 1,
                ip_address: "192.168.1.1".to_string(),
                description: Some("Office".to_string()),
                is_active: true,
            },
        )
        .await
        .unwrap();
        svc.add_entry(
            1,
            CreateIpWhitelistEntry {
                tenant_id: 1,
                ip_address: "10.0.0.0/8".to_string(),
                description: Some("VPN".to_string()),
                is_active: true,
            },
        )
        .await
        .unwrap();

        let entries = svc.list_entries(1).await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_check_ip_detailed() {
        let svc = setup_service();
        let entry = svc
            .add_entry(
                1,
                CreateIpWhitelistEntry {
                    tenant_id: 1,
                    ip_address: "192.168.1.0/24".to_string(),
                    description: None,
                    is_active: true,
                },
            )
            .await
            .unwrap();

        let result = svc.check_ip(1, "192.168.1.50").await.unwrap();
        assert!(result.allowed);
        assert!(result.matched_entry.is_some());
        assert_eq!(result.matched_entry.unwrap().id, entry.id);

        let result = svc.check_ip(1, "10.0.0.1").await.unwrap();
        assert!(!result.allowed);
        assert!(result.matched_entry.is_none());
    }

    #[tokio::test]
    async fn test_update_entry() {
        let svc = setup_service();
        let entry = svc
            .add_entry(
                1,
                CreateIpWhitelistEntry {
                    tenant_id: 1,
                    ip_address: "192.168.1.1".to_string(),
                    description: None,
                    is_active: true,
                },
            )
            .await
            .unwrap();

        let updated = svc
            .update_entry(
                1,
                entry.id,
                UpdateIpWhitelistEntry {
                    ip_address: Some("10.0.0.0/8".to_string()),
                    description: Some(Some("Updated".to_string())),
                    is_active: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.ip_address, "10.0.0.0/8");
        assert_eq!(updated.description, Some("Updated".to_string()));
    }
}
