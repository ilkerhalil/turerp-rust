//! Secrets management service — Vault integration with env fallback

use crate::error::ApiError;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for reading secrets from an external store
#[async_trait]
pub trait SecretsService: Send + Sync {
    /// Read a secret by path and key
    async fn get_secret(&self, path: &str, key: &str) -> Result<Option<String>, ApiError>;

    /// Check connectivity to the secrets backend
    async fn health_check(&self) -> Result<(), ApiError>;
}

/// Type alias for boxed secrets service
pub type BoxSecretsService = Arc<dyn SecretsService>;

// ── HashiCorp Vault implementation ─────────────────────────────────

/// Vault-backed secrets service (KV v2)
pub struct VaultSecretsService {
    client: vaultrs::client::VaultClient,
    mount: String,
}

impl VaultSecretsService {
    /// Create a new Vault secrets service
    pub async fn new(addr: &str, token: &str, mount: &str) -> Result<Self, ApiError> {
        let settings = vaultrs::client::VaultClientSettingsBuilder::default()
            .address(addr)
            .token(token)
            .build()
            .map_err(|e| ApiError::Internal(format!("Vault client settings error: {}", e)))?;

        let client = vaultrs::client::VaultClient::new(settings)
            .map_err(|e| ApiError::Internal(format!("Vault client error: {}", e)))?;

        Ok(Self {
            client,
            mount: mount.to_string(),
        })
    }
}

#[async_trait]
impl SecretsService for VaultSecretsService {
    async fn get_secret(&self, path: &str, key: &str) -> Result<Option<String>, ApiError> {
        let secret: std::collections::HashMap<String, String> =
            vaultrs::kv2::read(&self.client, &self.mount, path)
                .await
                .map_err(|e| ApiError::Internal(format!("Vault read error: {}", e)))?;

        Ok(secret.get(key).cloned())
    }

    async fn health_check(&self) -> Result<(), ApiError> {
        vaultrs::sys::health(&self.client)
            .await
            .map_err(|e| ApiError::Internal(format!("Vault health check failed: {}", e)))?;
        Ok(())
    }
}

// ── Environment fallback implementation ────────────────────────────

/// Fallback secrets service that reads from environment variables
pub struct EnvFallbackSecretsService {
    prefix: String,
}

impl EnvFallbackSecretsService {
    /// Create a new env fallback service with optional prefix
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    fn env_key(&self, path: &str, key: &str) -> String {
        let path = path.replace(['/', '-'], "_").to_uppercase();
        let key = key.to_uppercase();
        format!("{}_{}_{}", self.prefix, path, key)
    }
}

#[async_trait]
impl SecretsService for EnvFallbackSecretsService {
    async fn get_secret(&self, path: &str, key: &str) -> Result<Option<String>, ApiError> {
        let env_key = self.env_key(path, key);
        Ok(std::env::var(&env_key).ok())
    }

    async fn health_check(&self) -> Result<(), ApiError> {
        Ok(())
    }
}

// ── Cached secrets wrapper ───────────────────────────────────────

/// Wrapper that caches secrets in memory with TTL
pub struct CachedSecretsService {
    inner: BoxSecretsService,
    cache: parking_lot::Mutex<HashMap<String, (String, std::time::Instant)>>,
    ttl: std::time::Duration,
}

impl CachedSecretsService {
    pub fn new(inner: BoxSecretsService, ttl_secs: u64) -> Self {
        Self {
            inner,
            cache: parking_lot::Mutex::new(HashMap::new()),
            ttl: std::time::Duration::from_secs(ttl_secs),
        }
    }

    fn cache_key(path: &str, key: &str) -> String {
        format!("{}#{}", path, key)
    }
}

#[async_trait]
impl SecretsService for CachedSecretsService {
    async fn get_secret(&self, path: &str, key: &str) -> Result<Option<String>, ApiError> {
        let cache_key = Self::cache_key(path, key);

        {
            let cache = self.cache.lock();
            if let Some((value, fetched_at)) = cache.get(&cache_key) {
                if fetched_at.elapsed() < self.ttl {
                    return Ok(Some(value.clone()));
                }
            }
        }

        let value = self.inner.get_secret(path, key).await?;

        if let Some(ref v) = value {
            let mut cache = self.cache.lock();
            cache.insert(cache_key, (v.clone(), std::time::Instant::now()));
        }

        Ok(value)
    }

