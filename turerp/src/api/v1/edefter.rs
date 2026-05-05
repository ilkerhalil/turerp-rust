//! e-Defter API endpoints (v1)
//!
//! REST endpoints for Turkish electronic ledger (e-Defter) integration
//! with GIB (Gelir Idaresi Baskanligi).

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::common::pagination::PaginationParams;
use crate::domain::edefter::model::{
    CreateLedgerPeriod, LedgerPeriodResponse, LedgerType, YevmiyeEntry,
};
use crate::domain::edefter::service::EDefterService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Response for XML generation endpoints
#[derive(Debug, serde::Serialize, ToSchema)]
pub struct XmlResponse {
    pub xml: String,
}

/// Create a new ledger period (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/edefter/periods", tag = "e-Defter",
    request_body = CreateLedgerPeriod,
    responses((status = 201, description = "Ledger period created", body = LedgerPeriodResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_period(
    admin_user: AdminUser,
    edefter_service: web::Data<EDefterService>,
    payload: web::Json<CreateLedgerPeriod>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match edefter_service
        .create_period(create, admin_user.0.tenant_id)
        .await
    {
        Ok(period) => Ok(HttpResponse::Created().json(LedgerPeriodResponse::from(period))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List ledger periods (paginated, optional year/type filters)
#[utoipa::path(
    get, path = "/api/v1/edefter/periods", tag = "e-Defter",
    params(
        PaginationParams,
        ("year" = Option<i32>, Query, description = "Filter by year"),
        ("period_type" = Option<String>, Query, description = "Filter by type (YevmiyeDefteri, BuyukDefter, KebirDefter)"),
    ),
    responses((status = 200, description = "List of ledger periods")),
    security(("bearer_auth" = []))
)]
pub async fn list_periods(
    auth_user: AuthUser,
    edefter_service: web::Data<EDefterService>,
    pagination: web::Query<PaginationParams>,
    year: web::Query<Option<i32>>,
    period_type: web::Query<Option<String>>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let type_filter = period_type
        .into_inner()
        .and_then(|s| s.parse::<LedgerType>().ok());
    match edefter_service
        .list_periods(
            auth_user.0.tenant_id,
            year.into_inner(),
            type_filter,
            pagination.into_inner(),
        )
        .await
    {
        Ok(result) => {
            let mapped = result.map(LedgerPeriodResponse::from);
            Ok(HttpResponse::Ok().json(mapped))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a ledger period by ID
#[utoipa::path(
    get, path = "/api/v1/edefter/periods/{id}", tag = "e-Defter",
    params(("id" = i64, Path, description = "Ledger period ID")),
    responses((status = 200, description = "Ledger period found", body = LedgerPeriodResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_period(
    auth_user: AuthUser,
    edefter_service: web::Data<EDefterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match edefter_service.get_period(id, auth_user.0.tenant_id).await {
        Ok(period) => Ok(HttpResponse::Ok().json(LedgerPeriodResponse::from(period))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Request body for populating an e-Defter period from accounting entries
#[derive(Debug, Deserialize, ToSchema)]
pub struct PopulatePeriodRequest {
    /// Yevmiye entries to populate the period with
    pub entries: Vec<YevmiyeEntry>,
}

/// Populate a ledger period from accounting entries (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/edefter/periods/{id}/populate", tag = "e-Defter",
    params(("id" = i64, Path, description = "Ledger period ID")),
    request_body = PopulatePeriodRequest,
    responses((status = 200, description = "Period populated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn populate_period(
    admin_user: AdminUser,
    edefter_service: web::Data<EDefterService>,
    path: web::Path<i64>,
    payload: web::Json<PopulatePeriodRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let entries = payload.into_inner().entries;
    match edefter_service
        .populate_from_accounting(id, admin_user.0.tenant_id, entries)
        .await
    {
        Ok(populated) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "Period populated",
            "entries_count": populated.len(),
        }))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Validate balance for a ledger period (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/edefter/periods/{id}/validate", tag = "e-Defter",
    params(("id" = i64, Path, description = "Ledger period ID")),
    responses((status = 200, description = "Balance check result", body = BalanceCheckResult), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn validate_period(
    admin_user: AdminUser,
    edefter_service: web::Data<EDefterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match edefter_service
        .validate_balance(id, admin_user.0.tenant_id)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Generate Yevmiye defteri XML (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/edefter/periods/{id}/yevmiye-xml", tag = "e-Defter",
    params(("id" = i64, Path, description = "Ledger period ID")),
    responses((status = 200, description = "Yevmiye XML generated", body = XmlResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn generate_yevmiye(
    admin_user: AdminUser,
    edefter_service: web::Data<EDefterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match edefter_service
        .generate_yevmiye_xml(id, admin_user.0.tenant_id)
        .await
    {
        Ok(xml) => Ok(HttpResponse::Ok().content_type("application/xml").body(xml)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Generate Buyuk defter XML (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/edefter/periods/{id}/buyuk-defter-xml", tag = "e-Defter",
    params(("id" = i64, Path, description = "Ledger period ID")),
    responses((status = 200, description = "Buyuk defter XML generated", body = XmlResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn generate_buyuk_defter(
    admin_user: AdminUser,
    edefter_service: web::Data<EDefterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match edefter_service
        .generate_buyuk_defter_xml(id, admin_user.0.tenant_id)
        .await
    {
        Ok(xml) => Ok(HttpResponse::Ok().content_type("application/xml").body(xml)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Sign a berat for a ledger period (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/edefter/periods/{id}/sign", tag = "e-Defter",
    params(("id" = i64, Path, description = "Ledger period ID")),
    responses((status = 200, description = "Berat signed", body = BeratInfo), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn sign_berat(
    admin_user: AdminUser,
    edefter_service: web::Data<EDefterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match edefter_service.sign_berat(id, admin_user.0.tenant_id).await {
        Ok(berat) => Ok(HttpResponse::Ok().json(berat)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Send a signed ledger period to saklayici (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/edefter/periods/{id}/send", tag = "e-Defter",
    params(("id" = i64, Path, description = "Ledger period ID")),
    responses((status = 200, description = "Period sent to saklayici", body = LedgerPeriodResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn send_to_saklayici(
    admin_user: AdminUser,
    edefter_service: web::Data<EDefterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match edefter_service
        .send_to_saklayici(id, admin_user.0.tenant_id)
        .await
    {
        Ok(period) => Ok(HttpResponse::Ok().json(LedgerPeriodResponse::from(period))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Check the status of a ledger period
#[utoipa::path(
    get, path = "/api/v1/edefter/periods/{id}/status", tag = "e-Defter",
    params(("id" = i64, Path, description = "Ledger period ID")),
    responses((status = 200, description = "Period status", body = EDefterStatus), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn check_status(
    auth_user: AuthUser,
    edefter_service: web::Data<EDefterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match edefter_service
        .check_status(id, auth_user.0.tenant_id)
        .await
    {
        Ok(status) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({"status": status.to_string()})))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure e-Defter routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/edefter/periods")
            .route(web::get().to(list_periods))
            .route(web::post().to(create_period)),
    )
    .service(web::resource("/v1/edefter/periods/{id}").route(web::get().to(get_period)))
    .service(
        web::resource("/v1/edefter/periods/{id}/populate").route(web::post().to(populate_period)),
    )
    .service(
        web::resource("/v1/edefter/periods/{id}/validate").route(web::post().to(validate_period)),
    )
    .service(
        web::resource("/v1/edefter/periods/{id}/yevmiye-xml")
            .route(web::post().to(generate_yevmiye)),
    )
    .service(
        web::resource("/v1/edefter/periods/{id}/buyuk-defter-xml")
            .route(web::post().to(generate_buyuk_defter)),
    )
    .service(web::resource("/v1/edefter/periods/{id}/sign").route(web::post().to(sign_berat)))
    .service(
        web::resource("/v1/edefter/periods/{id}/send").route(web::post().to(send_to_saklayici)),
    )
    .service(web::resource("/v1/edefter/periods/{id}/status").route(web::get().to(check_status)));
}
