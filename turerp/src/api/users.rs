//! Users API endpoints (legacy — deprecated, use /api/v1/users)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::user::model::{CreateUser, UpdateUser};
use crate::domain::user::service::UserService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::AuthUser;

/// Create user endpoint (requires authentication)
#[utoipa::path(
    post,
    path = "/api/users",
    tag = "Users (Legacy)",
    request_body = CreateUser,
    responses(
        (status = 201, description = "User created successfully", body = UserResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 409, description = "User already exists")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_user(
    _auth_user: AuthUser,
    user_service: web::Data<UserService>,
    payload: web::Json<CreateUser>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match user_service.create_user(create).await {
        Ok(user) => Ok(HttpResponse::Created().json(user)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get user by ID endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/users/{id}",
    tag = "Users (Legacy)",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User found", body = UserResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_user(
    auth_user: AuthUser,
    user_service: web::Data<UserService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = auth_user.0.tenant_id;
    match user_service.get_user(*path, tenant_id).await {
        Ok(user) => Ok(HttpResponse::Ok().json(user)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all users endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/users",
    tag = "Users (Legacy)",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of users"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_users(
    auth_user: AuthUser,
    user_service: web::Data<UserService>,
    pagination: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    let tenant_id = auth_user.0.tenant_id;
    match user_service
        .get_all_users_paginated(tenant_id, pagination.page, pagination.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update user endpoint (requires authentication)
#[utoipa::path(
    put,
    path = "/api/users/{id}",
    tag = "Users (Legacy)",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    request_body = UpdateUser,
    responses(
        (status = 200, description = "User updated", body = UserResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_user(
    auth_user: AuthUser,
    user_service: web::Data<UserService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateUser>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = auth_user.0.tenant_id;
    let id = *path;
    match user_service
        .update_user(id, tenant_id, payload.into_inner())
        .await
    {
        Ok(user) => Ok(HttpResponse::Ok().json(user)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete user endpoint (requires authentication)
#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    tag = "Users (Legacy)",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_user(
    auth_user: AuthUser,
    user_service: web::Data<UserService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = auth_user.0.tenant_id;
    let id = *path;
    match user_service.delete_user(id, tenant_id).await {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "user.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure user routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/users")
            .route(web::get().to(get_users))
            .route(web::post().to(create_user)),
    )
    .service(
        web::resource("/users/{id}")
            .route(web::get().to(get_user))
            .route(web::put().to(update_user))
            .route(web::delete().to(delete_user)),
    );
}
