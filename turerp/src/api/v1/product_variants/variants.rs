//! Product variant handlers

use actix_web::{web, HttpResponse};

use crate::common::MessageResponse;
use crate::domain::product::{CreateProductVariant, ProductService, UpdateProductVariant};
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

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
    admin_user: AdminUser,
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

    match service.create_variant(create, admin_user.0.tenant_id).await {
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
    auth_user: AuthUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let product_id = path.into_inner();
    match service
        .get_variants_by_product(product_id, auth_user.0.tenant_id)
        .await
    {
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

/// Soft delete product variant endpoint (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/variants/{id}",
    tag = "Products",
    params(
        ("id" = i64, Path, description = "Variant ID")
    ),
    responses(
        (status = 200, description = "Product variant soft deleted", body = MessageResponse),
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
    match service
        .soft_delete_variant(*path, _admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "product.variant.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted product variant (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/variants/{id}/restore",
    params(("id" = i64, Path, description = "Variant ID")),
    responses(
        (status = 200, description = "Product variant restored", body = ProductVariantResponse),
        (status = 404, description = "Variant not found or not deleted"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn restore_variant(
    _admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let variant = service.restore_variant(*path).await?;
    Ok(HttpResponse::Ok().json(variant))
}

/// List soft-deleted product variants for a product (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/products/{product_id}/variants/deleted",
    params(("product_id" = i64, Path, description = "Product ID")),
    responses(
        (status = 200, description = "List of deleted product variants"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_variants(
    _admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let variants = service.list_deleted_variants(*path).await?;
    Ok(HttpResponse::Ok().json(variants))
}

/// Permanently delete a product variant (admin only, after soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/variants/{id}/destroy",
    params(("id" = i64, Path, description = "Variant ID")),
    responses(
        (status = 204, description = "Product variant permanently deleted"),
        (status = 404, description = "Variant not found"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn destroy_variant(
    _admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    service.destroy_variant(*path).await?;
    Ok(HttpResponse::NoContent().finish())
}
