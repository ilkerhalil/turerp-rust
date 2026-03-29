//! Auth API endpoints

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::domain::auth::{AuthService, LoginRequest, RefreshTokenRequest, RegisterRequest};
use crate::domain::user::service::UserService;
use crate::error::ApiResult;
use crate::middleware::AuthUser;

/// Register endpoint (public - no authentication required)
///
/// Rate limited: 10 requests/minute per IP
#[utoipa::path(
    post,
    path = "/api/auth/register",
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
    _user_service: web::Data<UserService>,
    payload: web::Json<RegisterRequest>,
) -> ApiResult<HttpResponse> {
    let response = auth_service.register(payload.into_inner()).await?;
    Ok(HttpResponse::Created().json(response))
}

/// Login endpoint (public - no authentication required)
///
/// Rate limited: 10 requests/minute per IP
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
        (status = 401, description = "Invalid credentials"),
        (status = 429, description = "Rate limit exceeded")
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

/// Refresh token endpoint (public - no authentication required)
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

/// Get current user endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/auth/me",
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
) -> ApiResult<HttpResponse> {
    // Extract user ID from JWT claims
    let user_id: i64 = auth_user
        .0
        .sub
        .parse()
        .map_err(|_| crate::error::ApiError::InvalidToken("Invalid user ID in token".into()))?;

    let tenant_id = auth_user.0.tenant_id;

    // Fetch user from database
    let user = user_service.get_user(user_id, tenant_id).await?;

    Ok(HttpResponse::Ok().json(user))
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
