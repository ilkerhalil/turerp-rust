//! Rate limiting middleware using governor
//!
//! Limits the number of requests per time window to prevent abuse.
//! Supports trusted proxy configuration for secure IP extraction
//! behind load balancers and reverse proxies.

use actix_web::body::{EitherBody, MessageBody};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures::future::LocalBoxFuture;
use governor::{clock::DefaultClock, state::keyed::DashMapStateStore, Quota, RateLimiter};
use nonzero_ext::nonzero;
use std::collections::HashMap;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::config::RateLimitConfig;

/// Default rate limit: 10 requests per minute
const DEFAULT_REQUESTS_PER_MINUTE: u32 = 10;

/// Default burst size: 3 requests
const DEFAULT_BURST_SIZE: u32 = 3;

/// Maximum number of tracked client keys in the stats store. When this cap is
/// exceeded, entries with the oldest `last_request_at` are evicted to bound
/// memory usage. See issue #328.
const MAX_STATS_ENTRIES: usize = 10_000;

/// Keyed rate limiter type
pub type KeyedRateLimiter = RateLimiter<String, DashMapStateStore<String>, DefaultClock>;

/// Statistics for a single client IP
#[derive(Default, Debug, Clone)]
pub struct RateLimitStats {
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub last_request_at: Option<std::time::SystemTime>,
}

/// Shared store for rate limit statistics
pub type RateLimitStatsStore = Arc<parking_lot::RwLock<HashMap<String, RateLimitStats>>>;

/// Evict the oldest entries when the stats map exceeds `MAX_STATS_ENTRIES`.
/// Must be called while holding a **write** lock on the store.
fn evict_if_needed(map: &mut HashMap<String, RateLimitStats>) {
    if map.len() <= MAX_STATS_ENTRIES {
        return;
    }

    // Collect (key, last_request_at) pairs, sort ascending (oldest first).
    // Entries with `last_request_at == None` are treated as oldest.
    let mut keyed: Vec<(String, Option<std::time::SystemTime>)> = map
        .iter()
        .map(|(k, v)| (k.clone(), v.last_request_at))
        .collect();
    keyed.sort_by_key(|a| a.1);

    let to_remove = map.len() - MAX_STATS_ENTRIES;
    for (key, _) in keyed.into_iter().take(to_remove) {
        map.remove(&key);
    }
}
/// Rate limiting middleware
#[derive(Clone)]
pub struct RateLimitMiddleware {
    limiter: Arc<KeyedRateLimiter>,
    trusted_proxies: Vec<IpAddr>,
    stats: RateLimitStatsStore,
}

impl RateLimitMiddleware {
    /// Create a new rate limiter with default settings (10 req/min, burst 3)
    pub fn new() -> Self {
        let quota = Quota::per_minute(nonzero!(DEFAULT_REQUESTS_PER_MINUTE))
            .allow_burst(nonzero!(DEFAULT_BURST_SIZE));

        let limiter = RateLimiter::keyed(quota);

        Self {
            limiter: Arc::new(limiter),
            trusted_proxies: Vec::new(),
            stats: RateLimitStatsStore::default(),
        }
    }

    /// Create a rate limiter with configuration from RateLimitConfig
    pub fn with_config(config: &RateLimitConfig) -> Self {
        let requests = NonZeroU32::new(config.requests_per_minute)
            .unwrap_or(nonzero!(DEFAULT_REQUESTS_PER_MINUTE));
        let burst = NonZeroU32::new(config.burst_size).unwrap_or(nonzero!(DEFAULT_BURST_SIZE));

        let quota = Quota::per_minute(requests).allow_burst(burst);
        let limiter = RateLimiter::keyed(quota);

        let trusted_proxies: Vec<IpAddr> = config
            .trusted_proxies
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        if config.has_trusted_proxies() {
            tracing::info!(
                "Rate limiting configured with {} trusted proxy(es)",
                trusted_proxies.len()
            );
        } else {
            tracing::info!(
                "Rate limiting configured without trusted proxies - \
                 X-Forwarded-For headers will be ignored"
            );
        }

        Self {
            limiter: Arc::new(limiter),
            trusted_proxies,
            stats: RateLimitStatsStore::default(),
        }
    }

