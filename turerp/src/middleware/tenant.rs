//! Tenant context middleware
//!
//! This middleware extracts tenant context from incoming requests,
//! allowing for multi-tenant database isolation.

use actix_web::body::BoxBody;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage};
use futures::future::LocalBoxFuture;

use crate::middleware::AuthUser;

/// Tenant context extracted from the request
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: i64,
}

/// Tenant middleware for extracting tenant context
///
/// This middleware:
/// 1. Extracts tenant_id from authenticated user (if present)
/// 2. Stores tenant context in request extensions
/// 3. Makes tenant_id available to downstream handlers
pub struct TenantMiddleware;

impl TenantMiddleware {
    /// Create a new tenant middleware instance
    pub fn new() -> Self {
        TenantMiddleware
    }

    /// Extract tenant context from request
    ///
    /// Attempts to get tenant_id from:
    /// 1. AuthUser extension (from JWT claims)
    /// 2. X-Tenant-ID header (for service-to-service calls)
    pub fn extract_tenant_id(req: &ServiceRequest) -> Option<i64> {
        // First try to get from authenticated user
        if let Some(auth_user) = req.extensions().get::<AuthUser>() {
            return Some(auth_user.0.tenant_id);
        }

        // Fall back to header for internal service calls
        req.headers()
            .get("X-Tenant-ID")
            .and_then(|header| header.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
    }
}

impl Default for TenantMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of actix-web middleware for TenantMiddleware
impl<S> actix_web::dev::Transform<S, ServiceRequest> for TenantMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = TenantMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(TenantMiddlewareService { service }))
    }
}

/// The actual middleware service
pub struct TenantMiddlewareService<S> {
    service: S,
}

impl<S> actix_web::dev::Service<ServiceRequest> for TenantMiddlewareService<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Extract tenant_id from request
        if let Some(tenant_id) = TenantMiddleware::extract_tenant_id(&req) {
            // Store tenant context in extensions
            req.extensions_mut().insert(TenantContext { tenant_id });
        }

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}

/// Extension trait for getting tenant context from request
pub trait TenantContextExt {
    /// Get the tenant ID from the request
    fn tenant_id(&self) -> Option<i64>;
}

impl TenantContextExt for actix_web::HttpRequest {
    fn tenant_id(&self) -> Option<i64> {
        self.extensions()
            .get::<TenantContext>()
            .map(|ctx| ctx.tenant_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_context_creation() {
        let ctx = TenantContext { tenant_id: 123 };
        assert_eq!(ctx.tenant_id, 123);
    }

    #[test]
    fn test_default_middleware() {
        let _middleware = TenantMiddleware::default();
    }
}
