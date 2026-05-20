//! LDAP synchronization service

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::ldap::model::{
    CreateLdapConfig, LdapConfig, LdapConfigResponse, LdapSyncResult, LdapUser, UpdateLdapConfig,
};
use crate::domain::ldap::repository::BoxLdapConfigRepository;
use crate::domain::user::service::UserService;
use crate::error::ApiError;
use crate::utils::encryption::decrypt;

/// Trait for LDAP client operations, abstracting the actual LDAP library.
/// Enables mocking in tests.
#[async_trait]
pub trait LdapClient: Send + Sync {
    /// Test connectivity and bind credentials against an LDAP server
    async fn test_connection(
        &self,
        config: &LdapConfig,
        encryption_key: &[u8],
    ) -> Result<bool, ApiError>;

    /// Search for users in the LDAP directory
    async fn search_users(
        &self,
        config: &LdapConfig,
        encryption_key: &[u8],
    ) -> Result<Vec<LdapUser>, ApiError>;
}

/// Real LDAP client implementation using ldap3
pub struct Ldap3Client;

impl Ldap3Client {
    pub fn new() -> Self {
        Self
    }

    /// Decrypt the bind password from the config
    fn decrypt_password(config: &LdapConfig, key: &[u8]) -> Result<String, ApiError> {
        decrypt(&config.bind_password_encrypted, key)
            .map_err(|e| ApiError::Internal(format!("Failed to decrypt bind password: {}", e)))
    }
}

impl Default for Ldap3Client {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LdapClient for Ldap3Client {
    async fn test_connection(
        &self,
        config: &LdapConfig,
        encryption_key: &[u8],
    ) -> Result<bool, ApiError> {
        let password = Self::decrypt_password(config, encryption_key)?;

        match ldap3::LdapConnAsync::new(&config.ldap_url).await {
            Ok((conn, mut ldap)) => {
                let _drive = tokio::spawn(async move {
                    let _ = conn.drive().await;
                });

                match ldap.simple_bind(&config.bind_dn, &password).await {
                    Ok(result) => Ok(result.success().is_ok()),
                    Err(e) => {
                        tracing::warn!("LDAP bind failed: {}", e);
                        Ok(false)
                    }
                }
            }
            Err(e) => {
                tracing::warn!("LDAP connection failed: {}", e);
                Ok(false)
            }
        }
    }

    async fn search_users(
        &self,
        config: &LdapConfig,
        encryption_key: &[u8],
    ) -> Result<Vec<LdapUser>, ApiError> {
        let password = Self::decrypt_password(config, encryption_key)?;

        let (conn, mut ldap) = ldap3::LdapConnAsync::new(&config.ldap_url)
            .await
            .map_err(|e| ApiError::ServiceUnavailable(format!("LDAP connection failed: {}", e)))?;

        let _drive = tokio::spawn(async move {
            let _ = conn.drive().await;
        });

        ldap.simple_bind(&config.bind_dn, &password)
            .await
            .map_err(|e| ApiError::Unauthorized(format!("LDAP bind failed: {}", e)))?
            .success()
            .map_err(|e| ApiError::Unauthorized(format!("LDAP bind failed: {}", e)))?;

        let (entries, _res) = ldap
            .search(
                &config.base_dn,
                ldap3::Scope::Subtree,
                &config.user_filter,
                vec![
                    "cn",
                    "uid",
                    "sAMAccountName",
                    "mail",
                    "displayName",
                    "givenName",
                    "sn",
                    "memberOf",
                ],
            )
            .await
            .map_err(|e| ApiError::Internal(format!("LDAP search failed: {}", e)))?
            .success()
            .map_err(|e| ApiError::Internal(format!("LDAP search error: {}", e)))?;

        let mut users = Vec::with_capacity(entries.len());
        for entry in entries {
            let search_entry = ldap3::SearchEntry::construct(entry);
            let attrs = &search_entry.attrs;

            let username = attrs
                .get("sAMAccountName")
                .or_else(|| attrs.get("uid"))
                .or_else(|| attrs.get("cn"))
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or_default();

            if username.is_empty() {
                continue;
            }

            let email = attrs
                .get("mail")
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or_default();

            let full_name = attrs
                .get("displayName")
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or_else(|| {
                    let given = attrs
                        .get("givenName")
                        .and_then(|v| v.first())
                        .cloned()
                        .unwrap_or_default();
                    let sn = attrs
                        .get("sn")
                        .and_then(|v| v.first())
                        .cloned()
                        .unwrap_or_default();
                    if !given.is_empty() && !sn.is_empty() {
                        format!("{} {}", given, sn)
                    } else {
                        given
                    }
                });

            let groups = attrs.get("memberOf").cloned().unwrap_or_default();

            users.push(LdapUser {
                dn: search_entry.dn,
                username,
                email,
                full_name,
                groups,
            });
        }

        Ok(users)
    }
}

/// LDAP synchronization service
#[derive(Clone)]
pub struct LdapSyncService {
    repo: BoxLdapConfigRepository,
    user_service: Arc<UserService>,
    ldap_client: Arc<dyn LdapClient>,
    encryption_key: [u8; 32],
}

impl LdapSyncService {
    pub fn new(
        repo: BoxLdapConfigRepository,
        user_service: Arc<UserService>,
        encryption_key: [u8; 32],
    ) -> Self {
        Self {
            repo,
            user_service,
            ldap_client: Arc::new(Ldap3Client::new()),
            encryption_key,
        }
    }

