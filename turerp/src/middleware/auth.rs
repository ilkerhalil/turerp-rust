//! Authentication middleware for JWT token validation
//!
//! This middleware validates JWT tokens from Authorization headers
//! and injects AuthClaims into request extensions for use by handlers.

use actix_web::body::BoxBody;
use actix_web::{
    dev::Payload, dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage, HttpRequest,
};
use futures::future::LocalBoxFuture;

use crate::error::ApiError;
use crate::utils::jwt::{AuthClaims, JwtService};

/// Paths that don't require authentication
pub const PUBLIC_PATHS: &[&str] = &[
    "/api/auth/login",
    "/api/auth/register",
    "/api/auth/refresh",
    "/health",
    "/swagger-ui",
    "/api-docs",
];

/// JWT authentication middleware
pub struct JwtAuthMiddleware {
    jwt_service: JwtService,
}

impl JwtAuthMiddleware {
    /// Create a new JWT authentication middleware
    pub fn new(jwt_service: JwtService) -> Self {
        Self { jwt_service }
    }

    /// Check if path is public (doesn't require authentication)
    fn is_public_path(path: &str) -> bool {
        PUBLIC_PATHS.iter().any(|public| path.starts_with(public))
    }

    /// Extract Bearer token from Authorization header
    fn extract_bearer_token(req: &ServiceRequest) -> Result<String, ApiError> {
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header".into()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(ApiError::Unauthorized(
                "Invalid Authorization header format. Expected: Bearer <token>".into(),
            ));
        }

        Ok(auth_header[7..].to_string())
    }

    /// Validate token and inject claims into request extensions
    fn validate_and_inject_claims(
        req: &mut ServiceRequest,
        jwt_service: &JwtService,
    ) -> Result<AuthClaims, ApiError> {
        let token = Self::extract_bearer_token(req)?;
        let claims = jwt_service.decode_token(&token)?;
        req.extensions_mut().insert(claims.clone());
        Ok(claims)
    }
}

/// Implementation of actix-web middleware for JwtAuthMiddleware
impl<S> actix_web::dev::Transform<S, ServiceRequest> for JwtAuthMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtAuthMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(JwtAuthMiddlewareService {
            service,
            jwt_service: self.jwt_service.clone(),
        }))
    }
}

/// The actual middleware service
pub struct JwtAuthMiddlewareService<S> {
    service: S,
    jwt_service: JwtService,
}

impl<S> actix_web::dev::Service<ServiceRequest> for JwtAuthMiddlewareService<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &self,
        ctx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let path = req.path().to_string();

        // Skip authentication for public paths
        if JwtAuthMiddleware::is_public_path(&path) {
            let fut = self.service.call(req);
            return Box::pin(fut);
        }

        // Validate token and inject claims
        match JwtAuthMiddleware::validate_and_inject_claims(&mut req, &self.jwt_service) {
            Ok(_claims) => {
                let fut = self.service.call(req);
                Box::pin(fut)
            }
            Err(e) => {
                let response = actix_web::HttpResponse::Unauthorized()
                    .json(crate::error::ErrorResponse {
                        error: e.to_string(),
                    })
                    .map_into_boxed_body();
                Box::pin(async move { Ok(req.into_response(response)) })
            }
        }
    }
}

/// Extract auth claims from request extensions
pub fn get_auth_claims(req: &HttpRequest) -> Result<AuthClaims, ApiError> {
    req.extensions()
        .get::<AuthClaims>()
        .cloned()
        .ok_or_else(|| ApiError::Unauthorized("No authentication claims found".to_string()))
}

/// Auth extractor for extracting claims from request
/// Use this when you need authenticated user info
pub struct AuthUser(pub AuthClaims);

impl actix_web::FromRequest for AuthUser {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let result = req
            .extensions()
            .get::<AuthClaims>()
            .cloned()
            .ok_or_else(|| actix_web::error::ErrorUnauthorized("Authentication required"));

        std::future::ready(result.map(AuthUser))
    }
}

/// Admin user extractor - only allows Admin role
/// Use this when you need admin-only endpoints
pub struct AdminUser(pub AuthClaims);

impl actix_web::FromRequest for AdminUser {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let claims = req
            .extensions()
            .get::<AuthClaims>()
            .cloned()
            .ok_or_else(|| actix_web::error::ErrorUnauthorized("Authentication required"));

        match claims {
            Ok(claims) => {
                if claims.role == "Admin" {
                    std::future::ready(Ok(AdminUser(claims)))
                } else {
                    std::future::ready(Err(actix_web::error::ErrorForbidden(
                        "Admin access required",
                    )))
                }
            }
            Err(e) => std::future::ready(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_paths() {
        assert!(JwtAuthMiddleware::is_public_path("/api/auth/login"));
        assert!(JwtAuthMiddleware::is_public_path("/api/auth/register"));
        assert!(JwtAuthMiddleware::is_public_path("/health"));
        assert!(JwtAuthMiddleware::is_public_path("/swagger-ui/index.html"));
        assert!(JwtAuthMiddleware::is_public_path("/api-docs/openapi.json"));

        assert!(!JwtAuthMiddleware::is_public_path("/api/users"));
        assert!(!JwtAuthMiddleware::is_public_path("/api/auth/me"));
    }

    #[test]
    fn test_bearer_token_extraction() {
        let service = JwtService::new("test-secret".to_string(), 3600, 604800);
        let tokens = service
            .generate_tokens(
                1,
                1,
                "test".to_string(),
                crate::domain::user::model::Role::User,
            )
            .unwrap();

        // Token should be valid
        let claims = service.decode_token(&tokens.access_token).unwrap();
        assert_eq!(claims.sub, "1");
    }
}
