//! Security headers middleware
//!
//! Adds security-related HTTP headers to every response.

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header;
use actix_web::Error;
use futures::future::LocalBoxFuture;

use crate::config::SecurityHeadersConfig;

/// Security headers middleware
///
/// This middleware adds security-related HTTP headers to every response,
/// including HSTS, CSP, X-Frame-Options, and others.
#[derive(Default)]
pub struct SecurityHeadersMiddleware {
    config: SecurityHeadersConfig,
    is_production: bool,
}

impl SecurityHeadersMiddleware {
    /// Create a new security headers middleware instance
    pub fn new(config: &SecurityHeadersConfig, is_production: bool) -> Self {
        Self {
            config: config.clone(),
            is_production,
        }
    }
}

/// Implementation of actix-web middleware for SecurityHeadersMiddleware
impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for SecurityHeadersMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SecurityHeadersMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(SecurityHeadersMiddlewareService {
            service,
            config: self.config.clone(),
            is_production: self.is_production,
        }))
    }
}

/// The actual middleware service
pub struct SecurityHeadersMiddlewareService<S> {
    service: S,
    config: SecurityHeadersConfig,
    is_production: bool,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for SecurityHeadersMiddlewareService<S>
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
        let fut = self.service.call(req);
        let is_production = self.is_production;
        let enabled = self.config.enabled;

        Box::pin(async move {
            let mut res = fut.await?;
            if enabled {
                let headers = res.response_mut().headers_mut();
                if is_production {
                    headers.insert(
                        header::STRICT_TRANSPORT_SECURITY,
                        header::HeaderValue::from_static("max-age=31536000; includeSubDomains"),
                    );
                }
                headers.insert(
                    header::CONTENT_SECURITY_POLICY,
                    header::HeaderValue::from_static("default-src 'self'; frame-ancestors 'none'"),
                );
                headers.insert(
                    header::X_FRAME_OPTIONS,
                    header::HeaderValue::from_static("DENY"),
                );
                headers.insert(
                    header::X_CONTENT_TYPE_OPTIONS,
                    header::HeaderValue::from_static("nosniff"),
                );
                headers.insert(
                    header::REFERRER_POLICY,
                    header::HeaderValue::from_static("strict-origin-when-cross-origin"),
                );
                headers.insert(
                    header::HeaderName::from_static("x-xss-protection"),
                    header::HeaderValue::from_static("1; mode=block"),
                );
            }
            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_headers_middleware_default() {
        let middleware = SecurityHeadersMiddleware::default();
        assert!(!middleware.is_production);
        assert!(middleware.config.enabled);
    }

    #[test]
    fn test_security_headers_middleware_new() {
        let config = SecurityHeadersConfig { enabled: false };
        let middleware = SecurityHeadersMiddleware::new(&config, true);
        assert!(middleware.is_production);
        assert!(!middleware.config.enabled);
    }
}
