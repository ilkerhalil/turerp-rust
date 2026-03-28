//! Auth API endpoints

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::domain::auth::{AuthService, LoginRequest, RefreshTokenRequest, RegisterRequest};
use crate::domain::user::service::UserService;
use crate::error::ApiResult;

/// Register endpoint
#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = LoginResponse),
        (status = 400, description = "Validation error"),
        (status = 409, description = "User already exists")
    )
)]
pub async fn register(
    auth_service: web::Data<AuthService>,
    _user_service: web::Data<UserService>,
    payload: web::Json<RegisterRequest>,
) -> ApiResult<HttpResponse> {
    let response = auth_service.register(payload.into_inner()).await?;
    Ok(HttpResponse::Created().json(response))
}

/// Login endpoint
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    params(
        ("tenant_id" = Option<i64>, Query, description = "Tenant ID (default: 1)")
    ),
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn login(
    auth_service: web::Data<AuthService>,
    payload: web::Json<LoginRequest>,
    web::Query(params): web::Query<LoginParams>,
) -> ApiResult<HttpResponse> {
    let tenant_id = params.tenant_id.unwrap_or(1);
    let response = auth_service.login(payload.into_inner(), tenant_id).await?;
    Ok(HttpResponse::Ok().json(response))
}

/// Refresh token endpoint
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
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
) -> ApiResult<HttpResponse> {
    let tokens = auth_service.refresh_token(payload.into_inner()).await?;
    Ok(HttpResponse::Ok().json(tokens))
}

/// Get current user endpoint
#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "Auth",
    responses(
        (status = 200, description = "Current user info")
    )
)]
pub async fn me(_user_service: web::Data<UserService>) -> ApiResult<HttpResponse> {
    // TODO: Get from auth context
    // For now, return a placeholder
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "message": "Auth context not implemented - use login to get tokens"
    })))
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
