//! IP whitelist middleware for tenant-scoped access control
//!
//! Extracts the client IP from the request and checks it against the tenant's
//! whitelist. If the tenant has no whitelist entries, all IPs are allowed (opt-in).
//!
//! Trusted proxy configuration: only extracts the real client IP from
//! X-Forwarded-For / X-Real-IP headers when the direct peer is a trusted proxy
//! or loopback address. Otherwise the direct peer IP is used.

use actix_web::body::{EitherBody, MessageBody};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage};
use futures::future::LocalBoxFuture;
use std::net::IpAddr;
use std::task::{Context, Poll};

use crate::domain::ip_whitelist::service::IpWhitelistService;

/// IP whitelist middleware
#[derive(Clone)]
pub struct IpWhitelistMiddleware {
    service: IpWhitelistService,
    trusted_proxies: Vec<IpAddr>,
}

impl IpWhitelistMiddleware {
    pub fn new(service: IpWhitelistService) -> Self {
        Self {
            service,
            trusted_proxies: Vec::new(),
        }
    }

    /// Set trusted proxy IPs (e.g. load balancers / reverse proxies).
    pub fn with_trusted_proxies(mut self, proxies: Vec<IpAddr>) -> Self {
        self.trusted_proxies = proxies;
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

impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for IpWhitelistMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = IpWhitelistMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(IpWhitelistMiddlewareService {
            service,
            whitelist_service: self.service.clone(),
            trusted_proxies: self.trusted_proxies.clone(),
        }))
    }
}

/// The actual middleware service
pub struct IpWhitelistMiddlewareService<S> {
    service: S,
    whitelist_service: IpWhitelistService,
    trusted_proxies: Vec<IpAddr>,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for IpWhitelistMiddlewareService<S>
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
        let whitelist_service = self.whitelist_service.clone();
        let trusted_proxies = self.trusted_proxies.clone();

        // Extract tenant_id from request extensions
        let tenant_id = req
            .extensions()
            .get::<crate::middleware::tenant::TenantContext>()
            .map(|ctx| ctx.tenant_id);

        // Extract client IP using trusted proxy configuration
        let client_ip = Self::extract_client_ip(&req, &trusted_proxies);

        let fut = self.service.call(req);

