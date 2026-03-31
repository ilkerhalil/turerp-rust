//! Product Variants API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::domain::product::{CreateProductVariant, ProductService, UpdateProductVariant};
use crate::error::ApiResult;
use crate::middleware::AuthUser;

/// Create product variant endpoint (requires authentication)
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
        (status = 404, description = "Product not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_variant(
    _auth_user: AuthUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    payload: web::Json<CreateProductVariant>,
) -> ApiResult<HttpResponse> {
    let product_id = path.into_inner();
    let mut create = payload.into_inner();
    create.product_id = product_id; // Ensure product_id matches path

    let variant = service.create_variant(create).await?;
    Ok(HttpResponse::Created().json(variant))
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
) -> ApiResult<HttpResponse> {
    let product_id = path.into_inner();
    let variants = service.get_variants_by_product(product_id).await?;
    Ok(HttpResponse::Ok().json(variants))
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
) -> ApiResult<HttpResponse> {
    let variant = service.get_variant(*path).await?;
    Ok(HttpResponse::Ok().json(variant))
}

/// Update product variant endpoint (requires authentication)
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
        (status = 404, description = "Variant not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_variant(
    _auth_user: AuthUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateProductVariant>,
) -> ApiResult<HttpResponse> {
    let variant = service.update_variant(*path, payload.into_inner()).await?;
    Ok(HttpResponse::Ok().json(variant))
}

/// Delete product variant endpoint (requires authentication)
#[utoipa::path(
    delete,
    path = "/api/v1/variants/{id}",
    tag = "Products",
    params(
        ("id" = i64, Path, description = "Variant ID")
    ),
    responses(
        (status = 204, description = "Product variant deleted"),
        (status = 401, description = "Not authenticated - missing or invalid JWT token"),
        (status = 404, description = "Variant not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_variant(
    _auth_user: AuthUser,
    service: web::Data<ProductService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    service.delete_variant(*path).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Configure product variant routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
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
