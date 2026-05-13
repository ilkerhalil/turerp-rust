//! E-Archive API endpoints (v1)
//!
//! REST endpoints for Turkish e-Arşiv Fatura and E-Serbest Meslek Makbuzu
//! integration with GİB (Gelir Idaresi Baskanligi).

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::common::pagination::PaginationParams;
use crate::domain::earchive::model::{EarchiveResponse, EarchiveStatus, GenerateEarchiveRequest};
use crate::domain::earchive::service::EarchiveService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Request body for listing E-Archive documents with optional status filter
#[derive(Debug, Deserialize, ToSchema)]
pub struct ListEarchiveQuery {
    pub status: Option<String>,
}

/// Generate an E-Archive document from an invoice (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/earchive/generate", tag = "E-Archive",
    request_body = GenerateEarchiveRequest,
    responses((status = 201, description = "E-Archive document generated", body = EarchiveResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn generate_earchive(
    admin_user: AdminUser,
    earchive_service: web::Data<EarchiveService>,
    payload: web::Json<GenerateEarchiveRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    match earchive_service
        .generate_earchive(admin_user.0.tenant_id, req.invoice_id, req.document_type)
        .await
    {
        Ok(doc) => Ok(HttpResponse::Created().json(EarchiveResponse::from(doc))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List E-Archive documents (paginated, optional status filter)
#[utoipa::path(
    get, path = "/api/v1/earchive", tag = "E-Archive",
    params(
        PaginationParams,
        ("status" = Option<String>, Query, description = "Filter by status (Draft, Generated, Signed, Sent, Accepted, Rejected, Cancelled)"),
    ),
    responses((status = 200, description = "List of E-Archive documents")),
    security(("bearer_auth" = []))
)]
pub async fn list_earchives(
    auth_user: AuthUser,
    earchive_service: web::Data<EarchiveService>,
    pagination: web::Query<PaginationParams>,
    status: web::Query<Option<String>>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let status_filter = status
        .into_inner()
        .and_then(|s| s.parse::<EarchiveStatus>().ok());
    match earchive_service
        .list_documents(
            auth_user.0.tenant_id,
            status_filter,
            pagination.page,
            pagination.per_page,
        )
        .await
    {
        Ok(result) => {
            let mapped = result.map(EarchiveResponse::from);
            Ok(HttpResponse::Ok().json(mapped))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get an E-Archive document by ID
#[utoipa::path(
    get, path = "/api/v1/earchive/{id}", tag = "E-Archive",
    params(("id" = i64, Path, description = "E-Archive document ID")),
    responses((status = 200, description = "E-Archive document found", body = EarchiveResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_earchive(
    auth_user: AuthUser,
    earchive_service: web::Data<EarchiveService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match earchive_service
        .get_document(auth_user.0.tenant_id, id)
        .await
    {
        Ok(doc) => Ok(HttpResponse::Ok().json(EarchiveResponse::from(doc))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Sign an E-Archive document (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/earchive/{id}/sign", tag = "E-Archive",
    params(("id" = i64, Path, description = "E-Archive document ID")),
    responses((status = 200, description = "E-Archive document signed", body = EarchiveResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn sign_earchive(
    admin_user: AdminUser,
    earchive_service: web::Data<EarchiveService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match earchive_service
        .sign_document(admin_user.0.tenant_id, id)
        .await
    {
        Ok(doc) => Ok(HttpResponse::Ok().json(EarchiveResponse::from(doc))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Send an E-Archive document to GİB (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/earchive/{id}/send", tag = "E-Archive",
    params(("id" = i64, Path, description = "E-Archive document ID")),
    responses((status = 200, description = "E-Archive document sent to GİB", body = EarchiveResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn send_earchive(
    admin_user: AdminUser,
    earchive_service: web::Data<EarchiveService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match earchive_service
        .send_to_gib(admin_user.0.tenant_id, id)
        .await
    {
        Ok(doc) => Ok(HttpResponse::Ok().json(EarchiveResponse::from(doc))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Cancel an E-Archive document (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/earchive/{id}/cancel", tag = "E-Archive",
    params(("id" = i64, Path, description = "E-Archive document ID")),
    responses((status = 200, description = "E-Archive document cancelled", body = EarchiveResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn cancel_earchive(
    admin_user: AdminUser,
    earchive_service: web::Data<EarchiveService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match earchive_service
        .cancel_document(admin_user.0.tenant_id, id)
        .await
    {
        Ok(doc) => Ok(HttpResponse::Ok().json(EarchiveResponse::from(doc))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure E-Archive routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/v1/earchive/generate").route(web::post().to(generate_earchive)))
        .service(web::resource("/v1/earchive").route(web::get().to(list_earchives)))
        .service(web::resource("/v1/earchive/{id}").route(web::get().to(get_earchive)))
        .service(web::resource("/v1/earchive/{id}/sign").route(web::post().to(sign_earchive)))
        .service(web::resource("/v1/earchive/{id}/send").route(web::post().to(send_earchive)))
        .service(web::resource("/v1/earchive/{id}/cancel").route(web::post().to(cancel_earchive)));
}
