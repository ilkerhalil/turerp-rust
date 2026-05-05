//! Caching service with in-memory and pluggable backends
//!
//! Provides a `CacheService` trait with in-memory and Redis-compatible
//! implementations. Used for tenant config, feature flags, user
//! permissions, and hot-path query caching.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A cached entry with TTL
#[derive(Clone)]
struct CacheEntry {
    value: String,
    expires_at: Option<Instant>,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => Instant::now() > expires_at,
            None => false,
        }
    }
}

/// Cache service trait
#[async_trait::async_trait]
pub trait CacheService: Send + Sync {
    /// Get a value from the cache
    async fn get(&self, key: &str) -> Option<String>;

    /// Set a value with optional TTL
    async fn set(&self, key: &str, value: &str, ttl: Option<Duration>);

    /// Delete a value from the cache
    async fn delete(&self, key: &str);

    /// Check if a key exists (and is not expired)
    async fn exists(&self, key: &str) -> bool;

    /// Clear all entries in a namespace
    async fn clear_namespace(&self, namespace: &str);

    /// Get a value, or compute and cache it if missing
    async fn get_or_set<F, Fut>(&self, key: &str, ttl: Option<Duration>, f: F) -> String
    where
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = String> + Send;
}

/// In-memory cache implementation
pub struct InMemoryCacheService {
    cache: RwLock<HashMap<String, CacheEntry>>,
    max_entries: usize,
}

impl InMemoryCacheService {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            max_entries: 10_000,
        }
    }

    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            max_entries,
        }
    }

    fn evict_expired(&self) {
        let mut cache = self.cache.write();
        cache.retain(|_, v| !v.is_expired());
    }
}

impl Default for InMemoryCacheService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CacheService for InMemoryCacheService {
    async fn get(&self, key: &str) -> Option<String> {
        self.evict_expired();
        self.cache.read().get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.value.clone())
            }
        })
    }

    async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) {
        self.evict_expired();
        let mut cache = self.cache.write();
        if cache.len() >= self.max_entries {
            // Evict oldest expired entry, or if none expired, evict the one closest to expiry
            if let Some(evict_key) = cache
                .iter()
                .filter(|(_, v)| v.is_expired())
                .map(|(k, _)| k.clone())
                .next()
                .or_else(|| {
                    cache
                        .iter()
                        .min_by_key(|(_, v)| {
                            v.expires_at
                                .unwrap_or(Instant::now() + Duration::from_secs(86400))
                        })
                        .map(|(k, _)| k.clone())
                })
            {
                cache.remove(&evict_key);
            }
        }
        cache.insert(
            key.to_string(),
            CacheEntry {
                value: value.to_string(),
                expires_at: ttl.map(|d| Instant::now() + d),
            },
        );
    }

    async fn delete(&self, key: &str) {
        self.cache.write().remove(key);
    }

    async fn exists(&self, key: &str) -> bool {
        self.evict_expired();
        self.cache
            .read()
            .get(key)
            .map(|e| !e.is_expired())
            .unwrap_or(false)
    }

    async fn clear_namespace(&self, namespace: &str) {
        let prefix = format!("{}:", namespace);
        let mut cache = self.cache.write();
        cache.retain(|k, _| !k.starts_with(&prefix));
    }

    async fn get_or_set<F, Fut>(&self, key: &str, ttl: Option<Duration>, f: F) -> String
    where
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = String> + Send,
    {
        if let Some(value) = self.get(key).await {
            return value;
        }
        let value = f().await;
        self.set(key, &value, ttl).await;
        value
    }
}

/// Type alias for boxed cache service
pub type BoxCacheService = Arc<dyn CacheService>;

/// Namespaced cache key helper
pub fn cache_key(namespace: &str, key: &str) -> String {
    format!("{}:{}", namespace, key)
}

/// Common cache namespaces
pub mod namespaces {
    pub const TENANT_CONFIG: &str = "tenant_config";
    pub const FEATURE_FLAGS: &str = "feature_flags";
    pub const USER_PERMISSIONS: &str = "user_perms";
    pub const CARI: &str = "cari";
    pub const PRODUCT: &str = "product";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_set_and_get() {
        let cache = InMemoryCacheService::new();
        cache.set("test:key", "value1", None).await;
        let result = cache.get("test:key").await;
        assert_eq!(result, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_cache_get_missing() {
        let cache = InMemoryCacheService::new();
        let result = cache.get("nonexistent").await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_cache_delete() {
        let cache = InMemoryCacheService::new();
        cache.set("test:key", "value1", None).await;
        cache.delete("test:key").await;
        let result = cache.get("test:key").await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_cache_ttl_expired() {
        let cache = InMemoryCacheService::new();
        cache
            .set("test:key", "value1", Some(Duration::from_millis(1)))
            .await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        let result = cache.get("test:key").await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_cache_exists() {
        let cache = InMemoryCacheService::new();
        cache.set("test:key", "value1", None).await;
        assert!(cache.exists("test:key").await);
        assert!(!cache.exists("nonexistent").await);
    }

    #[tokio::test]
    async fn test_cache_clear_namespace() {
        let cache = InMemoryCacheService::new();
        cache.set("ns1:key1", "v1", None).await;
        cache.set("ns1:key2", "v2", None).await;
        cache.set("ns2:key1", "v3", None).await;

        cache.clear_namespace("ns1").await;
        assert_eq!(cache.get("ns1:key1").await, None);
        assert_eq!(cache.get("ns1:key2").await, None);
        assert_eq!(cache.get("ns2:key1").await, Some("v3".to_string()));
    }

    #[tokio::test]
    async fn test_cache_get_or_set() {
        let cache = InMemoryCacheService::new();
        let result = cache
            .get_or_set("test:key", None, || async { "computed".to_string() })
            .await;
        assert_eq!(result, "computed");

        // Second call should return cached value
        let result = cache
            .get_or_set("test:key", None, || async { "different".to_string() })
            .await;
        assert_eq!(result, "computed"); // Should still be "computed" from cache
    }

    #[test]
    fn test_cache_key_helper() {
        assert_eq!(cache_key("tenant_config", "1"), "tenant_config:1");
        assert_eq!(
            cache_key("feature_flags", "dark_mode"),
            "feature_flags:dark_mode"
        );
    }
}
