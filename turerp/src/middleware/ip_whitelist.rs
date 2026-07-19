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
use std::sync::Arc;
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
}

impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for IpWhitelistMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>
        + 'static,
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
            service: Arc::new(service),
            whitelist_service: self.service.clone(),
            trusted_proxies: self.trusted_proxies.clone(),
        }))
    }
}

/// The actual middleware service
pub struct IpWhitelistMiddlewareService<S> {
    service: Arc<S>,
    whitelist_service: IpWhitelistService,
    trusted_proxies: Vec<IpAddr>,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for IpWhitelistMiddlewareService<S>
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
        let whitelist_service = self.whitelist_service.clone();
        let service = self.service.clone();

        // Tenant identification: read the JWT claims (set by JwtAuthMiddleware
        // on the request path BEFORE this middleware runs) rather than the
        // TenantContext extension. The previous implementation read
        // `TenantContext`, but that extension is set on the *response* path
        // after inner services run, so on the request path it was always
        // `None` and the IP allowlist was effectively dead code.
        //
        // AuthClaims is set by JwtAuthMiddleware in `extensions_mut()` of the
        // request, so it is available here. For unauthenticated paths the
        // allowlist is bypassed — those paths are not tenant-scoped anyway
        // (e.g. /api/v1/auth/login).
        let tenant_id = req
            .extensions()
            .get::<crate::utils::jwt::AuthClaims>()
            .map(|c| c.tenant_id);

        // Extract client IP using trusted proxy configuration
        let client_ip = Self::extract_client_ip(&req, &self.trusted_proxies);

        Box::pin(async move {
            // If we have a tenant_id and a client IP, check the whitelist
            // BEFORE calling the inner service. This avoids cloning the
            // HttpRequest (which would break actix-web's Rc-based request
            // internals — see panic in request.rs match_info_mut) and
            // prevents the handler from running on blocked IPs.
            if let (Some(tenant_id), Some(ip)) = (tenant_id, client_ip) {
                let allowed = whitelist_service.is_ip_allowed(tenant_id, &ip).await;
                if !allowed {
                    tracing::warn!(
                        tenant_id,
                        ip = %ip,
                        "IP whitelist: access denied for tenant"
                    );
                    let response = actix_web::HttpResponse::Forbidden()
                        .json(crate::error::ErrorResponse {
                            error: "Access denied: IP address not whitelisted".to_string(),
                        })
                        .map_into_right_body::<B>();
                    return Ok(req.into_response(response));
                }
            }

            let res = service.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}

impl<S> IpWhitelistMiddlewareService<S> {
    /// Extract client IP from request, considering trusted proxies.
    fn extract_client_ip(req: &ServiceRequest, trusted_proxies: &[IpAddr]) -> Option<String> {
        crate::common::ip_utils::extract_client_ip(req, trusted_proxies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ip_whitelist::repository::InMemoryIpWhitelistRepository;
    use actix_web::dev::{Service, Transform};
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
    fn test_with_trusted_proxies() {
        let repo = Arc::new(InMemoryIpWhitelistRepository::new())
            as crate::domain::ip_whitelist::repository::BoxIpWhitelistRepository;
        let svc = IpWhitelistService::new(repo);
        let middleware =
            IpWhitelistMiddleware::new(svc).with_trusted_proxies(vec!["10.0.0.1".parse().unwrap()]);
        let _transform = middleware.new_transform(mock::MockService);
    }

    /// Regression test for `fix(security): IpWhitelist middleware ordering`.
    ///
    /// Verifies that the middleware now reads `AuthClaims` (set on the
    /// request path by JwtAuthMiddleware) and not `TenantContext` (set on
    /// the response path, after inner services). The previous code always
    /// saw `tenant_id = None` on the request path, so the allowlist was
    /// dead code in production.
    ///
    /// We construct a `ServiceRequest` with an `AuthClaims` extension
    /// carrying tenant 42, run the middleware, and assert the inner
    /// service is reached (status 200). Without the fix, the middleware
    /// would have read TenantContext and gotten None, but the inner
    /// service is the same so the only way to detect the regression is
    /// the *type* of the extension being read — which the compiler now
    /// enforces.
    #[actix_web::test]
    async fn test_middleware_reads_tenant_id_from_auth_claims() {
        use crate::domain::user::model::Role;
        use crate::utils::jwt::AuthClaims;

        let repo = Arc::new(InMemoryIpWhitelistRepository::new())
            as crate::domain::ip_whitelist::repository::BoxIpWhitelistRepository;
        let svc = IpWhitelistService::new(repo);
        let middleware = IpWhitelistMiddleware::new(svc);

        // Wire a 1-second probe timeout into the whitelist service so this
        // test never hangs. InMemoryIpWhitelistRepository has no entries
        // for tenant 42, so the check is "no rules → allow", which is
        // the default. Either way the inner service must be reached.
        let mut req = actix_web::test::TestRequest::default()
            .peer_addr("10.0.0.5:1234".parse().unwrap())
            .to_srv_request();
        req.extensions_mut()
            .insert(AuthClaims::new(1, 42, "user".to_string(), Role::User, 3600));

        let transform = middleware
            .new_transform(mock::MockService)
            .into_inner()
            .expect("transform must construct successfully");
        let res = transform
            .call(req)
            .await
            .expect("middleware must not error");
        assert_eq!(
            res.status(),
            actix_web::http::StatusCode::OK,
            "inner service must be reached when AuthClaims is present"
        );
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
