//! Auth API endpoints (legacy — deprecated, use /api/v1/auth)

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::domain::auth::{AuthService, LoginRequest, RefreshTokenRequest, RegisterRequest};
use crate::domain::user::service::UserService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::AuthUser;

/// Register endpoint (public - no authentication required)
pub async fn register(
    auth_service: web::Data<AuthService>,
    _user_service: web::Data<UserService>,
    payload: web::Json<RegisterRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match auth_service.register(payload.into_inner()).await {
        Ok(response) => Ok(HttpResponse::Created().json(response)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Login endpoint (public - no authentication required)
pub async fn login(
    auth_service: web::Data<AuthService>,
    payload: web::Json<LoginRequest>,
    web::Query(params): web::Query<LoginParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = params.tenant_id.unwrap_or(1);
    match auth_service.login(payload.into_inner(), tenant_id).await {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Refresh token endpoint (public - no authentication required)
pub async fn refresh_token(
    auth_service: web::Data<AuthService>,
    payload: web::Json<RefreshTokenRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match auth_service.refresh_token(payload.into_inner()).await {
        Ok(tokens) => Ok(HttpResponse::Ok().json(tokens)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get current user endpoint (requires authentication)
pub async fn me(
    auth_user: AuthUser,
    user_service: web::Data<UserService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let user_id: i64 = auth_user
        .0
        .sub
        .parse()
        .map_err(|_| crate::error::ApiError::InvalidToken("Invalid user ID in token".into()))?;

    let tenant_id = auth_user.0.tenant_id;

    match user_service.get_user(user_id, tenant_id).await {
        Ok(user) => Ok(HttpResponse::Ok().json(user)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct LoginParams {
    #[serde(default = "default_tenant_id")]
    pub tenant_id: Option<i64>,
}

fn default_tenant_id() -> Option<i64> {
    Some(1)
}

/// Configure auth routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/auth/register").route(web::post().to(register)))
        .service(web::resource("/auth/login").route(web::post().to(login)))
        .service(web::resource("/auth/refresh").route(web::post().to(refresh_token)))
        .service(web::resource("/auth/me").route(web::get().to(me)));
}