    /// Create a rate limiter with custom settings
    pub fn with_quota(requests_per_minute: NonZeroU32, burst_size: NonZeroU32) -> Self {
        let quota = Quota::per_minute(requests_per_minute).allow_burst(burst_size);
        let limiter = RateLimiter::keyed(quota);

        Self {
            limiter: Arc::new(limiter),
            trusted_proxies: Vec::new(),
            stats: RateLimitStatsStore::default(),
        }
    }

    /// Get a clone of the internal stats store for dashboard access
    pub fn stats_store(&self) -> RateLimitStatsStore {
        self.stats.clone()
    }

    /// Replace the internal stats store with a shared one (used by main.rs)
    pub fn with_stats_store(mut self, stats: RateLimitStatsStore) -> Self {
        self.stats = stats;
        self
    }
}

impl Default for RateLimitMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(RateLimitMiddlewareService {
            service,
            limiter: self.limiter.clone(),
            trusted_proxies: self.trusted_proxies.clone(),
            stats: self.stats.clone(),
        }))
    }
}

/// The actual middleware service
pub struct RateLimitMiddlewareService<S> {
    service: S,
    limiter: Arc<KeyedRateLimiter>,
    trusted_proxies: Vec<IpAddr>,
    stats: RateLimitStatsStore,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for RateLimitMiddlewareService<S>
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
        let key = self.get_client_key(&req);
        let stats = self.stats.clone();

        match self.limiter.check_key(&key) {
            Ok(_) => {
                // Record allowed request
                {
                    let mut map = stats.write();
                    let entry = map.entry(key.clone()).or_default();
                    entry.total_requests += 1;
                    entry.last_request_at = Some(std::time::SystemTime::now());
                    evict_if_needed(&mut map);
                }
                // Request allowed
                let fut = self.service.call(req);
                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_left_body())
                })
            }
            Err(_) => {
                // Record blocked request
                {
                    let mut map = stats.write();
                    let entry = map.entry(key).or_default();
                    entry.total_requests += 1;
                    entry.blocked_requests += 1;
                    entry.last_request_at = Some(std::time::SystemTime::now());
                    evict_if_needed(&mut map);
                }
                // Rate limit exceeded
                let response = actix_web::HttpResponse::TooManyRequests()
                    .json(crate::error::ErrorResponse {
                        error: "Rate limit exceeded. Please try again later.".to_string(),
                    })
                    .map_into_right_body::<B>();
                Box::pin(async move { Ok(req.into_response(response)) })
            }
        }
    }
}

