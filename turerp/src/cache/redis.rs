//! Redis-backed cache implementation

use crate::cache::CacheService;
use crate::error::ApiError;
use async_trait::async_trait;
use std::sync::Arc;

/// Redis-backed cache service
pub struct RedisCacheService {
    client: Arc<redis::aio::MultiplexedConnection>,
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
            client: Arc::new(conn),
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
        let mut conn = (*self.client).clone();
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
        let mut conn = (*self.client).clone();

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
        let mut conn = (*self.client).clone();
        redis::cmd("DEL")
            .arg(key)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| ApiError::Internal(format!("Redis DEL error: {}", e)))?;
        Ok(())
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<u64, ApiError> {
        let mut conn = (*self.client).clone();
        let mut cursor: u64 = 0;
        let mut keys = Vec::new();

        loop {
            let (next_cursor, batch): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| ApiError::Internal(format!("Redis SCAN error: {}", e)))?;

            keys.extend(batch);
            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

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

    async fn health_check(&self) -> Result<(), ApiError> {
        let mut conn = (*self.client).clone();
        redis::cmd("PING")
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| ApiError::Internal(format!("Redis health check failed: {}", e)))?;
        Ok(())
    }
}
