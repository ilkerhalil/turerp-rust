//! e-Fatura API endpoints (v1)
//!
//! REST endpoints for Turkish electronic invoicing (e-Fatura) integration
//! with GIB (Gelir Idaresi Baskanligi).

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::common::pagination::PaginationParams;
use crate::domain::efatura::model::{CreateEFatura, EFaturaResponse, EFaturaStatus};
use crate::domain::efatura::service::EFaturaService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Request body for cancelling an e-Fatura
#[derive(Debug, Deserialize, ToSchema)]
pub struct CancelEFaturaRequest {
    pub reason: String,
}

/// Create an e-Fatura from an invoice (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/efatura", tag = "e-Fatura",
    request_body = CreateEFatura,
    responses((status = 201, description = "e-Fatura created", body = EFaturaResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_efatura(
    admin_user: AdminUser,
    efatura_service: web::Data<EFaturaService>,
    payload: web::Json<CreateEFatura>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match efatura_service
        .create_from_invoice(create.invoice_id, create.profile_id, admin_user.0.tenant_id)
        .await
    {
        Ok(fatura) => Ok(HttpResponse::Created().json(EFaturaResponse::from(fatura))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List e-Fatura documents (paginated, optional status filter)
#[utoipa::path(
    get, path = "/api/v1/efatura", tag = "e-Fatura",
    params(
        PaginationParams,
        ("status" = Option<String>, Query, description = "Filter by status (Draft, Signed, Sent, Accepted, Rejected, Cancelled, Error)"),
    ),
    responses((status = 200, description = "List of e-Fatura documents")),
    security(("bearer_auth" = []))
)]
pub async fn list_efaturas(
    auth_user: AuthUser,
    efatura_service: web::Data<EFaturaService>,
    pagination: web::Query<PaginationParams>,
    status: web::Query<Option<String>>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let status_filter = status
        .into_inner()
        .and_then(|s| s.parse::<EFaturaStatus>().ok());
    match efatura_service
        .list_efaturas(
            auth_user.0.tenant_id,
            status_filter,
            pagination.into_inner(),
        )
        .await
    {
        Ok(result) => {
            let mapped = result.map(EFaturaResponse::from);
            Ok(HttpResponse::Ok().json(mapped))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get an e-Fatura by ID
#[utoipa::path(
    get, path = "/api/v1/efatura/{id}", tag = "e-Fatura",
    params(("id" = i64, Path, description = "e-Fatura ID")),
    responses((status = 200, description = "e-Fatura found", body = EFaturaResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_efatura(
    auth_user: AuthUser,
    efatura_service: web::Data<EFaturaService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match efatura_service.get_efatura(id, auth_user.0.tenant_id).await {
        Ok(fatura) => Ok(HttpResponse::Ok().json(EFaturaResponse::from(fatura))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get the XML content of an e-Fatura
#[utoipa::path(
    get, path = "/api/v1/efatura/{id}/xml", tag = "e-Fatura",
    params(("id" = i64, Path, description = "e-Fatura ID")),
    responses((status = 200, description = "XML content"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_efatura_xml(
    auth_user: AuthUser,
    efatura_service: web::Data<EFaturaService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match efatura_service.get_xml(id, auth_user.0.tenant_id).await {
        Ok(xml) => Ok(HttpResponse::Ok().content_type("application/xml").body(xml)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Send an e-Fatura to GIB (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/efatura/{id}/send", tag = "e-Fatura",
    params(("id" = i64, Path, description = "e-Fatura ID")),
    responses((status = 200, description = "e-Fatura sent to GIB", body = EFaturaResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn send_efatura(
    admin_user: AdminUser,
    efatura_service: web::Data<EFaturaService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match efatura_service
        .send_to_gib(id, admin_user.0.tenant_id)
        .await
    {
        Ok(fatura) => Ok(HttpResponse::Ok().json(EFaturaResponse::from(fatura))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Cancel an e-Fatura (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/efatura/{id}/cancel", tag = "e-Fatura",
    params(("id" = i64, Path, description = "e-Fatura ID")),
    request_body = CancelEFaturaRequest,
    responses((status = 200, description = "e-Fatura cancelled", body = EFaturaResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn cancel_efatura(
    admin_user: AdminUser,
    efatura_service: web::Data<EFaturaService>,
    path: web::Path<i64>,
    payload: web::Json<CancelEFaturaRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let reason = payload.into_inner().reason;
    match efatura_service
        .cancel_efatura(id, admin_user.0.tenant_id, reason)
        .await
    {
        Ok(fatura) => Ok(HttpResponse::Ok().json(EFaturaResponse::from(fatura))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Check the GIB status of an e-Fatura by UUID
#[utoipa::path(
    get, path = "/api/v1/efatura/status/{uuid}", tag = "e-Fatura",
    params(("uuid" = String, Path, description = "e-Fatura UUID")),
    responses((status = 200, description = "e-Fatura status updated", body = EFaturaResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn check_efatura_status(
    auth_user: AuthUser,
    efatura_service: web::Data<EFaturaService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let uuid = path.into_inner();
    match efatura_service
        .check_status(&uuid, auth_user.0.tenant_id)
        .await
    {
        Ok(fatura) => Ok(HttpResponse::Ok().json(EFaturaResponse::from(fatura))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure e-Fatura routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/efatura")
            .route(web::get().to(list_efaturas))
            .route(web::post().to(create_efatura)),
    )
    .service(web::resource("/v1/efatura/{id}").route(web::get().to(get_efatura)))
    .service(web::resource("/v1/efatura/{id}/xml").route(web::get().to(get_efatura_xml)))
    .service(web::resource("/v1/efatura/{id}/send").route(web::post().to(send_efatura)))
    .service(web::resource("/v1/efatura/{id}/cancel").route(web::post().to(cancel_efatura)))
    .service(web::resource("/v1/efatura/status/{uuid}").route(web::get().to(check_efatura_status)));
}