impl<S> RateLimitMiddlewareService<S> {
    /// Get client IP, considering trusted proxies configuration.
    fn get_client_key(&self, req: &ServiceRequest) -> String {
        crate::common::ip_utils::extract_client_ip(req, &self.trusted_proxies).unwrap_or_else(
            || {
                req.connection_info()
                    .peer_addr()
                    .unwrap_or("unknown")
                    .to_string()
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimitMiddleware::new();
        assert!(Arc::strong_count(&limiter.limiter) > 0);
    }

    #[test]
    fn test_custom_rate_limiter() {
        let limiter = RateLimitMiddleware::with_quota(
            NonZeroU32::new(100).unwrap(),
            NonZeroU32::new(10).unwrap(),
        );
        assert!(Arc::strong_count(&limiter.limiter) > 0);
    }

    #[test]
    fn test_rate_limit_config_from_env() {
        // This test asserts the safe defaults (120 rpm / 30 burst) returned
        // by `from_env` when env vars are unset. The previous version of
        // this test pinned the buggy 10/3 values; it was updated as part
        // of the same fix so the regression test now matches production
        // behavior. If the env happens to be set in the test runner, this
        // assertion will only catch the env-unset path, so it is best
        // paired with `tests/rate_limit_env_loading_test.rs` which fully
        // scrubs the env.
        std::env::remove_var("TURERP_RATE_LIMIT_REQUESTS_PER_MINUTE");
        std::env::remove_var("TURERP_RATE_LIMIT_BURST");
        std::env::remove_var("TURERP_TRUSTED_PROXIES");
        let config = RateLimitConfig::from_env();
        assert!(!config.has_trusted_proxies());
        assert_eq!(config.requests_per_minute, 120);
        assert_eq!(config.burst_size, 30);
    }

    #[test]
    fn test_rate_limit_config_with_proxies() {
        let config = RateLimitConfig {
            trusted_proxies: vec!["10.0.0.1".to_string(), "10.0.0.2".to_string()],
            requests_per_minute: 100,
            burst_size: 20,
        };
        assert!(config.has_trusted_proxies());
        assert_eq!(config.requests_per_minute, 100);
        assert_eq!(config.burst_size, 20);
    }

    #[test]
    fn test_rate_limit_config_with_config() {
        let config = RateLimitConfig {
            trusted_proxies: vec!["10.0.0.1".to_string()],
            requests_per_minute: 50,
            burst_size: 10,
        };
        let limiter = RateLimitMiddleware::with_config(&config);
        assert!(Arc::strong_count(&limiter.limiter) > 0);
        assert_eq!(limiter.trusted_proxies.len(), 1);
    }

    #[test]
    fn test_is_loopback() {
        assert!(crate::common::ip_utils::is_loopback("127.0.0.1"));
        assert!(crate::common::ip_utils::is_loopback("::1"));
    }

    #[test]
    fn test_is_not_loopback() {
        assert!(!crate::common::ip_utils::is_loopback("192.168.1.1"));
        assert!(!crate::common::ip_utils::is_loopback("10.0.0.1"));
        assert!(!crate::common::ip_utils::is_loopback("unknown"));
    }

    #[test]
    fn test_is_in_trusted_proxies() {
        let proxies: Vec<IpAddr> = vec!["10.0.0.1".parse().unwrap(), "10.0.0.2".parse().unwrap()];
        assert!(crate::common::ip_utils::is_in_trusted_proxies(
            "10.0.0.1", &proxies
        ));
        assert!(crate::common::ip_utils::is_in_trusted_proxies(
            "10.0.0.2", &proxies
        ));
        assert!(!crate::common::ip_utils::is_in_trusted_proxies(
            "10.0.0.3", &proxies
        ));
        assert!(!crate::common::ip_utils::is_in_trusted_proxies(
            "invalid", &proxies
        ));
    }

    #[test]
    fn test_evict_if_needed_under_cap() {
        let mut map = HashMap::new();
        map.insert("a".to_string(), RateLimitStats::default());
        map.insert("b".to_string(), RateLimitStats::default());
        evict_if_needed(&mut map);
        assert_eq!(map.len(), 2, "no eviction when under cap");
    }

    #[test]
    fn test_evict_if_needed_over_cap() {
        // Insert MAX + 5 entries with increasing timestamps; the 5 oldest
        // should be evicted, keeping exactly MAX_STATS_ENTRIES.
        let mut map = HashMap::new();
        let base = std::time::SystemTime::UNIX_EPOCH;
        for i in 0..(MAX_STATS_ENTRIES + 5) {
            map.insert(
                format!("client-{i}"),
                RateLimitStats {
                    total_requests: 1,
                    blocked_requests: 0,
                    last_request_at: Some(base + std::time::Duration::from_secs(i as u64)),
                },
            );
        }
        assert_eq!(map.len(), MAX_STATS_ENTRIES + 5);
        evict_if_needed(&mut map);
        assert_eq!(map.len(), MAX_STATS_ENTRIES);
        // The 5 oldest (client-0..client-4) should be gone, client-5 retained
        assert!(!map.contains_key("client-0"));
        assert!(!map.contains_key("client-4"));
        assert!(map.contains_key("client-5"));
        assert!(map.contains_key(&format!("client-{}", MAX_STATS_ENTRIES + 4)));
    }

    #[test]
    fn test_evict_if_needed_none_timestamps_oldest() {
        // Entries with None timestamps should be evicted first (treated as oldest).
        let mut map = HashMap::new();
        for i in 0..(MAX_STATS_ENTRIES + 2) {
            let ts = if i < 2 {
                None // these two should be evicted first
            } else {
                Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(i as u64))
            };
            map.insert(
                format!("client-{i}"),
                RateLimitStats {
                    total_requests: 1,
                    blocked_requests: 0,
                    last_request_at: ts,
                },
            );
        }
        evict_if_needed(&mut map);
        assert_eq!(map.len(), MAX_STATS_ENTRIES);
        assert!(!map.contains_key("client-0"));
        assert!(!map.contains_key("client-1"));
        assert!(map.contains_key("client-2"));
    }
}
