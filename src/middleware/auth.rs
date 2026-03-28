//! Authentication middleware types

use crate::error::ApiError;
use crate::utils::jwt::AuthClaims;
use actix_web::HttpMessage;

/// Extract auth claims from request
pub fn get_auth_claims(req: &actix_web::HttpRequest) -> Result<AuthClaims, ApiError> {
    req.extensions()
        .get::<AuthClaims>()
        .cloned()
        .ok_or_else(|| ApiError::Unauthorized("No authentication claims found".to_string()))
}

/// Auth extractor for extracting claims from request
pub struct AuthUser(pub AuthClaims);

impl actix_web::FromRequest for AuthUser {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let result = req
            .extensions()
            .get::<AuthClaims>()
            .cloned()
            .ok_or_else(|| actix_web::error::ErrorUnauthorized("No authentication claims found"));

        std::future::ready(result.map(AuthUser))
    }
}
