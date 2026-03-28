//! Users API endpoints

use actix_web::{web, HttpResponse};

use crate::domain::user::model::{CreateUser, UpdateUser};
use crate::domain::user::service::UserService;
use crate::error::ApiResult;

/// Create user endpoint
#[utoipa::path(
    post,
    path = "/api/users",
    tag = "Users",
    request_body = CreateUser,
    responses(
        (status = 201, description = "User created successfully", body = UserResponse),
        (status = 400, description = "Validation error"),
        (status = 409, description = "User already exists")
    )
)]
pub async fn create_user(
    user_service: web::Data<UserService>,
    payload: web::Json<CreateUser>,
) -> ApiResult<HttpResponse> {
    let create = payload.into_inner();
    let user = user_service.create_user(create).await?;
    Ok(HttpResponse::Created().json(user))
}

/// Get user by ID endpoint
#[utoipa::path(
    get,
    path = "/api/users/{id}",
    tag = "Users",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User found", body = UserResponse),
        (status = 404, description = "User not found")
    )
)]
pub async fn get_user(
    user_service: web::Data<UserService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    // Use default tenant_id for now
    let user = user_service.get_user(*path, 1).await?;
    Ok(HttpResponse::Ok().json(user))
}

/// Get all users endpoint
#[utoipa::path(
    get,
    path = "/api/users",
    tag = "Users",
    responses(
        (status = 200, description = "Users found", body = Vec<UserResponse>)
    )
)]
pub async fn get_users(user_service: web::Data<UserService>) -> ApiResult<HttpResponse> {
    // Use default tenant_id for now
    let users = user_service.get_all_users(1).await?;
    Ok(HttpResponse::Ok().json(users))
}

/// Update user endpoint
#[utoipa::path(
    put,
    path = "/api/users/{id}",
    tag = "Users",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    request_body = UpdateUser,
    responses(
        (status = 200, description = "User updated", body = UserResponse),
        (status = 404, description = "User not found")
    )
)]
pub async fn update_user(
    user_service: web::Data<UserService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateUser>,
) -> ApiResult<HttpResponse> {
    let id = *path;
    let user = user_service
        .update_user(id, 1, payload.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(user))
}

/// Delete user endpoint
#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    tag = "Users",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 204, description = "User deleted"),
        (status = 404, description = "User not found")
    )
)]
pub async fn delete_user(
    user_service: web::Data<UserService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let id = *path;
    user_service.delete_user(id, 1).await?;
    Ok(HttpResponse::NoContent().finish())
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
