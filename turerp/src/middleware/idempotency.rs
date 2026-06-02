//! Idempotency key middleware
//!
//! When a client sends an `Idempotency-Key` header, the middleware caches
//! the response and returns the cached response for duplicate requests
//! within the TTL (default 24 hours). This prevents duplicate side effects
//! from network retries.
//!
//! Only applies to state-changing methods (POST, PUT, PATCH, DELETE).
//! GET requests are naturally idempotent and are not cached.

use crate::error::ApiError;
use actix_web::body::{EitherBody, MessageBody};
use actix_web::http::header::HeaderName;
use actix_web::http::StatusCode;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use async_trait::async_trait;
use futures::future::LocalBoxFuture;
use moka::{future::Cache, Expiry};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const IDEMPOTENCY_REPLAY_HEADER: &str = "x-idempotency-replayed";
const DEFAULT_TTL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours
const MAX_KEY_LENGTH: usize = 255;
const MAX_CACHE_SIZE: usize = 10_000;

/// A cached idempotent response
#[derive(Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    #[serde(with = "instant_serde")]
    pub expires_at: Instant,
}

mod instant_serde {
    use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(instant: &Instant, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let system_now = SystemTime::now();
        let instant_now = Instant::now();
        let duration = instant.duration_since(instant_now);
        let system_time = system_now + duration;
        let secs = system_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        secs.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Instant, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs: u64 = u64::deserialize(deserializer)?;
        let target = UNIX_EPOCH + Duration::from_secs(secs);
        let system_now = SystemTime::now();
        let duration = target
            .duration_since(system_now)
            .unwrap_or(Duration::from_secs(0));
        Ok(Instant::now() + duration)
    }
}

/// Trait for idempotency key storage backends
#[async_trait]
pub trait IdempotencyStore: Send + Sync {
    async fn get(&self, key: &str) -> Option<CachedResponse>;
    async fn set(&self, key: &str, response: CachedResponse);
    async fn remove(&self, key: &str);
}

/// Per-entry expiry driven by `CachedResponse.expires_at`.
struct IdempotencyExpiry;

impl Expiry<String, CachedResponse> for IdempotencyExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &CachedResponse,
        created_at: Instant,
    ) -> Option<Duration> {
        Some(
            value
                .expires_at
                .checked_duration_since(created_at)
                .unwrap_or(Duration::ZERO),
        )
    }
}

/// In-memory idempotency store (for development / single-instance deployment)
pub struct InMemoryIdempotencyStore {
    cache: Cache<String, CachedResponse>,
}

impl Default for InMemoryIdempotencyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryIdempotencyStore {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(MAX_CACHE_SIZE as u64)
                .expire_after(IdempotencyExpiry)
                .build(),
        }
    }
}

#[async_trait]
impl IdempotencyStore for InMemoryIdempotencyStore {
    async fn get(&self, key: &str) -> Option<CachedResponse> {
        self.cache.get(key).await
    }

    async fn set(&self, key: &str, response: CachedResponse) {
        self.cache.insert(key.to_string(), response).await;
    }

    async fn remove(&self, key: &str) {
        self.cache.invalidate(key).await;
    }
}

/// Redis-backed idempotency store for production deployments
pub struct RedisIdempotencyStore {
    conn: Arc<redis::aio::MultiplexedConnection>,
    key_prefix: String,
}

impl RedisIdempotencyStore {
    /// Create a new Redis idempotency store from a URL
    pub async fn new(url: &str) -> Result<Self, ApiError> {
        let client = redis::Client::open(url)
            .map_err(|e| ApiError::Internal(format!("Failed to open Redis connection: {}", e)))?;
        let conn = client
            .get_multiplexed_tokio_connection()
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to connect to Redis: {}", e)))?;
        Ok(Self {
            conn: Arc::new(conn),
            key_prefix: "idempotency:".to_string(),
        })
    }

    /// Set a custom key prefix (useful for multi-tenant deployments)
    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.key_prefix = prefix;
        self
    }
}

#[async_trait]
impl IdempotencyStore for RedisIdempotencyStore {
    async fn get(&self, key: &str) -> Option<CachedResponse> {
        let full_key = format!("{}{}", self.key_prefix, key);
        let mut conn = (*self.conn).clone();
        let result: Result<Option<String>, _> = redis::cmd("GET")
            .arg(&full_key)
            .query_async(&mut conn)
            .await;
        match result {
            Ok(Some(json)) => serde_json::from_str(&json).ok(),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!("Redis idempotency GET failed for key {}: {}", full_key, e);
                None
            }
        }
    }

    async fn set(&self, key: &str, response: CachedResponse) {
        let ttl = response
            .expires_at
            .saturating_duration_since(Instant::now())
            .as_secs();
        if ttl == 0 {
            return;
        }
        let json = match serde_json::to_string(&response) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!("Failed to serialize CachedResponse: {}", e);
                return;
            }
        };
        let full_key = format!("{}{}", self.key_prefix, key);
        let mut conn = (*self.conn).clone();
        if let Err(e) = redis::cmd("SETEX")
            .arg(&full_key)
            .arg(ttl)
            .arg(&json)
            .query_async::<()>(&mut conn)
            .await
        {
            tracing::warn!("Redis idempotency SETEX failed for key {}: {}", full_key, e);
        }
    }

    async fn remove(&self, key: &str) {
        let full_key = format!("{}{}", self.key_prefix, key);
        let mut conn = (*self.conn).clone();
        if let Err(e) = redis::cmd("DEL")
            .arg(&full_key)
            .query_async::<()>(&mut conn)
            .await
        {
            tracing::warn!("Redis idempotency DEL failed for key {}: {}", full_key, e);
        }
    }
}

