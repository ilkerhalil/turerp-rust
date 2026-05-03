//! Product Variants API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::product::{CreateProductVariant, ProductService, UpdateProductVariant};
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

// --- Products ---

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
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Categories ---

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
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Create product variant endpoint (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/products/{product_id}/variants",
    tag = "Products",
    params(
        ("product_id" = i64, Path, description = "Product ID")
    ),
    request_body = CreateProductVariant,
    responses(
        (status = 201, description = "Product variant created successfully", body = ProductVariantResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Product not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_variant(
    _admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    payload: web::Json<CreateProductVariant>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let product_id = path.into_inner();
    let mut create = payload.into_inner();
    create.product_id = product_id; // Ensure product_id matches path

    match service.create_variant(create).await {
        Ok(variant) => Ok(HttpResponse::Created().json(variant)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all variants for a product endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/products/{product_id}/variants",
    tag = "Products",
    params(
        ("product_id" = i64, Path, description = "Product ID")
    ),
    responses(
        (status = 200, description = "Product variants found", body = Vec<ProductVariantResponse>),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Product not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_variants_by_product(
    _auth_user: AuthUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let product_id = path.into_inner();
    match service.get_variants_by_product(product_id).await {
        Ok(variants) => Ok(HttpResponse::Ok().json(variants)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get product variant by ID endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/variants/{id}",
    tag = "Products",
    params(
        ("id" = i64, Path, description = "Variant ID")
    ),
    responses(
        (status = 200, description = "Product variant found", body = ProductVariantResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Variant not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_variant(
    _auth_user: AuthUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.get_variant(*path).await {
        Ok(variant) => Ok(HttpResponse::Ok().json(variant)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update product variant endpoint (requires admin role)
#[utoipa::path(
    put,
    path = "/api/v1/variants/{id}",
    tag = "Products",
    params(
        ("id" = i64, Path, description = "Variant ID")
    ),
    request_body = UpdateProductVariant,
    responses(
        (status = 200, description = "Product variant updated", body = ProductVariantResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Variant not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_variant(
    _admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateProductVariant>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.update_variant(*path, payload.into_inner()).await {
        Ok(variant) => Ok(HttpResponse::Ok().json(variant)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete product variant endpoint (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/variants/{id}",
    tag = "Products",
    params(
        ("id" = i64, Path, description = "Variant ID")
    ),
    responses(
        (status = 200, description = "Product variant deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Variant not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_variant(
    _admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.delete_variant(*path).await {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "product.variant.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure product variant routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/v1/products").route(web::get().to(get_products)))
        .service(web::resource("/v1/categories").route(web::get().to(get_categories)))
        .service(
            web::resource("/v1/products/{product_id}/variants")
                .route(web::get().to(get_variants_by_product))
                .route(web::post().to(create_variant)),
        )
        .service(
            web::resource("/v1/variants/{id}")
                .route(web::get().to(get_variant))
                .route(web::put().to(update_variant))
                .route(web::delete().to(delete_variant)),
        );
}
