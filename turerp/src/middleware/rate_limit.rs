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

    /// Check if a peer IP is a trusted proxy.
    /// Loopback addresses are always trusted (useful for local development).
    fn is_loopback(peer_ip: &str) -> bool {
        let Ok(parsed) = peer_ip.parse::<IpAddr>() else {
            return false;
        };
        parsed.is_loopback()
    }

    /// Check if a peer IP is in the trusted proxies list.
    fn is_in_trusted_proxies(peer_ip: &str, trusted_proxies: &[IpAddr]) -> bool {
        let Ok(parsed) = peer_ip.parse::<IpAddr>() else {
            return false;
        };

        trusted_proxies.contains(&parsed)
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
        let peer_ip = req
            .connection_info()
            .peer_addr()
            .unwrap_or("unknown")
            .to_string();

        // If we have trusted proxies configured, check if the peer is one
        if !self.trusted_proxies.is_empty() {
            if RateLimitMiddleware::is_in_trusted_proxies(&peer_ip, &self.trusted_proxies) {
                // Peer is a trusted proxy - extract real client IP from headers
                if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
                    if let Ok(forwarded_str) = forwarded.to_str() {
                        // X-Forwarded-For: client, proxy1, proxy2
                        // The leftmost value is the original client
                        if let Some(client_ip) = forwarded_str.split(',').next() {
                            let trimmed = client_ip.trim().to_string();
                            if !trimmed.is_empty() {
                                return trimmed;
                            }
                        }
                    }
                }

                if let Some(real_ip) = req.headers().get("X-Real-IP") {
                    if let Ok(ip) = real_ip.to_str() {
                        let trimmed = ip.trim().to_string();
                        if !trimmed.is_empty() {
                            return trimmed;
                        }
                    }
                }
            }
            // Peer is NOT a trusted proxy - use peer IP directly
            return peer_ip;
        }

        // No trusted proxies configured - use loopback check only
        // This path is used when RateLimitMiddleware::new() or with_quota() is called
        if RateLimitMiddleware::is_loopback(&peer_ip) {
            // Local/loopback connection - try headers for convenience in dev
            if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
                if let Ok(forwarded_str) = forwarded.to_str() {
                    if let Some(client_ip) = forwarded_str.split(',').next() {
                        let trimmed = client_ip.trim().to_string();
                        if !trimmed.is_empty() {
                            return trimmed;
                        }
                    }
                }
            }

            if let Some(real_ip) = req.headers().get("X-Real-IP") {
                if let Ok(ip) = real_ip.to_str() {
                    return ip.to_string();
                }
            }
        }

        peer_ip
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
        let config = RateLimitConfig::from_env();
        assert!(!config.has_trusted_proxies());
        assert_eq!(config.requests_per_minute, 10);
        assert_eq!(config.burst_size, 3);
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
        assert!(RateLimitMiddleware::is_loopback("127.0.0.1"));
        assert!(RateLimitMiddleware::is_loopback("::1"));
    }

    #[test]
    fn test_is_not_loopback() {
        assert!(!RateLimitMiddleware::is_loopback("192.168.1.1"));
        assert!(!RateLimitMiddleware::is_loopback("10.0.0.1"));
        assert!(!RateLimitMiddleware::is_loopback("unknown"));
    }

    #[test]
    fn test_is_in_trusted_proxies() {
        let proxies: Vec<IpAddr> = vec!["10.0.0.1".parse().unwrap(), "10.0.0.2".parse().unwrap()];
        assert!(RateLimitMiddleware::is_in_trusted_proxies(
            "10.0.0.1", &proxies
        ));
        assert!(RateLimitMiddleware::is_in_trusted_proxies(
            "10.0.0.2", &proxies
        ));
        assert!(!RateLimitMiddleware::is_in_trusted_proxies(
            "10.0.0.3", &proxies
        ));
        assert!(!RateLimitMiddleware::is_in_trusted_proxies(
            "invalid", &proxies
        ));
    }
}
