//! Quality control handlers

use actix_web::{web, HttpResponse};

use crate::common::MessageResponse;
use crate::domain::quality_control::model::{
    CreateInspection, CreateNonConformanceReport, UpdateInspection, UpdateNonConformanceReport,
};
use crate::domain::quality_control::QualityControlService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

// --- Inspections ---

/// Create inspection (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/inspections", tag = "Manufacturing",
    request_body = CreateInspection,
    responses((status = 201, description = "Inspection created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_inspection(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    payload: web::Json<CreateInspection>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    // Force the auth-derived tenant onto the body so a tenant admin cannot
    // create an inspection attributed to another tenant via a client-supplied
    // `tenant_id` field.
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        qc_service.create_inspection(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get all inspections for tenant
#[utoipa::path(
    get, path = "/api/v1/manufacturing/inspections", tag = "Manufacturing",
    responses((status = 200, description = "List of inspections")),
    security(("bearer_auth" = []))
)]
pub async fn get_inspections(
    auth_user: AuthUser,
    qc_service: web::Data<QualityControlService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        qc_service.get_inspections_by_tenant(auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get inspection by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/inspections/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    responses((status = 200, description = "Inspection found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_inspection(
    auth_user: AuthUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        qc_service.get_inspection(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Update inspection (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/inspections/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    request_body = UpdateInspection,
    responses((status = 200, description = "Inspection updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_inspection(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateInspection>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        qc_service.update_inspection(*path, admin_user.0.tenant_id, payload.into_inner()),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Delete inspection (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/inspections/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    responses((status = 200, description = "Inspection deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_inspection(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match qc_service
        .delete_inspection(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "manufacturing.inspection.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted inspection (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/inspections/{id}/restore", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    responses((status = 200, description = "Inspection restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_inspection(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let inspection = qc_service
        .restore_inspection(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(inspection))
}

/// List soft-deleted inspections (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/manufacturing/inspections/deleted", tag = "Manufacturing",
    responses((status = 200, description = "List of deleted inspections")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_inspections(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
) -> ApiResult<HttpResponse> {
    let deleted = qc_service
        .list_deleted_inspections(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// Permanently destroy an inspection (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/inspections/{id}/destroy", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    responses((status = 204, description = "Inspection permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_inspection(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    qc_service
        .destroy_inspection(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

// --- NCR ---

/// Create NCR (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/ncrs", tag = "Manufacturing",
    request_body = CreateNonConformanceReport,
    responses((status = 201, description = "NCR created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_ncr(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    payload: web::Json<CreateNonConformanceReport>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    // Force the auth-derived tenant onto the body so a tenant admin cannot
    // create an NCR attributed to another tenant via a client-supplied
    // `tenant_id` field.
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        qc_service.create_ncr(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get all NCRs for tenant
#[utoipa::path(
    get, path = "/api/v1/manufacturing/ncrs", tag = "Manufacturing",
    responses((status = 200, description = "List of NCRs")),
    security(("bearer_auth" = []))
)]
pub async fn get_ncrs(
    auth_user: AuthUser,
    qc_service: web::Data<QualityControlService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        qc_service.get_ncrs_by_tenant(auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get NCR by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/ncrs/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    responses((status = 200, description = "NCR found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_ncr(
    auth_user: AuthUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        qc_service.get_ncr(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Update NCR (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/ncrs/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    request_body = UpdateNonConformanceReport,
    responses((status = 200, description = "NCR updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_ncr(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateNonConformanceReport>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        qc_service.update_ncr(*path, admin_user.0.tenant_id, payload.into_inner()),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Delete NCR (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/ncrs/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    responses((status = 200, description = "NCR deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_ncr(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match qc_service
        .delete_ncr(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "manufacturing.ncr.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted NCR (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/ncrs/{id}/restore", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    responses((status = 200, description = "NCR restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_ncr(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let ncr = qc_service
        .restore_ncr(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(ncr))
}

/// List soft-deleted NCRs (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/manufacturing/ncrs/deleted", tag = "Manufacturing",
    responses((status = 200, description = "List of deleted NCRs")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_ncrs(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
) -> ApiResult<HttpResponse> {
    let deleted = qc_service.list_deleted_ncrs(admin_user.0.tenant_id).await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// Permanently destroy an NCR (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/ncrs/{id}/destroy", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    responses((status = 204, description = "NCR permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_ncr(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    qc_service
        .destroy_ncr(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