    async fn health_check(&self) -> Result<(), ApiError> {
        self.inner.health_check().await
    }
}

// ── Chained secrets service (try primary, then fallback) ─────────

/// Try a primary service first, then fallback on failure or missing secret
pub struct ChainedSecretsService {
    primary: BoxSecretsService,
    fallback: BoxSecretsService,
}

impl ChainedSecretsService {
    pub fn new(primary: BoxSecretsService, fallback: BoxSecretsService) -> Self {
        Self { primary, fallback }
    }
}

#[async_trait]
impl SecretsService for ChainedSecretsService {
    async fn get_secret(&self, path: &str, key: &str) -> Result<Option<String>, ApiError> {
        match self.primary.get_secret(path, key).await {
            Ok(Some(v)) => Ok(Some(v)),
            Ok(None) | Err(_) => self.fallback.get_secret(path, key).await,
        }
    }

    async fn health_check(&self) -> Result<(), ApiError> {
        match self.primary.health_check().await {
            Ok(()) => Ok(()),
            Err(_) => self.fallback.health_check().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSecretsService {
        data: std::collections::HashMap<String, String>,
        healthy: bool,
    }

    impl MockSecretsService {
        fn new(data: std::collections::HashMap<String, String>, healthy: bool) -> Self {
            Self { data, healthy }
        }
    }

    #[async_trait]
    impl SecretsService for MockSecretsService {
        async fn get_secret(&self, path: &str, key: &str) -> Result<Option<String>, ApiError> {
            let full = format!("{}#{}", path, key);
            Ok(self.data.get(&full).cloned())
        }

        async fn health_check(&self) -> Result<(), ApiError> {
            if self.healthy {
                Ok(())
            } else {
                Err(ApiError::Internal("unhealthy".to_string()))
            }
        }
    }

    #[tokio::test]
    async fn test_env_fallback_reads_env_var() {
        let svc = EnvFallbackSecretsService::new("TURERP");
        std::env::set_var("TURERP_TEST_PATH_TEST_KEY", "my-secret");
        let result = svc.get_secret("test-path", "test-key").await.unwrap();
        assert_eq!(result, Some("my-secret".to_string()));
    }

    #[tokio::test]
    async fn test_chained_uses_primary_when_available() {
        let mut primary_data = std::collections::HashMap::new();
        primary_data.insert("path#key".to_string(), "primary-value".to_string());
        let primary = Arc::new(MockSecretsService::new(primary_data, true)) as BoxSecretsService;

        let fallback = Arc::new(MockSecretsService::new(
            std::collections::HashMap::new(),
            true,
        )) as BoxSecretsService;

        let chained = ChainedSecretsService::new(primary, fallback);
        let result = chained.get_secret("path", "key").await.unwrap();
        assert_eq!(result, Some("primary-value".to_string()));
    }

    #[tokio::test]
    async fn test_chained_falls_back_when_primary_missing() {
        let primary = Arc::new(MockSecretsService::new(
            std::collections::HashMap::new(),
            true,
        )) as BoxSecretsService;

        let mut fallback_data = std::collections::HashMap::new();
        fallback_data.insert("path#key".to_string(), "fallback-value".to_string());
        let fallback = Arc::new(MockSecretsService::new(fallback_data, true)) as BoxSecretsService;

        let chained = ChainedSecretsService::new(primary, fallback);
        let result = chained.get_secret("path", "key").await.unwrap();
        assert_eq!(result, Some("fallback-value".to_string()));
    }

    #[tokio::test]
    async fn test_cached_service_caches_value() {
        let mut data = std::collections::HashMap::new();
        data.insert("path#key".to_string(), "cached-value".to_string());
        let inner = Arc::new(MockSecretsService::new(data, true)) as BoxSecretsService;

        let cached = CachedSecretsService::new(inner, 60);
        let result1 = cached.get_secret("path", "key").await.unwrap();
        let result2 = cached.get_secret("path", "key").await.unwrap();
        assert_eq!(result1, Some("cached-value".to_string()));
        assert_eq!(result2, Some("cached-value".to_string()));
    }
}
