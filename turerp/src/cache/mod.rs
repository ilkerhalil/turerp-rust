//! Cache module - Redis-based caching with TTL support

use crate::error::ApiError;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;

/// Cache key prefix for tenant-scoped entries
const TENANT_PREFIX: &str = "turerp";

/// Cache service trait for get/set/delete operations
///
/// Uses JSON strings as the value format to avoid trait object incompatibility
/// with generic methods. Callers serialize/deserialize via the helper functions.
#[async_trait]
pub trait CacheService: Send + Sync {
    /// Get a cached raw JSON string by key, returning None if missing or expired
    async fn get_raw(&self, key: &str) -> Result<Option<String>, ApiError>;

    /// Set a raw JSON string with optional TTL (seconds)
    async fn set_raw(
        &self,
        key: &str,
        value: &str,
        ttl_seconds: Option<u64>,
    ) -> Result<(), ApiError>;

    /// Delete a cached entry
    async fn delete(&self, key: &str) -> Result<(), ApiError>;

    /// Delete entries matching a pattern (Redis scan + del)
    async fn delete_pattern(&self, pattern: &str) -> Result<u64, ApiError>;

    /// Check if caching is enabled
    fn is_enabled(&self) -> bool;
}

/// Helper: serialize a value to JSON and cache it
pub async fn cache_set<T: Serialize + Send + Sync>(
    cache: &dyn CacheService,
    key: &str,
    value: &T,
    ttl_seconds: Option<u64>,
) -> Result<(), ApiError> {
    let json = serde_json::to_string(value)
        .map_err(|e| ApiError::Internal(format!("Cache serialization error: {}", e)))?;
    cache.set_raw(key, &json, ttl_seconds).await
}

/// Helper: get a cached value and deserialize from JSON
pub async fn cache_get<T: DeserializeOwned + Send>(
    cache: &dyn CacheService,
    key: &str,
) -> Result<Option<T>, ApiError> {
    match cache.get_raw(key).await? {
        Some(json) => {
            let value = serde_json::from_str(&json)
                .map_err(|e| ApiError::Internal(format!("Cache deserialization error: {}", e)))?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

/// Build a tenant-scoped cache key
pub fn cache_key(tenant_id: i64, resource: &str, id: &str) -> String {
    format!("{}:t{}:{}:{}", TENANT_PREFIX, tenant_id, resource, id)
}

/// Build a cache key for list results
pub fn list_cache_key(tenant_id: i64, resource: &str, page: u32, per_page: u32) -> String {
    format!(
        "{}:t{}:{}:list:{}:{}",
        TENANT_PREFIX, tenant_id, resource, page, per_page
    )
}

// ---------------------------------------------------------------------------
// No-op cache (used when Redis is disabled)
// ---------------------------------------------------------------------------

/// No-op cache implementation - always returns cache misses
pub struct NoopCacheService;

#[async_trait]
impl CacheService for NoopCacheService {
    async fn get_raw(&self, _key: &str) -> Result<Option<String>, ApiError> {
        Ok(None)
    }

    async fn set_raw(
        &self,
        _key: &str,
        _value: &str,
        _ttl_seconds: Option<u64>,
    ) -> Result<(), ApiError> {
        Ok(())
    }

    async fn delete(&self, _key: &str) -> Result<(), ApiError> {
        Ok(())
    }

    async fn delete_pattern(&self, _pattern: &str) -> Result<u64, ApiError> {
        Ok(0)
    }

    fn is_enabled(&self) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Redis cache
// ---------------------------------------------------------------------------

/// Redis-backed cache service
pub struct RedisCacheService {
    client: redis::aio::MultiplexedConnection,
    default_ttl: u64,
}

impl RedisCacheService {
    /// Create a new Redis cache service from a URL
    pub async fn new(url: &str, default_ttl: u64) -> Result<Self, ApiError> {
        let client = redis::Client::open(url)
            .map_err(|e| ApiError::Internal(format!("Failed to open Redis connection: {}", e)))?;
        let conn = client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to connect to Redis: {}", e)))?;
        Ok(Self {
            client: conn,
            default_ttl,
        })
    }

    /// Convert to boxed trait object
    pub fn into_arc(self) -> Arc<dyn CacheService> {
        Arc::new(self)
    }
}

#[async_trait]
impl CacheService for RedisCacheService {
    async fn get_raw(&self, key: &str) -> Result<Option<String>, ApiError> {
        let mut conn = self.client.clone();
        redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| ApiError::Internal(format!("Redis GET error: {}", e)))
    }

    async fn set_raw(
        &self,
        key: &str,
        value: &str,
        ttl_seconds: Option<u64>,
    ) -> Result<(), ApiError> {
        let ttl = ttl_seconds.unwrap_or(self.default_ttl);
        let mut conn = self.client.clone();

        redis::cmd("SETEX")
            .arg(key)
            .arg(ttl)
            .arg(value)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| ApiError::Internal(format!("Redis SETEX error: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), ApiError> {
        let mut conn = self.client.clone();
        redis::cmd("DEL")
            .arg(key)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| ApiError::Internal(format!("Redis DEL error: {}", e)))?;
        Ok(())
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<u64, ApiError> {
        let mut conn = self.client.clone();
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| ApiError::Internal(format!("Redis KEYS error: {}", e)))?;

        if keys.is_empty() {
            return Ok(0);
        }

        let count: u64 = redis::cmd("DEL")
            .arg(&keys)
            .query_async(&mut conn)
            .await
            .map_err(|e| ApiError::Internal(format!("Redis DEL error: {}", e)))?;

        Ok(count)
    }

    fn is_enabled(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_format() {
        let key = cache_key(42, "products", "123");
        assert_eq!(key, "turerp:t42:products:123");
    }

    #[test]
    fn test_list_cache_key_format() {
        let key = list_cache_key(42, "products", 1, 20);
        assert_eq!(key, "turerp:t42:products:list:1:20");
    }

    #[tokio::test]
    async fn test_noop_cache_always_miss() {
        let cache = NoopCacheService;
        let result = cache.get_raw("any-key").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_noop_cache_set_is_noop() {
        let cache = NoopCacheService;
        cache.set_raw("key", "value", None).await.unwrap();
        let result = cache.get_raw("key").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_get_set_helpers() {
        let cache = NoopCacheService;
        let value = vec!["a", "b", "c"];
        cache_set(&cache, "list", &value, None).await.unwrap();
        let result: Option<Vec<String>> = cache_get(&cache, "list").await.unwrap();
        assert!(result.is_none());
    }
}
