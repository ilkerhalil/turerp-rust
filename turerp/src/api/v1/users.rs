//! Users API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::user::model::{CreateUser, UpdateUser};
use crate::domain::user::service::UserService;
use crate::error::ApiResult;
use crate::middleware::{AdminUser, AuthUser};

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
) -> ApiResult<HttpResponse> {
    let create = payload.into_inner();
    let user = user_service.create_user(create).await?;
    Ok(HttpResponse::Created().json(user))
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
) -> ApiResult<HttpResponse> {
    let tenant_id = auth_user.0.tenant_id;
    let user = user_service.get_user(*path, tenant_id).await?;
    Ok(HttpResponse::Ok().json(user))
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
) -> ApiResult<HttpResponse> {
    pagination
        .validate()
        .map_err(crate::error::ApiError::Validation)?;
    let tenant_id = auth_user.0.tenant_id;
    let result = user_service
        .get_all_users_paginated(tenant_id, pagination.page, pagination.per_page)
        .await?;
    Ok(HttpResponse::Ok().json(result))
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
) -> ApiResult<HttpResponse> {
    let tenant_id = auth_user.0.tenant_id;
    let user_id = auth_user.0.sub.parse::<i64>().unwrap_or(0);
    let id = *path;
    let update = payload.into_inner();

    // Check authorization: users can update their own profile, but only admins can change roles
    let is_self = user_id == id;
    let is_admin = auth_user.0.role == "admin";
    let is_role_change = update.role.is_some();

    if !is_self && !is_admin {
        return Err(crate::error::ApiError::Forbidden(
            "Can only update own profile".to_string(),
        ));
    }

    // Non-admins cannot change roles
    if is_role_change && !is_admin {
        return Err(crate::error::ApiError::Forbidden(
            "Only admins can change user roles".to_string(),
        ));
    }

    let user = user_service.update_user(id, tenant_id, update).await?;
    Ok(HttpResponse::Ok().json(user))
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
        (status = 204, description = "User deleted"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_user(
    _admin_user: AdminUser,
    user_service: web::Data<UserService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    // AdminUser extractor ensures only admins can access this endpoint
    // The underscore prefix indicates we don't need the claims for this operation
    let tenant_id = _admin_user.0.tenant_id;
    let id = *path;
    user_service.delete_user(id, tenant_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Configure user routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/users")
            .route(web::get().to(get_users))
            .route(web::post().to(create_user)),
    )
    .service(
        web::resource("/v1/users/{id}")
            .route(web::get().to(get_user))
            .route(web::put().to(update_user))
            .route(web::delete().to(delete_user)),
    );
}
