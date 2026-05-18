//! BOM handlers

use actix_web::{web, HttpResponse};

use crate::common::MessageResponse;
use crate::domain::manufacturing::model::{CreateBillOfMaterials, CreateBillOfMaterialsLine};
use crate::domain::manufacturing::service::ManufacturingService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

/// Create BOM (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/boms", tag = "Manufacturing",
    request_body = CreateBillOfMaterials,
    responses((status = 201, description = "BOM created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_bom(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateBillOfMaterials>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        mfg_service.create_bom(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get BOM by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/boms/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "BOM ID")),
    responses((status = 200, description = "BOM found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_bom(
    auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.get_bom(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get BOMs by product
#[utoipa::path(
    get, path = "/api/v1/manufacturing/boms/product/{product_id}", tag = "Manufacturing",
    params(("product_id" = i64, Path, description = "Product ID")),
    responses((status = 200, description = "BOMs for product")),
    security(("bearer_auth" = []))
)]
pub async fn get_boms_by_product(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.get_boms_by_product(*path),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Add BOM line (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/boms/lines", tag = "Manufacturing",
    request_body = CreateBillOfMaterialsLine,
    responses((status = 201, description = "BOM line added"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn add_bom_line(
    _admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateBillOfMaterialsLine>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.add_bom_line(payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get BOM lines
#[utoipa::path(
    get, path = "/api/v1/manufacturing/boms/{bom_id}/lines", tag = "Manufacturing",
    params(("bom_id" = i64, Path, description = "BOM ID")),
    responses((status = 200, description = "BOM lines")),
    security(("bearer_auth" = []))
)]
pub async fn get_bom_lines(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        mfg_service.get_bom_lines(*path),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Soft-delete a BOM (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/boms/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "BOM ID")),
    responses((status = 200, description = "BOM soft-deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_bom(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service
        .soft_delete_bom(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "BOM soft-deleted".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted BOM (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/boms/{id}/restore", tag = "Manufacturing",
    params(("id" = i64, Path, description = "BOM ID")),
    responses((status = 200, description = "BOM restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_bom(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let bom = mfg_service
        .restore_bom(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(bom))
}

/// List soft-deleted BOMs (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/manufacturing/boms/deleted", tag = "Manufacturing",
    responses((status = 200, description = "List of deleted BOMs")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_boms(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
) -> ApiResult<HttpResponse> {
    let deleted = mfg_service
        .list_deleted_boms(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// Permanently destroy a BOM (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/boms/{id}/destroy", tag = "Manufacturing",
    params(("id" = i64, Path, description = "BOM ID")),
    responses((status = 204, description = "BOM permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_bom(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    mfg_service
        .destroy_bom(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
