//! Audit log API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::audit::model::AuditLogQueryParams;
use crate::domain::audit::service::AuditService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::AdminUser;

/// Get audit logs (requires admin role)
#[utoipa::path(
    get,
    path = "/api/v1/audit-logs",
    tag = "Audit",
    params(
        ("user_id" = Option<i64>, Query, description = "Filter by user ID"),
        ("path" = Option<String>, Query, description = "Filter by path (contains)"),
        ("from_date" = Option<String>, Query, description = "Filter from date (ISO 8601)"),
        ("to_date" = Option<String>, Query, description = "Filter to date (ISO 8601)"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Paginated list of audit logs"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_audit_logs(
    _admin_user: AdminUser,
    audit_service: web::Data<AuditService>,
    query: web::Query<AuditLogQueryParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = _admin_user.0.tenant_id;
    match audit_service.get_logs(tenant_id, query.into_inner()).await {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure audit routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/v1/audit-logs").route(web::get().to(get_audit_logs)));
}