    /// Create service with a custom LDAP client (for testing)
    pub fn with_client(mut self, client: Arc<dyn LdapClient>) -> Self {
        self.ldap_client = client;
        self
    }

    /// Get the LDAP configuration for a tenant
    pub async fn get_ldap_config(
        &self,
        tenant_id: i64,
    ) -> Result<Option<LdapConfigResponse>, ApiError> {
        let config = self.repo.find_by_tenant(tenant_id).await?;
        Ok(config.map(|c| c.into()))
    }

    /// Create or replace the LDAP configuration for a tenant
    pub async fn create_ldap_config(
        &self,
        tenant_id: i64,
        create: CreateLdapConfig,
    ) -> Result<LdapConfigResponse, ApiError> {
        let config = self
            .repo
            .create(tenant_id, create, &self.encryption_key)
            .await?;
        Ok(config.into())
    }

    /// Update the LDAP configuration for a tenant
    pub async fn update_ldap_config(
        &self,
        tenant_id: i64,
        update: UpdateLdapConfig,
    ) -> Result<LdapConfigResponse, ApiError> {
        let config = self
            .repo
            .update(tenant_id, update, &self.encryption_key)
            .await?;
        Ok(config.into())
    }

    /// Delete the LDAP configuration for a tenant
    pub async fn delete_ldap_config(&self, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.delete(tenant_id).await
    }

    /// Test LDAP connection using the stored configuration
    pub async fn test_connection(&self, tenant_id: i64) -> Result<bool, ApiError> {
        let config = self
            .repo
            .find_by_tenant(tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("LDAP configuration not found".to_string()))?;

        self.ldap_client
            .test_connection(&config, &self.encryption_key)
            .await
    }

    /// Test LDAP connection with explicit parameters (before saving config)
    pub async fn test_connection_with_params(
        &self,
        request: crate::domain::ldap::model::TestLdapConnectionRequest,
    ) -> Result<bool, ApiError> {
        let temp_config = LdapConfig {
            id: 0,
            tenant_id: 0,
            ldap_url: request.ldap_url,
            bind_dn: request.bind_dn,
            bind_password_encrypted: crate::utils::encryption::encrypt(
                &request.bind_password,
                &self.encryption_key,
            )
            .map_err(|e| ApiError::Internal(format!("Encryption failed: {}", e)))?,
            base_dn: request.base_dn,
            user_filter: "(objectClass=person)".to_string(),
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: None,
        };

        self.ldap_client
            .test_connection(&temp_config, &self.encryption_key)
            .await
    }

