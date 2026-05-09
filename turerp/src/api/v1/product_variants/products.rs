//! Product handlers

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::product::ProductService;
use crate::domain::product::{CreateProduct, ProductResponse};
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Get all products (paginated)
#[utoipa::path(
    get,
    path = "/api/v1/products",
    tag = "Products",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated list of products"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_products(
    auth_user: AuthUser,
    service: web::Data<ProductService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .get_products_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await
    {
        Ok(result) => {
            let response = crate::common::pagination::PaginatedResult::new(
                result
                    .items
                    .into_iter()
                    .map(ProductResponse::from)
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

/// Create a new product (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/products",
    tag = "Products",
    request_body = CreateProduct,
    responses(
        (status = 201, description = "Product created successfully", body = ProductResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 409, description = "Product code already exists")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_product(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    payload: web::Json<CreateProduct>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;

    match service.create_product(create).await {
        Ok(product) => Ok(HttpResponse::Created().json(ProductResponse::from(product))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a single product by ID
#[utoipa::path(
    get,
    path = "/api/v1/products/{id}",
    tag = "Products",
    params(("id" = i64, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Product found", body = ProductResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Product not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_product(
    auth_user: AuthUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.get_product(*path, auth_user.0.tenant_id).await {
        Ok(product) => Ok(HttpResponse::Ok().json(ProductResponse::from(product))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update a product (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/products/{id}",
    tag = "Products",
    params(("id" = i64, Path, description = "Product ID")),
    request_body = crate::domain::product::UpdateProduct,
    responses(
        (status = 200, description = "Product updated", body = ProductResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Product not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_product(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    payload: web::Json<crate::domain::product::UpdateProduct>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .update_product(*path, admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(product) => Ok(HttpResponse::Ok().json(ProductResponse::from(product))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete a product (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/products/{id}",
    tag = "Products",
    params(("id" = i64, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Product soft deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Product not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_product(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .soft_delete_product(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "product.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted product (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/products/{id}/restore",
    params(("id" = i64, Path, description = "Product ID")),
    responses(
        (status = 200, description = "Product restored", body = ProductResponse),
        (status = 404, description = "Product not found or not deleted"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn restore_product(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let product = service
        .restore_product(*path, admin_user.0.tenant_id)
        .await?;
    let response: ProductResponse = product.into();
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted products (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/products/deleted",
    responses(
        (status = 200, description = "List of deleted products"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_products(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
) -> ApiResult<HttpResponse> {
    let products: Vec<_> = service
        .list_deleted_products(admin_user.0.tenant_id)
        .await?
        .into_iter()
        .map(ProductResponse::from)
        .collect();
    Ok(HttpResponse::Ok().json(products))
}

/// Permanently delete a product (admin only, after soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/products/{id}/destroy",
    params(("id" = i64, Path, description = "Product ID")),
    responses(
        (status = 204, description = "Product permanently deleted"),
        (status = 404, description = "Product not found"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn destroy_product(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    service
        .destroy_product(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Search products (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/products/search",
    tag = "Products",
    params(("q" = String, Query, description = "Search query")),
    responses(
        (status = 200, description = "Search results", body = Vec<ProductResponse>),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn search_products(
    auth_user: AuthUser,
    service: web::Data<ProductService>,
    query: web::Query<SearchQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = query.validate() {
        let err = crate::error::ApiError::Validation(e);
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    match service
        .search_products(auth_user.0.tenant_id, &query.q)
        .await
    {
        Ok(products) => {
            let responses: Vec<ProductResponse> =
                products.into_iter().map(ProductResponse::from).collect();
            Ok(HttpResponse::Ok().json(responses))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "crate::common::pagination::default_page")]
    pub page: u32,
    #[serde(default = "crate::common::pagination::default_per_page")]
    pub per_page: u32,
}

impl SearchQuery {
    /// Validate search query
    pub fn validate(&self) -> Result<(), String> {
        if self.q.trim().is_empty() {
            return Err("Search query cannot be empty".to_string());
        }
        if self.q.len() > 200 {
            return Err("Search query must be at most 200 characters".to_string());
        }
        Ok(())
    }
}
