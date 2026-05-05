//! Auth API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::domain::auth::{AuthService, LoginRequest, RefreshTokenRequest, RegisterRequest};
use crate::domain::user::service::UserService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::AuthUser;

/// Register endpoint (public - no authentication required)
///
/// Rate limited: 10 requests/minute per IP
#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = LoginResponse),
        (status = 400, description = "Validation error - password requirements not met"),
        (status = 409, description = "User already exists"),
        (status = 429, description = "Rate limit exceeded")
    )
)]
pub async fn register(
    auth_service: web::Data<AuthService>,
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
///
/// Rate limited: 10 requests/minute per IP
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    params(
        ("tenant_id" = Option<i64>, Query, description = "Tenant ID (default: 1)")
    ),
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 429, description = "Rate limit exceeded")
    )
)]
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
        Err(ApiError::MfaRequired(token)) => Ok(HttpResponse::Forbidden().json(
            crate::domain::mfa::MfaRequiredResponse {
                mfa_token: token,
                message: "Multi-factor authentication required".to_string(),
            },
        )),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Refresh token endpoint (public - no authentication required)
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "Auth",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Tokens refreshed successfully", body = TokenPair),
        (status = 401, description = "Invalid refresh token")
    )
)]
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
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "Auth",
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
    let user_id: i64 = auth_user.0.user_id()?;

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

/// Configure auth routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/v1/auth/register").route(web::post().to(register)))
        .service(web::resource("/v1/auth/login").route(web::post().to(login)))
        .service(web::resource("/v1/auth/refresh").route(web::post().to(refresh_token)))
        .service(web::resource("/v1/auth/me").route(web::get().to(me)));
}