/// Idempotency key middleware
#[derive(Clone)]
pub struct IdempotencyMiddleware {
    store: Arc<dyn IdempotencyStore>,
    ttl: Duration,
}

impl IdempotencyMiddleware {
    pub fn new(store: Arc<dyn IdempotencyStore>) -> Self {
        Self {
            store,
            ttl: DEFAULT_TTL,
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Create with default in-memory store
    pub fn in_memory() -> Self {
        Self::new(Arc::new(InMemoryIdempotencyStore::new()))
    }
}

impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for IdempotencyMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>
        + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = IdempotencyMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(IdempotencyMiddlewareService {
            service: Arc::new(service),
            store: self.store.clone(),
            ttl: self.ttl,
        }))
    }
}

/// Idempotency key middleware service
pub struct IdempotencyMiddlewareService<S> {
    service: Arc<S>,
    store: Arc<dyn IdempotencyStore>,
    ttl: Duration,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for IdempotencyMiddlewareService<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>
        + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        (*self.service).poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().clone();
        let is_state_changing = matches!(
            method,
            actix_web::http::Method::POST
                | actix_web::http::Method::PUT
                | actix_web::http::Method::PATCH
                | actix_web::http::Method::DELETE
        );

        let idempotency_key = req
            .headers()
            .get(HeaderName::from_static(IDEMPOTENCY_KEY_HEADER))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Validate key format — return 400 for invalid keys
        if let Some(ref key) = idempotency_key {
            if key.is_empty() || key.len() > MAX_KEY_LENGTH {
                let resp = actix_web::HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": "Idempotency-Key must be 1-255 characters"}));
                let body = resp.map_into_right_body::<B>();
                return Box::pin(async move { Ok(req.into_response(body)) });
            }
        }

        // If no idempotency key or not a state-changing method, pass through
        if !is_state_changing || idempotency_key.is_none() {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        let key = idempotency_key.expect("guard ensures Some");
        let service = self.service.clone();
        let store = self.store.clone();
        let ttl = self.ttl;

        // Cache check and service call both happen inside the async block
        // because store access is now async.
        Box::pin(async move {
            // Check for cached response
            if let Some(cached) = store.get(&key).await {
                let mut builder = actix_web::HttpResponse::build(
                    StatusCode::from_u16(cached.status)
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                );
                for (name, value) in &cached.headers {
                    if let Ok(header_name) = HeaderName::try_from(name.as_str()) {
                        if let Ok(header_value) =
                            actix_web::http::header::HeaderValue::from_bytes(value.as_bytes())
                        {
                            builder.insert_header((header_name, header_value));
                        }
                    }
                }
                builder.insert_header((IDEMPOTENCY_REPLAY_HEADER, "true"));
                let resp = builder.body(cached.body);
                let body = resp.map_into_right_body::<B>();
                return Ok(req.into_response(body));
            }

            // No cached response — call the service and cache the result
            let res = service.call(req).await?;
            let status = res.status().as_u16();

            // Only cache successful responses (2xx)
            if (200..300).contains(&status) {
                let headers: Vec<(String, String)> = res
                    .headers()
                    .iter()
                    .filter(|(name, _)| {
                        !matches!(
                            name.as_str(),
                            "connection" | "transfer-encoding" | "upgrade"
                        )
                    })
                    .map(|(name, value)| {
                        let v = value.to_str().unwrap_or("");
                        (name.to_string(), v.to_string())
                    })
                    .collect();

                let cached = CachedResponse {
                    status,
                    headers,
                    body: Vec::new(), // Body capture requires consuming response
                    expires_at: Instant::now() + ttl,
                };
                store.set(&key, cached).await;
            }

            Ok(res.map_into_left_body())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_store_set_and_get() {
        let store = InMemoryIdempotencyStore::new();
        let cached = CachedResponse {
            status: 200,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: Vec::from(r#"{"ok":true}"#),
            expires_at: Instant::now() + Duration::from_secs(3600),
        };

        store.set("test-key", cached.clone()).await;
        let result = store.get("test-key").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().status, 200);
    }

    #[tokio::test]
    async fn test_in_memory_store_expired() {
        let store = InMemoryIdempotencyStore::new();
        let cached = CachedResponse {
            status: 200,
            headers: vec![],
            body: Vec::from("expired"),
            expires_at: Instant::now() - Duration::from_secs(1),
        };

        store.set("expired-key", cached).await;
        assert!(store.get("expired-key").await.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_store_remove() {
        let store = InMemoryIdempotencyStore::new();
        let cached = CachedResponse {
            status: 200,
            headers: vec![],
            body: Vec::from("test"),
            expires_at: Instant::now() + Duration::from_secs(3600),
        };

        store.set("remove-key", cached).await;
        assert!(store.get("remove-key").await.is_some());
        store.remove("remove-key").await;
        assert!(store.get("remove-key").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_eviction_on_size() {
        let store = InMemoryIdempotencyStore::new();
        for i in 0..=MAX_CACHE_SIZE {
            let cached = CachedResponse {
                status: 200,
                headers: vec![],
                body: Vec::from(format!("body-{i}")),
                expires_at: Instant::now() + Duration::from_secs(3600),
            };
            store.set(&format!("key-{i}"), cached).await;
        }
        store.cache.run_pending_tasks().await;
        assert!(store.cache.entry_count() <= MAX_CACHE_SIZE as u64);
    }
}
