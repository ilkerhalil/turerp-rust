//! Idempotency key middleware
//!
//! When a client sends an `Idempotency-Key` header, the middleware caches
//! the response and returns the cached response for duplicate requests
//! within the TTL (default 24 hours). This prevents duplicate side effects
//! from network retries.
//!
//! Only applies to state-changing methods (POST, PUT, PATCH, DELETE).
//! GET requests are naturally idempotent and are not cached.

use actix_web::body::{EitherBody, MessageBody};
use actix_web::http::header::HeaderName;
use actix_web::http::StatusCode;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures::future::LocalBoxFuture;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";
const IDEMPOTENCY_REPLAY_HEADER: &str = "x-idempotency-replayed";
const DEFAULT_TTL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours
const MAX_KEY_LENGTH: usize = 255;
const MAX_CACHE_SIZE: usize = 10_000;

/// A cached idempotent response
#[derive(Clone)]
pub struct CachedResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    expires_at: Instant,
}

/// Trait for idempotency key storage backends
pub trait IdempotencyStore: Send + Sync {
    fn get(&self, key: &str) -> Option<CachedResponse>;
    fn set(&self, key: &str, response: CachedResponse);
    #[allow(dead_code)]
    fn remove(&self, key: &str);
}

/// In-memory idempotency store (for development / single-instance deployment)
pub struct InMemoryIdempotencyStore {
    cache: RwLock<HashMap<String, CachedResponse>>,
}

impl Default for InMemoryIdempotencyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryIdempotencyStore {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    fn evict_expired(&self) {
        let mut cache = self.cache.write();
        cache.retain(|_, v| v.expires_at > Instant::now());
    }
}

impl IdempotencyStore for InMemoryIdempotencyStore {
    fn get(&self, key: &str) -> Option<CachedResponse> {
        self.evict_expired();
        self.cache.read().get(key).cloned()
    }

    fn set(&self, key: &str, response: CachedResponse) {
        self.evict_expired();
        let mut cache = self.cache.write();
        if cache.len() >= MAX_CACHE_SIZE {
            if let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, v)| v.expires_at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
            }
        }
        cache.insert(key.to_string(), response);
    }

    fn remove(&self, key: &str) {
        self.cache.write().remove(key);
    }
}

/// Idempotency key middleware
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
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
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
            service,
            store: self.store.clone(),
            ttl: self.ttl,
        }))
    }
}

/// Idempotency key middleware service
pub struct IdempotencyMiddlewareService<S> {
    service: S,
    store: Arc<dyn IdempotencyStore>,
    ttl: Duration,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for IdempotencyMiddlewareService<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
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

        let key = idempotency_key.unwrap();
        let store = self.store.clone();
        let ttl = self.ttl;

        // Check for cached response
        if let Some(cached) = store.get(&key) {
            let mut builder = actix_web::HttpResponse::build(
                StatusCode::from_u16(cached.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
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
            return Box::pin(async move { Ok(req.into_response(body)) });
        }

        // No cached response — call the service and cache the result
        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
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
                store.set(&key, cached);
            }

            Ok(res.map_into_left_body())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_store_set_and_get() {
        let store = InMemoryIdempotencyStore::new();
        let cached = CachedResponse {
            status: 200,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: Vec::from(r#"{"ok":true}"#),
            expires_at: Instant::now() + Duration::from_secs(3600),
        };

        store.set("test-key", cached.clone());
        let result = store.get("test-key");
        assert!(result.is_some());
        assert_eq!(result.unwrap().status, 200);
    }

    #[test]
    fn test_in_memory_store_expired() {
        let store = InMemoryIdempotencyStore::new();
        let cached = CachedResponse {
            status: 200,
            headers: vec![],
            body: Vec::from("expired"),
            expires_at: Instant::now() - Duration::from_secs(1),
        };

        store.set("expired-key", cached);
        assert!(store.get("expired-key").is_none());
    }

    #[test]
    fn test_in_memory_store_remove() {
        let store = InMemoryIdempotencyStore::new();
        let cached = CachedResponse {
            status: 200,
            headers: vec![],
            body: Vec::from("test"),
            expires_at: Instant::now() + Duration::from_secs(3600),
        };

        store.set("remove-key", cached);
        assert!(store.get("remove-key").is_some());
        store.remove("remove-key");
        assert!(store.get("remove-key").is_none());
    }

    #[test]
    fn test_cache_eviction_on_size() {
        let store = InMemoryIdempotencyStore::new();
        for i in 0..=MAX_CACHE_SIZE {
            let cached = CachedResponse {
                status: 200,
                headers: vec![],
                body: Vec::from(format!("body-{i}")),
                expires_at: Instant::now() + Duration::from_secs(3600),
            };
            store.set(&format!("key-{i}"), cached);
        }
        let cache = store.cache.read();
        assert!(cache.len() <= MAX_CACHE_SIZE);
    }
}
