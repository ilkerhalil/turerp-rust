//! POS (Point of Sale) API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::pos::model::{
    CreatePosSale, CreatePosTerminal, CreateZReport, UpdatePosTerminal,
};
use crate::domain::pos::service::PosService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

// --- POS Terminals ---

/// Create a POS terminal (admin only)
#[utoipa::path(
    post, path = "/api/v1/pos/terminals", tag = "POS",
    request_body = CreatePosTerminal,
    responses((status = 201, description = "POS terminal created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_pos_terminal(
    admin_user: AdminUser,
    pos_service: web::Data<PosService>,
    payload: web::Json<CreatePosTerminal>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        pos_service.create_terminal(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// List POS terminals (paginated)
#[utoipa::path(
    get, path = "/api/v1/pos/terminals", tag = "POS",
    params(PaginationParams),
    responses((status = 200, description = "Paginated list of POS terminals")),
    security(("bearer_auth" = []))
)]
pub async fn get_pos_terminals(
    auth_user: AuthUser,
    pos_service: web::Data<PosService>,
    pagination: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    json_resp!(
        pos_service.list_terminals(auth_user.0.tenant_id, pagination.page, pagination.per_page),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get POS terminal by ID
#[utoipa::path(
    get, path = "/api/v1/pos/terminals/{id}", tag = "POS",
    params(("id" = i64, Path, description = "Terminal ID")),
    responses((status = 200, description = "Terminal found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_pos_terminal(
    auth_user: AuthUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    json_resp!(
        pos_service.get_terminal(id, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Update POS terminal
#[utoipa::path(
    put, path = "/api/v1/pos/terminals/{id}", tag = "POS",
    params(("id" = i64, Path, description = "Terminal ID")),
    request_body = UpdatePosTerminal,
    responses((status = 200, description = "Terminal updated"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn update_pos_terminal(
    admin_user: AdminUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    payload: web::Json<UpdatePosTerminal>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    json_resp!(
        pos_service.update_terminal(id, admin_user.0.tenant_id, payload.into_inner()),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Delete (soft-delete) a POS terminal
#[utoipa::path(
    delete, path = "/api/v1/pos/terminals/{id}", tag = "POS",
    params(("id" = i64, Path, description = "Terminal ID")),
    responses((status = 204, description = "Terminal deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn delete_pos_terminal(
    admin_user: AdminUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let deleted_by = admin_user.0.sub.parse::<i64>().unwrap_or(0);
    json_resp!(
        pos_service.delete_terminal(id, admin_user.0.tenant_id, deleted_by),
        HttpResponse::NoContent,
        i18n,
        locale.as_str()
    )
}

/// Sync terminal (update last_sync_at)
#[utoipa::path(
    post, path = "/api/v1/pos/terminals/{id}/sync", tag = "POS",
    params(("id" = i64, Path, description = "Terminal ID")),
    responses((status = 200, description = "Terminal synced"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn sync_pos_terminal(
    auth_user: AuthUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    json_resp!(
        pos_service.sync_terminal(id, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// List sales by terminal
#[utoipa::path(
    get, path = "/api/v1/pos/terminals/{id}/sales", tag = "POS",
    params(("id" = i64, Path, description = "Terminal ID")),
    responses((status = 200, description = "List of sales for terminal")),
    security(("bearer_auth" = []))
)]
pub async fn get_terminal_sales(
    auth_user: AuthUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    json_resp!(
        pos_service.list_sales_by_terminal(id, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

// --- POS Sales ---

/// Record a POS sale
#[utoipa::path(
    post, path = "/api/v1/pos/sales", tag = "POS",
    request_body = CreatePosSale,
    responses((status = 201, description = "POS sale recorded"), (status = 400, description = "Bad request")),
    security(("bearer_auth" = []))
)]
pub async fn create_pos_sale(
    admin_user: AdminUser,
    pos_service: web::Data<PosService>,
    payload: web::Json<CreatePosSale>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        pos_service.create_sale(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// List POS sales (paginated)
#[utoipa::path(
    get, path = "/api/v1/pos/sales", tag = "POS",
    params(PaginationParams),
    responses((status = 200, description = "Paginated list of POS sales")),
    security(("bearer_auth" = []))
)]
pub async fn get_pos_sales(
    auth_user: AuthUser,
    pos_service: web::Data<PosService>,
    pagination: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    json_resp!(
        pos_service.list_sales(auth_user.0.tenant_id, pagination.page, pagination.per_page),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get POS sale by ID
#[utoipa::path(
    get, path = "/api/v1/pos/sales/{id}", tag = "POS",
    params(("id" = i64, Path, description = "Sale ID")),
    responses((status = 200, description = "Sale found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_pos_sale(
    auth_user: AuthUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    json_resp!(
        pos_service.get_sale(id, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Delete (soft-delete) a POS sale
#[utoipa::path(
    delete, path = "/api/v1/pos/sales/{id}", tag = "POS",
    params(("id" = i64, Path, description = "Sale ID")),
    responses((status = 204, description = "Sale deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn delete_pos_sale(
    admin_user: AdminUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let deleted_by = admin_user.0.sub.parse::<i64>().unwrap_or(0);
    json_resp!(
        pos_service.delete_sale(id, admin_user.0.tenant_id, deleted_by),
        HttpResponse::NoContent,
        i18n,
        locale.as_str()
    )
}

// --- Z-Reports ---

/// Create (open) a Z-report
#[utoipa::path(
    post, path = "/api/v1/pos/z-reports", tag = "POS",
    request_body = CreateZReport,
    responses((status = 201, description = "Z-report created"), (status = 409, description = "Conflict")),
    security(("bearer_auth" = []))
)]
pub async fn create_z_report(
    admin_user: AdminUser,
    pos_service: web::Data<PosService>,
    payload: web::Json<CreateZReport>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        pos_service.create_z_report(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// List Z-reports (paginated)
#[utoipa::path(
    get, path = "/api/v1/pos/z-reports", tag = "POS",
    params(PaginationParams),
    responses((status = 200, description = "Paginated list of Z-reports")),
    security(("bearer_auth" = []))
)]
pub async fn get_z_reports(
    auth_user: AuthUser,
    pos_service: web::Data<PosService>,
    pagination: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    json_resp!(
        pos_service.list_z_reports(auth_user.0.tenant_id, pagination.page, pagination.per_page),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get Z-report by ID
#[utoipa::path(
    get, path = "/api/v1/pos/z-reports/{id}", tag = "POS",
    params(("id" = i64, Path, description = "Z-report ID")),
    responses((status = 200, description = "Z-report found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_z_report(
    auth_user: AuthUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    json_resp!(
        pos_service.get_z_report(id, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Close a Z-report (compute totals from assigned sales)
#[utoipa::path(
    post, path = "/api/v1/pos/z-reports/{id}/close", tag = "POS",
    params(("id" = i64, Path, description = "Z-report ID")),
    responses((status = 200, description = "Z-report closed"), (status = 400, description = "Bad request")),
    security(("bearer_auth" = []))
)]
pub async fn close_z_report(
    admin_user: AdminUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    json_resp!(
        pos_service.close_z_report(id, admin_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Reconcile a closed Z-report
#[utoipa::path(
    post, path = "/api/v1/pos/z-reports/{id}/reconcile", tag = "POS",
    params(("id" = i64, Path, description = "Z-report ID")),
    responses((status = 200, description = "Z-report reconciled"), (status = 400, description = "Bad request")),
    security(("bearer_auth" = []))
)]
pub async fn reconcile_z_report(
    admin_user: AdminUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    json_resp!(
        pos_service.reconcile_z_report(id, admin_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// List Z-reports by terminal
#[utoipa::path(
    get, path = "/api/v1/pos/terminals/{id}/z-reports", tag = "POS",
    params(("id" = i64, Path, description = "Terminal ID")),
    responses((status = 200, description = "List of Z-reports for terminal")),
    security(("bearer_auth" = []))
)]
pub async fn get_terminal_z_reports(
    auth_user: AuthUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    json_resp!(
        pos_service.list_z_reports_by_terminal(id, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

// --- Sync Queue ---

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct EnqueueSyncRequest {
    pub terminal_id: i64,
    pub payload: String,
}

/// Enqueue an offline sync item
#[utoipa::path(
    post, path = "/api/v1/pos/sync", tag = "POS",
    request_body = EnqueueSyncRequest,
    responses((status = 201, description = "Sync item enqueued")),
    security(("bearer_auth" = []))
)]
pub async fn enqueue_sync(
    admin_user: AdminUser,
    pos_service: web::Data<PosService>,
    payload: web::Json<EnqueueSyncRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    match pos_service
        .enqueue_sync(admin_user.0.tenant_id, req.terminal_id, req.payload)
        .await
    {
        Ok(id) => Ok(HttpResponse::Created().json(serde_json::json!({ "id": id }))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get pending sync count for a terminal
#[utoipa::path(
    get, path = "/api/v1/pos/sync/pending/{terminal_id}", tag = "POS",
    params(("terminal_id" = i64, Path, description = "Terminal ID")),
    responses((status = 200, description = "Pending sync count")),
    security(("bearer_auth" = []))
)]
pub async fn pending_sync_count(
    auth_user: AuthUser,
    pos_service: web::Data<PosService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let terminal_id = path.into_inner();
    json_resp!(
        pos_service.pending_sync_count(terminal_id, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Configure POS routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/pos/terminals")
            .route(web::get().to(get_pos_terminals))
            .route(web::post().to(create_pos_terminal)),
    )
    .service(
        web::resource("/v1/pos/terminals/{id}")
            .route(web::get().to(get_pos_terminal))
            .route(web::put().to(update_pos_terminal))
            .route(web::delete().to(delete_pos_terminal)),
    )
    .service(web::resource("/v1/pos/terminals/{id}/sync").route(web::post().to(sync_pos_terminal)))
    .service(web::resource("/v1/pos/terminals/{id}/sales").route(web::get().to(get_terminal_sales)))
    .service(
        web::resource("/v1/pos/terminals/{id}/z-reports")
            .route(web::get().to(get_terminal_z_reports)),
    )
    .service(
        web::resource("/v1/pos/sales")
            .route(web::get().to(get_pos_sales))
            .route(web::post().to(create_pos_sale)),
    )
    .service(
        web::resource("/v1/pos/sales/{id}")
            .route(web::get().to(get_pos_sale))
            .route(web::delete().to(delete_pos_sale)),
    )
    .service(
        web::resource("/v1/pos/z-reports")
            .route(web::get().to(get_z_reports))
            .route(web::post().to(create_z_report)),
    )
    .service(web::resource("/v1/pos/z-reports/{id}").route(web::get().to(get_z_report)))
    .service(web::resource("/v1/pos/z-reports/{id}/close").route(web::post().to(close_z_report)))
    .service(
        web::resource("/v1/pos/z-reports/{id}/reconcile").route(web::post().to(reconcile_z_report)),
    )
    .service(web::resource("/v1/pos/sync").route(web::post().to(enqueue_sync)))
    .service(
        web::resource("/v1/pos/sync/pending/{terminal_id}")
            .route(web::get().to(pending_sync_count)),
    );
}
