//! Rate limiting middleware using governor
//!
//! Limits the number of requests per time window to prevent abuse

use actix_web::body::BoxBody;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures::future::LocalBoxFuture;
use governor::{clock::DefaultClock, state::keyed::DashMapStateStore, Quota, RateLimiter};
use nonzero_ext::nonzero;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::task::{Context, Poll};

/// Default rate limit: 10 requests per minute
const DEFAULT_REQUESTS_PER_MINUTE: u32 = 10;

/// Default burst size: 3 requests
const DEFAULT_BURST_SIZE: u32 = 3;

/// Keyed rate limiter type
pub type KeyedRateLimiter = RateLimiter<String, DashMapStateStore<String>, DefaultClock>;

/// Rate limiting middleware
pub struct RateLimitMiddleware {
    limiter: Arc<KeyedRateLimiter>,
}

impl RateLimitMiddleware {
    /// Create a new rate limiter with default settings (10 req/min, burst 3)
    pub fn new() -> Self {
        let quota = Quota::per_minute(nonzero!(DEFAULT_REQUESTS_PER_MINUTE))
            .allow_burst(nonzero!(DEFAULT_BURST_SIZE));

        let limiter = RateLimiter::keyed(quota);

        Self {
            limiter: Arc::new(limiter),
        }
    }

    /// Create a rate limiter with custom settings
    pub fn with_quota(requests_per_minute: NonZeroU32, burst_size: NonZeroU32) -> Self {
        let quota = Quota::per_minute(requests_per_minute).allow_burst(burst_size);

        let limiter = RateLimiter::keyed(quota);

        Self {
            limiter: Arc::new(limiter),
        }
    }

    /// Get client IP from request
    fn get_client_key(req: &ServiceRequest) -> String {
        // Try X-Forwarded-For first (for reverse proxy setups)
        if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                if let Some(client_ip) = forwarded_str.split(',').next() {
                    return client_ip.trim().to_string();
                }
            }
        }

        // Try X-Real-IP
        if let Some(real_ip) = req.headers().get("X-Real-IP") {
            if let Ok(ip) = real_ip.to_str() {
                return ip.to_string();
            }
        }

        // Fallback to connection info
        req.connection_info()
            .peer_addr()
            .unwrap_or("unknown")
            .to_string()
    }
}

impl Default for RateLimitMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> actix_web::dev::Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimitMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(RateLimitMiddlewareService {
            service,
            limiter: self.limiter.clone(),
        }))
    }
}

/// The actual middleware service
pub struct RateLimitMiddlewareService<S> {
    service: S,
    limiter: Arc<KeyedRateLimiter>,
}

impl<S> actix_web::dev::Service<ServiceRequest> for RateLimitMiddlewareService<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let key = RateLimitMiddleware::get_client_key(&req);

        match self.limiter.check_key(&key) {
            Ok(_) => {
                // Request allowed
                let fut = self.service.call(req);
                Box::pin(fut)
            }
            Err(_) => {
                // Rate limit exceeded
                let response = actix_web::HttpResponse::TooManyRequests()
                    .json(crate::error::ErrorResponse {
                        error: "Rate limit exceeded. Please try again later.".to_string(),
                    })
                    .map_into_boxed_body();
                Box::pin(async move { Ok(req.into_response(response)) })
            }
        }
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
}
