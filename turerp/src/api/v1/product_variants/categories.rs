//! Category handlers

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::product::{CategoryResponse, CreateCategory, ProductService, UpdateCategory};
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Get all categories (paginated)
#[utoipa::path(
    get,
    path = "/api/v1/categories",
    tag = "Products",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of categories"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_categories(
    auth_user: AuthUser,
    service: web::Data<ProductService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .get_categories_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await
    {
        Ok(result) => {
            let response = crate::common::pagination::PaginatedResult::new(
                result
                    .items
                    .into_iter()
                    .map(CategoryResponse::from)
                    .collect(),
                result.page,
                result.per_page,
                result.total,
            );
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Create a new category (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/categories",
    tag = "Products",
    request_body = CreateCategory,
    responses(
        (status = 201, description = "Category created successfully", body = CategoryResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_category(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    payload: web::Json<CreateCategory>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;

    match service.create_category(create).await {
        Ok(category) => Ok(HttpResponse::Created().json(CategoryResponse::from(category))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a single category by ID
#[utoipa::path(
    get,
    path = "/api/v1/categories/{id}",
    tag = "Products",
    params(("id" = i64, Path, description = "Category ID")),
    responses(
        (status = 200, description = "Category found", body = CategoryResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Category not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_category(
    auth_user: AuthUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.get_category(*path, auth_user.0.tenant_id).await {
        Ok(category) => Ok(HttpResponse::Ok().json(CategoryResponse::from(category))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update a category (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/categories/{id}",
    tag = "Products",
    params(("id" = i64, Path, description = "Category ID")),
    request_body = UpdateCategory,
    responses(
        (status = 200, description = "Category updated", body = CategoryResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Category not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_category(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateCategory>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .update_category(*path, admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(category) => Ok(HttpResponse::Ok().json(CategoryResponse::from(category))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete a category (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/categories/{id}",
    tag = "Products",
    params(("id" = i64, Path, description = "Category ID")),
    responses(
        (status = 200, description = "Category soft deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Category not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_category(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .soft_delete_category(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "category.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted category (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/categories/{id}/restore",
    params(("id" = i64, Path, description = "Category ID")),
    responses(
        (status = 200, description = "Category restored", body = CategoryResponse),
        (status = 404, description = "Category not found or not deleted"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn restore_category(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let category = service
        .restore_category(*path, admin_user.0.tenant_id)
        .await?;
    let response: CategoryResponse = category.into();
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted categories (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/categories/deleted",
    responses(
        (status = 200, description = "List of deleted categories"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_categories(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
) -> ApiResult<HttpResponse> {
    let categories: Vec<_> = service
        .list_deleted_categories(admin_user.0.tenant_id)
        .await?
        .into_iter()
        .map(CategoryResponse::from)
        .collect();
    Ok(HttpResponse::Ok().json(categories))
}

/// Permanently delete a category (admin only, after soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/categories/{id}/destroy",
    params(("id" = i64, Path, description = "Category ID")),
    responses(
        (status = 204, description = "Category permanently deleted"),
        (status = 404, description = "Category not found"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn destroy_category(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    service
        .destroy_category(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
