//! Tenant context middleware
//!
//! This middleware extracts tenant context from incoming requests,
//! allowing for multi-tenant database isolation.

use actix_web::body::MessageBody;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage};
use futures::future::LocalBoxFuture;
use std::cell::Cell;

// Task-local tenant ID for observability.
// Stores the current tenant ID so downstream error handlers (e.g. map_sqlx_error)
// can include it in structured logs without threading it through every call site.
tokio::task_local! {
    pub static CURRENT_TENANT_ID: Cell<Option<i64>>;
}

/// Tenant context extracted from the request
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: i64,
}

/// Tenant middleware for extracting tenant context
///
/// This middleware:
/// 1. Checks if tenant context was already set by upstream auth middleware
/// 2. Falls back to X-Tenant-ID header for service-to-service calls
/// 3. Stores tenant context in request extensions
pub struct TenantMiddleware;

impl TenantMiddleware {
    /// Create a new tenant middleware instance
    pub fn new() -> Self {
        TenantMiddleware
    }

    /// Extract tenant context from request
    ///
    /// Attempts to get tenant_id from:
    /// 1. Existing TenantContext extension (set by auth middleware)
    /// 2. X-Tenant-ID header (for service-to-service calls)
    pub fn extract_tenant_id(req: &ServiceRequest) -> Option<i64> {
        // First check if tenant context was already set by upstream auth middleware
        if let Some(ctx) = req.extensions().get::<TenantContext>() {
            return Some(ctx.tenant_id);
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
impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for TenantMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
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

impl<S, B> actix_web::dev::Service<ServiceRequest> for TenantMiddlewareService<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Extract tenant_id from request
        let tenant_id = TenantMiddleware::extract_tenant_id(&req);
        if let Some(tid) = tenant_id {
            req.extensions_mut()
                .insert(TenantContext { tenant_id: tid });
        }

        let fut = self.service.call(req);

        if let Some(tid) = tenant_id {
            Box::pin(CURRENT_TENANT_ID.scope(Cell::new(Some(tid)), async move {
                let res = fut.await?;
                Ok(res)
            }))
        } else {
            Box::pin(async move {
                let res = fut.await?;
                Ok(res)
            })
        }
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
        let _middleware = TenantMiddleware;
    }
}