    /// Synchronize users from LDAP into the application
    pub async fn sync_users(&self, tenant_id: i64) -> Result<LdapSyncResult, ApiError> {
        let config = self
            .repo
            .find_by_tenant(tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("LDAP configuration not found".to_string()))?;

        if !config.is_active {
            return Err(ApiError::BadRequest(
                "LDAP configuration is disabled".to_string(),
            ));
        }

        let ldap_users = self
            .ldap_client
            .search_users(&config, &self.encryption_key)
            .await?;

        let mut result = LdapSyncResult::new();

        for ldap_user in ldap_users {
            match self
                .user_service
                .get_user_by_username(&ldap_user.username, tenant_id)
                .await
            {
                Ok(existing) => {
                    // User exists — update if needed (email or full_name changed)
                    let needs_update = existing.email != ldap_user.email
                        || existing.full_name != ldap_user.full_name;

                    if needs_update {
                        let update = crate::domain::user::model::UpdateUser {
                            email: Some(ldap_user.email.clone()),
                            full_name: Some(ldap_user.full_name.clone()),
                            ..Default::default()
                        };

                        match self
                            .user_service
                            .update_user(existing.id, tenant_id, update)
                            .await
                        {
                            Ok(_) => result.updated += 1,
                            Err(_) => result.errors += 1,
                        }
                    } else {
                        result.skipped += 1;
                    }
                }
                Err(ApiError::NotFound(_)) => {
                    // Create new user with a random secure password
                    let create = crate::domain::user::model::CreateUser {
                        username: ldap_user.username.clone(),
                        email: ldap_user.email.clone(),
                        full_name: ldap_user.full_name.clone(),
                        password: generate_random_password(),
                        tenant_id,
                        role: Some(crate::domain::user::model::Role::User),
                    };

                    match self.user_service.create_user(create).await {
                        Ok(_) => result.imported += 1,
                        Err(_) => result.errors += 1,
                    }
                }
                Err(_) => {
                    result.errors += 1;
                }
            }
        }

        Ok(result)
    }
}

/// Generate a random secure password for LDAP-imported users.
/// Users will authenticate via LDAP, so the local password is a placeholder.
fn generate_random_password() -> String {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    BASE64.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ldap::model::CreateLdapConfig;
    use crate::domain::ldap::repository::InMemoryLdapConfigRepository;
    use crate::domain::user::repository::InMemoryUserRepository;
    use crate::domain::user::service::UserService;

    fn test_key() -> [u8; 32] {
        [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ]
    }

    fn create_test_service() -> (LdapSyncService, Arc<UserService>) {
        let repo = Arc::new(InMemoryLdapConfigRepository::new()) as BoxLdapConfigRepository;
        let user_repo = Arc::new(InMemoryUserRepository::new())
            as crate::domain::user::repository::BoxUserRepository;
        let user_service = Arc::new(UserService::new(user_repo));
        let service = LdapSyncService::new(repo, user_service.clone(), test_key());
        (service, user_service)
    }

    #[tokio::test]
    async fn test_get_ldap_config_not_found() {
        let (service, _) = create_test_service();
        let result = service.get_ldap_config(1).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_create_and_get_ldap_config() {
        let (service, _) = create_test_service();

        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        let created = service.create_ldap_config(1, create).await.unwrap();
        assert_eq!(created.tenant_id, 1);
        assert_eq!(created.ldap_url, "ldap://localhost:389");

        let found = service.get_ldap_config(1).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_update_ldap_config() {
        let (service, _) = create_test_service();

        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        service.create_ldap_config(1, create).await.unwrap();

        let update = UpdateLdapConfig {
            ldap_url: Some("ldap://newhost:636".to_string()),
            ..Default::default()
        };

        let updated = service.update_ldap_config(1, update).await.unwrap();
        assert_eq!(updated.ldap_url, "ldap://newhost:636");
    }

    #[tokio::test]
    async fn test_delete_ldap_config() {
        let (service, _) = create_test_service();

        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        service.create_ldap_config(1, create).await.unwrap();
        service.delete_ldap_config(1).await.unwrap();

        let found = service.get_ldap_config(1).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_sync_users_no_config() {
        let (service, _) = create_test_service();
        let result = service.sync_users(1).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_sync_users_inactive_config() {
        let (service, _) = create_test_service();

        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        service.create_ldap_config(1, create).await.unwrap();

        let update = UpdateLdapConfig {
            is_active: Some(false),
            ..Default::default()
        };
        service.update_ldap_config(1, update).await.unwrap();

        let result = service.sync_users(1).await;
        assert!(matches!(result, Err(ApiError::BadRequest(_))));
    }

    /// Mock LDAP client for testing sync logic without a real server
    struct MockLdapClient {
        users: Vec<LdapUser>,
    }

    #[async_trait]
    impl LdapClient for MockLdapClient {
        async fn test_connection(
            &self,
            _config: &LdapConfig,
            _encryption_key: &[u8],
        ) -> Result<bool, ApiError> {
            Ok(true)
        }

        async fn search_users(
            &self,
            _config: &LdapConfig,
            _encryption_key: &[u8],
        ) -> Result<Vec<LdapUser>, ApiError> {
            Ok(self.users.clone())
        }
    }

    #[tokio::test]
    async fn test_sync_users_with_mock() {
        let repo = Arc::new(InMemoryLdapConfigRepository::new()) as BoxLdapConfigRepository;
        let user_repo = Arc::new(InMemoryUserRepository::new())
            as crate::domain::user::repository::BoxUserRepository;
        let user_service = Arc::new(UserService::new(user_repo));
        let service = LdapSyncService::new(repo, user_service.clone(), test_key()).with_client(
            Arc::new(MockLdapClient {
                users: vec![
                    LdapUser {
                        dn: "cn=john,dc=example,dc=com".to_string(),
                        username: "john".to_string(),
                        email: "john@example.com".to_string(),
                        full_name: "John Doe".to_string(),
                        groups: vec!["users".to_string()],
                    },
                    LdapUser {
                        dn: "cn=jane,dc=example,dc=com".to_string(),
                        username: "jane".to_string(),
                        email: "jane@example.com".to_string(),
                        full_name: "Jane Doe".to_string(),
                        groups: vec!["users".to_string()],
                    },
                ],
            }),
        );

        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        service.create_ldap_config(1, create).await.unwrap();

        let result = service.sync_users(1).await.unwrap();
        assert_eq!(result.imported, 2);
        assert_eq!(result.updated, 0);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.errors, 0);

        // Verify users were created
        let john = user_service.get_user_by_username("john", 1).await.unwrap();
        assert_eq!(john.email, "john@example.com");
        let jane = user_service.get_user_by_username("jane", 1).await.unwrap();
        assert_eq!(jane.email, "jane@example.com");
    }

    #[tokio::test]
    async fn test_sync_users_update_existing() {
        let repo = Arc::new(InMemoryLdapConfigRepository::new()) as BoxLdapConfigRepository;
        let user_repo = Arc::new(InMemoryUserRepository::new())
            as crate::domain::user::repository::BoxUserRepository;
        let user_service = Arc::new(UserService::new(user_repo));

        // Pre-create a user
        let create_user = crate::domain::user::model::CreateUser {
            username: "john".to_string(),
            email: "old@example.com".to_string(),
            full_name: "Old Name".to_string(),
            password: "ValidPassword123!".to_string(),
            tenant_id: 1,
            role: None,
        };
        user_service.create_user(create_user).await.unwrap();

        let service = LdapSyncService::new(repo, user_service.clone(), test_key()).with_client(
            Arc::new(MockLdapClient {
                users: vec![LdapUser {
                    dn: "cn=john,dc=example,dc=com".to_string(),
                    username: "john".to_string(),
                    email: "new@example.com".to_string(),
                    full_name: "New Name".to_string(),
                    groups: vec!["users".to_string()],
                }],
            }),
        );

        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        service.create_ldap_config(1, create).await.unwrap();

        let result = service.sync_users(1).await.unwrap();
        assert_eq!(result.imported, 0);
        assert_eq!(result.updated, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(result.errors, 0);

        let john = user_service.get_user_by_username("john", 1).await.unwrap();
        assert_eq!(john.email, "new@example.com");
        assert_eq!(john.full_name, "New Name");
    }

    #[tokio::test]
    async fn test_sync_users_skip_unchanged() {
        let repo = Arc::new(InMemoryLdapConfigRepository::new()) as BoxLdapConfigRepository;
        let user_repo = Arc::new(InMemoryUserRepository::new())
            as crate::domain::user::repository::BoxUserRepository;
        let user_service = Arc::new(UserService::new(user_repo));

        // Pre-create a user that matches exactly
        let create_user = crate::domain::user::model::CreateUser {
            username: "john".to_string(),
            email: "john@example.com".to_string(),
            full_name: "John Doe".to_string(),
            password: "ValidPassword123!".to_string(),
            tenant_id: 1,
            role: None,
        };
        user_service.create_user(create_user).await.unwrap();

        let service = LdapSyncService::new(repo, user_service.clone(), test_key()).with_client(
            Arc::new(MockLdapClient {
                users: vec![LdapUser {
                    dn: "cn=john,dc=example,dc=com".to_string(),
                    username: "john".to_string(),
                    email: "john@example.com".to_string(),
                    full_name: "John Doe".to_string(),
                    groups: vec!["users".to_string()],
                }],
            }),
        );

        let create = CreateLdapConfig {
            ldap_url: "ldap://localhost:389".to_string(),
            bind_dn: "cn=admin,dc=example,dc=com".to_string(),
            bind_password: "secret123".to_string(),
            base_dn: "dc=example,dc=com".to_string(),
            user_filter: "(objectClass=person)".to_string(),
        };

        service.create_ldap_config(1, create).await.unwrap();

        let result = service.sync_users(1).await.unwrap();
        assert_eq!(result.imported, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.errors, 0);
    }
}
