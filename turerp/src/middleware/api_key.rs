//! API Key authentication support
//!
//! Provides an `ApiKeyAuth` extractor for API key-based authentication.
//! Use this extractor in handlers that accept API key authentication.
//! The API key is validated via `X-API-Key` header against the `ApiKeyService`.
//!
//! Note: API key auth is NOT a middleware — it's an extractor. Handlers that
//! accept API keys should use `ApiKeyAuth` instead of (or alongside) `AuthUser`.
//! This avoids the async-in-middleware problem in actix-web.

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::api_key::model::ApiKeyScope;
use crate::domain::api_key::service::ApiKeyService;

const API_KEY_HEADER: &str = "X-API-Key";

/// API Key claims obtained after successful validation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyClaims {
    pub api_key_id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub scopes: Vec<ApiKeyScope>,
}

/// Extractor for API key authentication
///
/// Use in handlers that accept API key auth:
/// ```ignore
/// async fn my_handler(api_key: ApiKeyAuth) -> Result<HttpResponse, Error> {
///     // api_key.0.tenant_id, api_key.0.scopes, etc.
/// }
/// ```
pub struct ApiKeyAuth(pub ApiKeyClaims);

impl FromRequest for ApiKeyAuth {
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let key_value = req
            .headers()
            .get(API_KEY_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let api_key_service = req
            .app_data::<actix_web::web::Data<ApiKeyService>>()
            .map(|s| s.get_ref().clone());

        Box::pin(async move {
            let key = key_value
                .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing X-API-Key header"))?;

            let service = api_key_service.ok_or_else(|| {
                actix_web::error::ErrorInternalServerError("API key service not configured")
            })?;

            match service.authenticate(&key).await {
                Ok(api_key) => {
                    let claims = ApiKeyClaims {
                        api_key_id: api_key.id,
                        tenant_id: api_key.tenant_id,
                        user_id: api_key.user_id,
                        scopes: api_key.scopes,
                    };
                    Ok(ApiKeyAuth(claims))
                }
                Err(e) => Err(actix_web::error::ErrorUnauthorized(e.to_string())),
            }
        })
    }
}

use futures::future::LocalBoxFuture;

/// Check if an API key has a specific scope
pub fn has_scope(claims: &ApiKeyClaims, scope: &ApiKeyScope) -> bool {
    claims.scopes.contains(&ApiKeyScope::All) || claims.scopes.contains(scope)
}

/// Check if an API key has any of the given scopes
pub fn has_any_scope(claims: &ApiKeyClaims, scopes: &[ApiKeyScope]) -> bool {
    if claims.scopes.contains(&ApiKeyScope::All) {
        return true;
    }
    scopes.iter().any(|s| claims.scopes.contains(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_scope_with_all() {
        let claims = ApiKeyClaims {
            api_key_id: 1,
            tenant_id: 1,
            user_id: 1,
            scopes: vec![ApiKeyScope::All],
        };
        assert!(has_scope(&claims, &ApiKeyScope::CariRead));
        assert!(has_scope(&claims, &ApiKeyScope::InvoiceWrite));
    }

    #[test]
    fn test_has_scope_specific() {
        let claims = ApiKeyClaims {
            api_key_id: 1,
            tenant_id: 1,
            user_id: 1,
            scopes: vec![ApiKeyScope::CariRead, ApiKeyScope::CariWrite],
        };
        assert!(has_scope(&claims, &ApiKeyScope::CariRead));
        assert!(has_scope(&claims, &ApiKeyScope::CariWrite));
        assert!(!has_scope(&claims, &ApiKeyScope::InvoiceRead));
    }

    #[test]
    fn test_has_any_scope() {
        let claims = ApiKeyClaims {
            api_key_id: 1,
            tenant_id: 1,
            user_id: 1,
            scopes: vec![ApiKeyScope::CariRead],
        };
        assert!(has_any_scope(
            &claims,
            &[ApiKeyScope::CariRead, ApiKeyScope::InvoiceRead]
        ));
        assert!(!has_any_scope(
            &claims,
            &[ApiKeyScope::InvoiceRead, ApiKeyScope::StockWrite]
        ));
    }
}
