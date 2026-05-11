//! Data Archiving API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::common::pagination::{default_page, default_per_page, PaginationParams};
use crate::domain::archive::model::{
    BulkRestoreFailed, BulkRestoreResponse, CreateArchiveJob, CreateArchivePolicy, RestoreRequest,
    UpdateArchivePolicy,
};
use crate::domain::archive::service::ArchiveService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Query parameters for listing archive policies
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListPoliciesQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

impl From<ListPoliciesQuery> for PaginationParams {
    fn from(q: ListPoliciesQuery) -> Self {
        Self {
            page: q.page,
            per_page: q.per_page,
        }
    }
}

/// Query parameters for listing archive jobs
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListJobsQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    pub policy_id: Option<i64>,
}

impl From<ListJobsQuery> for PaginationParams {
    fn from(q: ListJobsQuery) -> Self {
        Self {
            page: q.page,
            per_page: q.per_page,
        }
    }
}

/// Query parameters for listing archive records
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListRecordsQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    pub source_table: Option<String>,
    pub source_id: Option<i64>,
}

impl From<ListRecordsQuery> for PaginationParams {
    fn from(q: ListRecordsQuery) -> Self {
        Self {
            page: q.page,
            per_page: q.per_page,
        }
    }
}

/// Query parameters for getting active policies
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ActivePoliciesQuery {
    pub tenant_id: i64,
}

// ---- Archive Policy Handlers ----

