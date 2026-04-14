//! Audit logging middleware
//!
//! Logs request method, path, user info, and response status for
//! authenticated requests at INFO level, and public endpoints at DEBUG level.
//! Error responses are logged at WARN level.

use actix_web::body::BoxBody;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures::future::{ok, LocalBoxFuture, Ready};

use crate::middleware::auth::get_auth_claims;

/// Audit logging middleware factory
pub struct AuditLoggingMiddleware;

impl<S> Transform<S, ServiceRequest> for AuditLoggingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = AuditLoggingMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuditLoggingMiddlewareService { service })
    }
}

/// Audit logging middleware service
pub struct AuditLoggingMiddlewareService<S> {
    service: S,
}

impl<S> Service<ServiceRequest> for AuditLoggingMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let path = req.path().to_string();
        let auth_info = get_auth_claims(req.request()).ok();

        let fut = self.service.call(req);

        Box::pin(async move {
            let response = fut.await?;

            let status = response.status();

            if let Some(claims) = auth_info {
                if status.is_client_error() || status.is_server_error() {
                    tracing::warn!(
                        method = %method,
                        path = %path,
                        status = %status.as_u16(),
                        user_id = %claims.sub,
                        tenant_id = %claims.tenant_id,
                        role = %claims.role,
                        "Request completed with error"
                    );
                } else {
                    tracing::info!(
                        method = %method,
                        path = %path,
                        status = %status.as_u16(),
                        user_id = %claims.sub,
                        tenant_id = %claims.tenant_id,
                        role = %claims.role,
                        "Request completed"
                    );
                }
            } else {
                tracing::debug!(
                    method = %method,
                    path = %path,
                    status = %status.as_u16(),
                    "Unauthenticated request completed"
                );
            }

            Ok(response)
        })
    }
}
