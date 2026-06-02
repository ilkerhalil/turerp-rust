//! In-memory cache implementation

use crate::cache::CacheService;
use crate::error::ApiError;
use async_trait::async_trait;
use moka::future::Cache;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// A cached entry with optional TTL
#[derive(Clone, Debug)]
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

/// In-memory cache implementing the crate cache trait
pub struct InMemoryCacheService {
    cache: Cache<String, CacheEntry>,
    keys: RwLock<HashSet<String>>,
}

impl InMemoryCacheService {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder().max_capacity(10_000).build(),
            keys: RwLock::new(HashSet::new()),
        }
    }

    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            cache: Cache::builder().max_capacity(max_entries as u64).build(),
            keys: RwLock::new(HashSet::new()),
        }
    }
}

impl Default for InMemoryCacheService {
    fn default() -> Self {
        Self::new()
    }
}

/// Basic glob matcher for Redis KEYS patterns (supports * and ?)
fn glob_matches(pattern: &str, text: &str) -> bool {
    let mut pat = pattern.chars().peekable();
    let mut txt = text.chars().peekable();

    loop {
        match pat.next() {
            Some('*') => {
                // Skip consecutive stars
                while pat.peek() == Some(&'*') {
                    pat.next();
                }
                let remaining_pat: String = pat.clone().collect();
                if remaining_pat.is_empty() {
                    return true;
                }
                let remaining_txt: String = txt.clone().collect();
                // Try every possible suffix match
                for i in 0..remaining_txt.len() + 1 {
                    if glob_matches(&remaining_pat, &remaining_txt[i..]) {
                        return true;
                    }
                }
                return false;
            }
            Some('?') => {
                if txt.next().is_none() {
                    return false;
                }
            }
            Some(c) => {
                if txt.next() != Some(c) {
                    return false;
                }
            }
            None => return txt.next().is_none(),
        }
    }
}

#[async_trait]
impl CacheService for InMemoryCacheService {
    async fn get_raw(&self, key: &str) -> Result<Option<String>, ApiError> {
        match self.cache.get(key).await {
            Some(entry) if !entry.is_expired() => Ok(Some(entry.value)),
            Some(_) => {
                self.cache.invalidate(key).await;
                self.keys.write().remove(key);
                Ok(None)
            }
            None => Ok(None),
        }
    }

    async fn set_raw(
        &self,
        key: &str,
        value: &str,
        ttl_seconds: Option<u64>,
    ) -> Result<(), ApiError> {
        let expires_at = ttl_seconds.map(|ttl| Instant::now() + Duration::from_secs(ttl));
        let entry = CacheEntry {
            value: value.to_string(),
            expires_at,
        };
        self.cache.insert(key.to_string(), entry).await;
        self.keys.write().insert(key.to_string());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), ApiError> {
        self.cache.invalidate(key).await;
        self.keys.write().remove(key);
        Ok(())
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<u64, ApiError> {
        let keys_to_remove: Vec<String> = {
            let keys = self.keys.read();
            keys.iter()
                .filter(|k| glob_matches(pattern, k))
                .cloned()
                .collect()
        };

        let count = keys_to_remove.len() as u64;
        for key in &keys_to_remove {
            self.cache.invalidate(key).await;
        }
        {
            let mut keys = self.keys.write();
            for key in keys_to_remove {
                keys.remove(&key);
            }
        }
        Ok(count)
    }

    fn is_enabled(&self) -> bool {
        true
    }

    async fn health_check(&self) -> Result<(), ApiError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_cache_set_and_get() {
        let cache = InMemoryCacheService::new();
        cache.set_raw("test:key", "value1", Some(60)).await.unwrap();
        let result = cache.get_raw("test:key").await.unwrap();
        assert_eq!(result, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_in_memory_cache_get_missing() {
        let cache = InMemoryCacheService::new();
        let result = cache.get_raw("nonexistent").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_in_memory_cache_delete() {
        let cache = InMemoryCacheService::new();
        cache.set_raw("test:key", "value1", Some(60)).await.unwrap();
        cache.delete("test:key").await.unwrap();
        let result = cache.get_raw("test:key").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_in_memory_cache_ttl_expired() {
        let cache = InMemoryCacheService::new();
        cache.set_raw("test:key", "value1", Some(1)).await.unwrap();
        tokio::time::sleep(Duration::from_secs(2)).await;
        let result = cache.get_raw("test:key").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_in_memory_cache_delete_pattern() {
        let cache = InMemoryCacheService::new();
        cache.set_raw("ns1:key1", "v1", None).await.unwrap();
        cache.set_raw("ns1:key2", "v2", None).await.unwrap();
        cache.set_raw("ns2:key1", "v3", None).await.unwrap();

        let count = cache.delete_pattern("ns1:*").await.unwrap();
        assert_eq!(count, 2);
        assert_eq!(cache.get_raw("ns1:key1").await.unwrap(), None);
        assert_eq!(cache.get_raw("ns1:key2").await.unwrap(), None);
        assert_eq!(
            cache.get_raw("ns2:key1").await.unwrap(),
            Some("v3".to_string())
        );
    }

    #[tokio::test]
    async fn test_in_memory_cache_is_enabled() {
        let cache = InMemoryCacheService::new();
        assert!(cache.is_enabled());
    }

    #[test]
    fn test_glob_matches() {
        assert!(glob_matches("*", "anything"));
        assert!(glob_matches("ns1:*", "ns1:key1"));
        assert!(!glob_matches("ns1:*", "ns2:key1"));
        assert!(glob_matches("ns1:key?", "ns1:key1"));
        assert!(!glob_matches("ns1:key?", "ns1:key10"));
        assert!(glob_matches("exact", "exact"));
        assert!(!glob_matches("exact", "exactly"));
    }
}