/// Create an archive policy (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/archive/policies", tag = "Archive",
    request_body = CreateArchivePolicy,
    responses((status = 201, description = "Archive policy created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_policy(
    admin_user: AdminUser,
    archive_service: web::Data<ArchiveService>,
    payload: web::Json<CreateArchivePolicy>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match archive_service
        .create_policy(create, admin_user.0.tenant_id)
        .await
    {
        Ok(policy) => Ok(HttpResponse::Created().json(policy)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List archive policies (paginated)
#[utoipa::path(
    get, path = "/api/v1/archive/policies", tag = "Archive",
    params(ListPoliciesQuery),
    responses((status = 200, description = "List of archive policies")),
    security(("bearer_auth" = []))
)]
pub async fn list_policies(
    auth_user: AuthUser,
    archive_service: web::Data<ArchiveService>,
    query: web::Query<ListPoliciesQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let q = query.into_inner();
    match archive_service
        .list_policies(auth_user.0.tenant_id, q.into())
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List active archive policies
#[utoipa::path(
    get, path = "/api/v1/archive/policies/active", tag = "Archive",
    responses((status = 200, description = "List of active archive policies")),
    security(("bearer_auth" = []))
)]
pub async fn list_active_policies(
    auth_user: AuthUser,
    archive_service: web::Data<ArchiveService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match archive_service
        .list_active_policies(auth_user.0.tenant_id)
        .await
    {
        Ok(policies) => Ok(HttpResponse::Ok().json(policies)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get an archive policy by ID
#[utoipa::path(
    get, path = "/api/v1/archive/policies/{id}", tag = "Archive",
    params(("id" = i64, Path, description = "Archive policy ID")),
    responses((status = 200, description = "Archive policy found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_policy(
    auth_user: AuthUser,
    archive_service: web::Data<ArchiveService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match archive_service.get_policy(id, auth_user.0.tenant_id).await {
        Ok(policy) => Ok(HttpResponse::Ok().json(policy)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update an archive policy (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/archive/policies/{id}", tag = "Archive",
    params(("id" = i64, Path, description = "Archive policy ID")),
    request_body = UpdateArchivePolicy,
    responses((status = 200, description = "Archive policy updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn update_policy(
    admin_user: AdminUser,
    archive_service: web::Data<ArchiveService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateArchivePolicy>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let update = payload.into_inner();
    match archive_service
        .update_policy(id, admin_user.0.tenant_id, update)
        .await
    {
        Ok(policy) => Ok(HttpResponse::Ok().json(policy)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete an archive policy (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/archive/policies/{id}", tag = "Archive",
    params(("id" = i64, Path, description = "Archive policy ID")),
    responses((status = 204, description = "Archive policy deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn delete_policy(
    admin_user: AdminUser,
    archive_service: web::Data<ArchiveService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match archive_service
        .delete_policy(id, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// ---- Archive Job Handlers ----

/// Create and start an archive job (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/archive/jobs", tag = "Archive",
    request_body = CreateArchiveJob,
    responses((status = 201, description = "Archive job created and started"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_job(
    admin_user: AdminUser,
    archive_service: web::Data<ArchiveService>,
    payload: web::Json<CreateArchiveJob>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match archive_service
        .create_job(create, admin_user.0.tenant_id)
        .await
    {
        Ok(job) => Ok(HttpResponse::Created().json(job)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List archive jobs (paginated, optional policy filter)
#[utoipa::path(
    get, path = "/api/v1/archive/jobs", tag = "Archive",
    params(ListJobsQuery),
    responses((status = 200, description = "List of archive jobs")),
    security(("bearer_auth" = []))
)]
pub async fn list_jobs(
    auth_user: AuthUser,
    archive_service: web::Data<ArchiveService>,
    query: web::Query<ListJobsQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let q = query.into_inner();
    let result = if let Some(policy_id) = q.policy_id {
        archive_service
            .list_jobs_by_policy(policy_id, auth_user.0.tenant_id, q.into())
            .await
    } else {
        archive_service
            .list_jobs(auth_user.0.tenant_id, q.into())
            .await
    };

    match result {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get an archive job by ID
#[utoipa::path(
    get, path = "/api/v1/archive/jobs/{id}", tag = "Archive",
    params(("id" = i64, Path, description = "Archive job ID")),
    responses((status = 200, description = "Archive job found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_job(
    auth_user: AuthUser,
    archive_service: web::Data<ArchiveService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match archive_service.get_job(id, auth_user.0.tenant_id).await {
        Ok(job) => Ok(HttpResponse::Ok().json(job)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete an archive job (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/archive/jobs/{id}", tag = "Archive",
    params(("id" = i64, Path, description = "Archive job ID")),
    responses((status = 204, description = "Archive job deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn delete_job(
    admin_user: AdminUser,
    archive_service: web::Data<ArchiveService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match archive_service.delete_job(id, admin_user.0.tenant_id).await {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// ---- Archive Record Handlers ----

/// List archived records (paginated, optional filters)
#[utoipa::path(
    get, path = "/api/v1/archive/records", tag = "Archive",
    params(ListRecordsQuery),
    responses((status = 200, description = "List of archive records")),
    security(("bearer_auth" = []))
)]
pub async fn list_records(
    auth_user: AuthUser,
    archive_service: web::Data<ArchiveService>,
    query: web::Query<ListRecordsQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let q = query.into_inner();
    let source_table = q.source_table.clone();
    match archive_service
        .list_records(auth_user.0.tenant_id, source_table, q.source_id, q.into())
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get an archive record by ID
#[utoipa::path(
    get, path = "/api/v1/archive/records/{id}", tag = "Archive",
    params(("id" = i64, Path, description = "Archive record ID")),
    responses((status = 200, description = "Archive record found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_record(
    auth_user: AuthUser,
    archive_service: web::Data<ArchiveService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match archive_service.get_record(id, auth_user.0.tenant_id).await {
        Ok(record) => Ok(HttpResponse::Ok().json(record)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore archived records (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/archive/records/restore", tag = "Archive",
    request_body = RestoreRequest,
    responses(
        (status = 200, description = "Records restored"),
        (status = 400, description = "Bad request — empty IDs list"),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn restore_records(
    admin_user: AdminUser,
    archive_service: web::Data<ArchiveService>,
    payload: web::Json<RestoreRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    if req.record_ids.is_empty() {
        return Ok(crate::error::ApiError::BadRequest(
            "Record IDs list cannot be empty".to_string(),
        )
        .to_http_response(i18n, locale.as_str()));
    }
    if req.record_ids.len() > 100 {
        return Ok(crate::error::ApiError::BadRequest(
            "Record IDs list cannot exceed 100 items".to_string(),
        )
        .to_http_response(i18n, locale.as_str()));
    }
    match archive_service
        .restore_records(req, admin_user.0.tenant_id)
        .await
    {
        Ok((restored_records, failed_tuples)) => {
            let items: Vec<_> = restored_records;
            let failed: Vec<BulkRestoreFailed> = failed_tuples
                .into_iter()
                .map(|(id, reason)| BulkRestoreFailed { id, reason })
                .collect();
            Ok(HttpResponse::Ok().json(BulkRestoreResponse {
                restored: items.len(),
                items,
                failed,
            }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Permanently delete an archive record (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/archive/records/{id}", tag = "Archive",
    params(("id" = i64, Path, description = "Archive record ID")),
    responses((status = 204, description = "Archive record deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn delete_record(
    admin_user: AdminUser,
    archive_service: web::Data<ArchiveService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match archive_service
        .delete_record(id, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure archive routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/archive/policies")
            .route(web::get().to(list_policies))
            .route(web::post().to(create_policy)),
    )
    .service(
        web::resource("/v1/archive/policies/active").route(web::get().to(list_active_policies)),
    )
    .service(
        web::resource("/v1/archive/policies/{id}")
            .route(web::get().to(get_policy))
            .route(web::put().to(update_policy))
            .route(web::delete().to(delete_policy)),
    )
    .service(
        web::resource("/v1/archive/jobs")
            .route(web::get().to(list_jobs))
            .route(web::post().to(create_job)),
    )
    .service(
        web::resource("/v1/archive/jobs/{id}")
            .route(web::get().to(get_job))
            .route(web::delete().to(delete_job)),
    )
    .service(web::resource("/v1/archive/records").route(web::get().to(list_records)))
    .service(
        web::resource("/v1/archive/records/{id}")
            .route(web::get().to(get_record))
            .route(web::delete().to(delete_record)),
    )
    .service(web::resource("/v1/archive/records/restore").route(web::post().to(restore_records)));
}
