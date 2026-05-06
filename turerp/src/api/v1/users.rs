//! Users API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::Serialize;

use crate::common::pagination::PaginationParams;
use crate::domain::user::model::{CreateUser, UpdateUser};
use crate::domain::user::service::UserService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Simple localized success message payload.
#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}

/// Create user endpoint (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "Users",
    request_body = CreateUser,
    responses(
        (status = 201, description = "User created successfully", body = UserResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 409, description = "User already exists")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_user(
    _admin_user: AdminUser,
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
    path = "/api/v1/users/{id}",
    tag = "Users",
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
    path = "/api/v1/users",
    tag = "Users",
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

/// Update user endpoint (requires authentication, self or admin for role changes)
#[utoipa::path(
    put,
    path = "/api/v1/users/{id}",
    tag = "Users",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    request_body = UpdateUser,
    responses(
        (status = 200, description = "User updated", body = UserResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - can only update own profile or admin role required for role changes"),
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
    let user_id = auth_user.0.user_id()?;
    let id = *path;
    let update = payload.into_inner();

    let is_self = user_id == id;
    let is_admin = auth_user.0.role == "admin";
    let is_role_change = update.role.is_some();

    if !is_self && !is_admin {
        let msg = i18n.t(locale.as_str(), "user.self_update_only");
        return Ok(HttpResponse::Forbidden().json(crate::error::ErrorResponse { error: msg }));
    }

    if is_role_change && !is_admin {
        let msg = i18n.t(locale.as_str(), "user.role_change_forbidden");
        return Ok(HttpResponse::Forbidden().json(crate::error::ErrorResponse { error: msg }));
    }

    match user_service.update_user(id, tenant_id, update).await {
        Ok(user) => Ok(HttpResponse::Ok().json(user)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete user endpoint (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    tag = "Users",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_user(
    admin_user: AdminUser,
    user_service: web::Data<UserService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = admin_user.0.tenant_id;
    let deleted_by = admin_user.0.user_id()?;
    let id = *path;
    match user_service
        .soft_delete_user(id, tenant_id, deleted_by)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "user.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted user (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/restore",
    tag = "Users",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User restored", body = UserResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn restore_user(
    admin_user: AdminUser,
    user_service: web::Data<UserService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = admin_user.0.tenant_id;
    let id = *path;
    match user_service.restore_user(id, tenant_id).await {
        Ok(user) => Ok(HttpResponse::Ok().json(user)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List deleted users for a tenant (requires admin role)
#[utoipa::path(
    get,
    path = "/api/v1/users/deleted",
    tag = "Users",
    responses(
        (status = 200, description = "List of deleted users"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_deleted_users(
    admin_user: AdminUser,
    user_service: web::Data<UserService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = admin_user.0.tenant_id;
    match user_service.list_deleted_users(tenant_id).await {
        Ok(users) => Ok(HttpResponse::Ok().json(users)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Permanently destroy a user (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}/destroy",
    tag = "Users",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User permanently destroyed", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn destroy_user(
    _admin_user: AdminUser,
    user_service: web::Data<UserService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = _admin_user.0.tenant_id;
    let id = *path;
    match user_service.destroy_user(id, tenant_id).await {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "user.destroyed");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure user routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/users")
            .route(web::get().to(get_users))
            .route(web::post().to(create_user)),
    )
    .service(web::resource("/v1/users/deleted").route(web::get().to(list_deleted_users)))
    .service(
        web::resource("/v1/users/{id}")
            .route(web::get().to(get_user))
            .route(web::put().to(update_user))
            .route(web::delete().to(delete_user)),
    )
    .service(web::resource("/v1/users/{id}/restore").route(web::post().to(restore_user)))
    .service(web::resource("/v1/users/{id}/destroy").route(web::delete().to(destroy_user)));
}
