//! Auth API endpoints (legacy — deprecated, use /api/v1/auth)

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::domain::auth::{
    AuthService, LoginRequest, LogoutRequest, RefreshTokenRequest, RegisterRequest,
};
use crate::domain::user::service::UserService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::AuthUser;
use tracing;

/// Register endpoint (public - no authentication required)
#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "Auth (Legacy)",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = LoginResponse),
        (status = 400, description = "Validation error - password requirements not met"),
        (status = 409, description = "User already exists"),
        (status = 429, description = "Rate limit exceeded")
    )
)]
#[tracing::instrument(skip(auth_service, _user_service, payload))]
pub async fn register(
    auth_service: web::Data<AuthService>,
    _user_service: web::Data<UserService>,
    payload: web::Json<RegisterRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        auth_service.register(payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Login endpoint (public - no authentication required)
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "Auth (Legacy)",
    request_body = LoginRequest,
    params(
        ("tenant_id" = i64, Query, description = "Tenant ID (required)")
    ),
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 429, description = "Rate limit exceeded")
    )
)]
#[tracing::instrument(skip(auth_service, payload, params))]
pub async fn login(
    auth_service: web::Data<AuthService>,
    payload: web::Json<LoginRequest>,
    web::Query(params): web::Query<LoginParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = params
        .tenant_id
        .ok_or_else(|| crate::error::ApiError::BadRequest("tenant_id is required".to_string()))?;
    json_resp!(
        auth_service.login(payload.into_inner(), tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Refresh token endpoint (public - no authentication required)
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "Auth (Legacy)",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Tokens refreshed successfully", body = TokenPair),
        (status = 401, description = "Invalid refresh token")
    )
)]
#[tracing::instrument(skip(auth_service, payload))]
pub async fn refresh_token(
    auth_service: web::Data<AuthService>,
    payload: web::Json<RefreshTokenRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        auth_service.refresh_token(payload.into_inner()),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Logout endpoint (revokes refresh token)
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "Auth (Legacy)",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logout successful"),
        (status = 401, description = "Invalid refresh token")
    )
)]
#[tracing::instrument(skip(auth_service, payload))]
pub async fn logout(
    auth_service: web::Data<AuthService>,
    payload: web::Json<LogoutRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        auth_service.logout(payload.into_inner()),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get current user endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "Auth (Legacy)",
    responses(
        (status = 200, description = "Current user info", body = UserResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
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
    None
}

/// Configure auth routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/auth/register").route(web::post().to(register)))
        .service(web::resource("/auth/login").route(web::post().to(login)))
        .service(web::resource("/auth/refresh").route(web::post().to(refresh_token)))
        .service(web::resource("/auth/logout").route(web::post().to(logout)))
        .service(web::resource("/auth/me").route(web::get().to(me)));
}