        Box::pin(async move {
            // If we have a tenant_id and a client IP, check the whitelist
            if let (Some(tenant_id), Some(ip)) = (tenant_id, client_ip) {
                let allowed = whitelist_service.is_ip_allowed(tenant_id, &ip).await;
                if !allowed {
                    let response = actix_web::HttpResponse::Forbidden()
                        .json(crate::error::ErrorResponse {
                            error: "Access denied: IP address not whitelisted".to_string(),
                        })
                        .map_into_right_body::<B>();
                    return Ok(fut.await?.into_response(response));
                }
            }

            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}

impl<S> IpWhitelistMiddlewareService<S> {
    /// Extract client IP from request, considering trusted proxies.
    fn extract_client_ip(req: &ServiceRequest, trusted_proxies: &[IpAddr]) -> Option<String> {
        let peer_ip = req
            .connection_info()
            .peer_addr()
            .unwrap_or("unknown")
            .to_string();

        let may_trust_headers = if !trusted_proxies.is_empty() {
            IpWhitelistMiddleware::is_in_trusted_proxies(&peer_ip, trusted_proxies)
        } else {
            IpWhitelistMiddleware::is_loopback(&peer_ip)
        };

        if may_trust_headers {
            // Check X-Forwarded-For first
            if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
                if let Ok(forwarded_str) = forwarded.to_str() {
                    if let Some(client_ip) = forwarded_str.split(',').next() {
                        let trimmed = client_ip.trim().to_string();
                        if !trimmed.is_empty() {
                            // Validate extracted IP format to prevent injection
                            if trimmed.parse::<std::net::IpAddr>().is_ok() {
                                return Some(trimmed);
                            }
                            tracing::warn!(peer_ip = %peer_ip, forwarded = %trimmed, "Invalid IP format in X-Forwarded-For, falling back to peer IP");
                        }
                    }
                }
            }

            // Fall back to X-Real-IP
            if let Some(real_ip) = req.headers().get("X-Real-IP") {
                if let Ok(ip) = real_ip.to_str() {
                    let trimmed = ip.trim().to_string();
                    if !trimmed.is_empty() {
                        // Validate extracted IP format to prevent injection
                        if trimmed.parse::<std::net::IpAddr>().is_ok() {
                            return Some(trimmed);
                        }
                        tracing::warn!(peer_ip = %peer_ip, real_ip = %trimmed, "Invalid IP format in X-Real-IP, falling back to peer IP");
                    }
                }
            }
        }

        // Direct peer IP (not behind a trusted proxy or no forwarding headers)
        req.connection_info().peer_addr().map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ip_whitelist::repository::InMemoryIpWhitelistRepository;
    use actix_web::dev::Transform;
    use std::sync::Arc;

    #[test]
    fn test_extract_client_ip_from_connection() {
        // We can't easily test extract_client_ip without a full service request,
        // but we can verify the middleware struct creation
        let repo = Arc::new(InMemoryIpWhitelistRepository::new())
            as crate::domain::ip_whitelist::repository::BoxIpWhitelistRepository;
        let svc = IpWhitelistService::new(repo);
        let _middleware = IpWhitelistMiddleware::new(svc);
    }

    #[test]
    fn test_middleware_creation() {
        let repo = Arc::new(InMemoryIpWhitelistRepository::new())
            as crate::domain::ip_whitelist::repository::BoxIpWhitelistRepository;
        let svc = IpWhitelistService::new(repo);
        let middleware = IpWhitelistMiddleware::new(svc);
        let _transform = middleware.new_transform(mock::MockService);
    }

    #[test]
    fn test_is_loopback() {
        assert!(IpWhitelistMiddleware::is_loopback("127.0.0.1"));
        assert!(IpWhitelistMiddleware::is_loopback("::1"));
    }

    #[test]
    fn test_is_not_loopback() {
        assert!(!IpWhitelistMiddleware::is_loopback("192.168.1.1"));
        assert!(!IpWhitelistMiddleware::is_loopback("10.0.0.1"));
        assert!(!IpWhitelistMiddleware::is_loopback("unknown"));
    }

    #[test]
    fn test_is_in_trusted_proxies() {
        let proxies: Vec<IpAddr> = vec!["10.0.0.1".parse().unwrap(), "10.0.0.2".parse().unwrap()];
        assert!(IpWhitelistMiddleware::is_in_trusted_proxies(
            "10.0.0.1", &proxies
        ));
        assert!(IpWhitelistMiddleware::is_in_trusted_proxies(
            "10.0.0.2", &proxies
        ));
        assert!(!IpWhitelistMiddleware::is_in_trusted_proxies(
            "10.0.0.3", &proxies
        ));
        assert!(!IpWhitelistMiddleware::is_in_trusted_proxies(
            "invalid", &proxies
        ));
    }

    #[test]
    fn test_with_trusted_proxies() {
        let repo = Arc::new(InMemoryIpWhitelistRepository::new())
            as crate::domain::ip_whitelist::repository::BoxIpWhitelistRepository;
        let svc = IpWhitelistService::new(repo);
        let middleware =
            IpWhitelistMiddleware::new(svc).with_trusted_proxies(vec!["10.0.0.1".parse().unwrap()]);
        let _transform = middleware.new_transform(mock::MockService);
    }
}

#[cfg(test)]
mod mock {
    use actix_web::body::BoxBody;
    use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpResponse};
    use futures::future::{ready, Ready};

    pub struct MockService;

    impl actix_web::dev::Service<ServiceRequest> for MockService {
        type Response = ServiceResponse<BoxBody>;
        type Error = Error;
        type Future = Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(
            &self,
            _ctx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn call(&self, req: ServiceRequest) -> Self::Future {
            let response = HttpResponse::Ok().finish();
            ready(Ok(req.into_response(response)))
        }
    }
}
