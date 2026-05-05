//! Product Variants API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::product::{
    CategoryResponse, CreateProductVariant, ProductResponse, ProductService, UnitResponse,
    UpdateProductVariant,
};
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

// --- Units ---

/// Soft delete a unit (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/units/{id}",
    tag = "Products",
    params(("id" = i64, Path, description = "Unit ID")),
    responses(
        (status = 200, description = "Unit soft deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Unit not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_unit(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .soft_delete_unit(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "unit.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted unit (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/units/{id}/restore",
    params(("id" = i64, Path, description = "Unit ID")),
    responses(
        (status = 200, description = "Unit restored", body = UnitResponse),
        (status = 404, description = "Unit not found or not deleted"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn restore_unit(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let unit = service.restore_unit(*path, admin_user.0.tenant_id).await?;
    let response: UnitResponse = unit.into();
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted units (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/units/deleted",
    responses(
        (status = 200, description = "List of deleted units"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_units(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
) -> ApiResult<HttpResponse> {
    let units: Vec<_> = service
        .list_deleted_units(admin_user.0.tenant_id)
        .await?
        .into_iter()
        .map(UnitResponse::from)
        .collect();
    Ok(HttpResponse::Ok().json(units))
}

/// Permanently delete a unit (admin only, after soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/units/{id}/destroy",
    params(("id" = i64, Path, description = "Unit ID")),
    responses(
        (status = 204, description = "Unit permanently deleted"),
        (status = 404, description = "Unit not found"),
    ),
    tag = "Products",
    security(("bearer_auth" = []))
)]
pub async fn destroy_unit(
    admin_user: AdminUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    service.destroy_unit(*path, admin_user.0.tenant_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

// --- Product Variants ---

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

/// Configure product variant routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/v1/products").route(web::get().to(get_products)))
        .service(web::resource("/v1/products/deleted").route(web::get().to(list_deleted_products)))
        .service(web::resource("/v1/products/{id}").route(web::delete().to(delete_product)))
        .service(web::resource("/v1/products/{id}/restore").route(web::put().to(restore_product)))
        .service(
            web::resource("/v1/products/{id}/destroy").route(web::delete().to(destroy_product)),
        )
        .service(web::resource("/v1/categories").route(web::get().to(get_categories)))
        .service(
            web::resource("/v1/categories/deleted").route(web::get().to(list_deleted_categories)),
        )
        .service(web::resource("/v1/categories/{id}").route(web::delete().to(delete_category)))
        .service(
            web::resource("/v1/categories/{id}/restore").route(web::put().to(restore_category)),
        )
        .service(
            web::resource("/v1/categories/{id}/destroy").route(web::delete().to(destroy_category)),
        )
        .service(web::resource("/v1/units/deleted").route(web::get().to(list_deleted_units)))
        .service(web::resource("/v1/units/{id}").route(web::delete().to(delete_unit)))
        .service(web::resource("/v1/units/{id}/restore").route(web::put().to(restore_unit)))
        .service(web::resource("/v1/units/{id}/destroy").route(web::delete().to(destroy_unit)))
        .service(
            web::resource("/v1/products/{product_id}/variants")
                .route(web::get().to(get_variants_by_product))
                .route(web::post().to(create_variant)),
        )
        .service(
            web::resource("/v1/products/{product_id}/variants/deleted")
                .route(web::get().to(list_deleted_variants)),
        )
        .service(
            web::resource("/v1/variants/{id}")
                .route(web::get().to(get_variant))
                .route(web::put().to(update_variant))
                .route(web::delete().to(delete_variant)),
        )
        .service(web::resource("/v1/variants/{id}/restore").route(web::put().to(restore_variant)))
        .service(
            web::resource("/v1/variants/{id}/destroy").route(web::delete().to(destroy_variant)),
        );
}
