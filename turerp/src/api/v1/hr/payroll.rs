//! Payroll handlers

use actix_web::{web, HttpResponse};

use crate::domain::hr::service::HrService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

/// Calculate payroll (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/payroll/calculate", tag = "HR",
    request_body = CalculatePayrollRequest,
    responses((status = 200, description = "Payroll calculated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn calculate_payroll(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    payload: web::Json<CalculatePayrollRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        hr_service.calculate_payroll(
            admin_user.0.tenant_id,
            payload.employee_id,
            payload.period_start,
            payload.period_end
        ),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get payroll by employee
#[utoipa::path(
    get, path = "/api/v1/hr/payroll/employee/{employee_id}", tag = "HR",
    params(("employee_id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Payroll records")),
    security(("bearer_auth" = []))
)]
pub async fn get_payroll_by_employee(
    auth_user: AuthUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        hr_service.get_payroll_by_employee(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Mark payroll as paid (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/payroll/{id}/paid", tag = "HR",
    params(("id" = i64, Path, description = "Payroll ID")),
    responses((status = 200, description = "Payroll marked as paid"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn mark_payroll_paid(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        hr_service.mark_payroll_paid(*path, admin_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Soft-delete payroll (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/payroll/{id}/soft-delete", tag = "HR",
    params(("id" = i64, Path, description = "Payroll ID")),
    responses((status = 204, description = "Payroll soft-deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_payroll(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let user_id: i64 = admin_user.0.user_id()?;
    hr_service
        .soft_delete_payroll(*path, admin_user.0.tenant_id, user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Restore a soft-deleted payroll (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/hr/payroll/{id}/restore", tag = "HR",
    params(("id" = i64, Path, description = "Payroll ID")),
    responses((status = 200, description = "Payroll restored"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_payroll(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let payroll = hr_service
        .restore_payroll(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(payroll))
}

/// List soft-deleted payroll records (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/hr/payroll/deleted", tag = "HR",
    responses((status = 200, description = "List of soft-deleted payroll records"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_payroll(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
) -> ApiResult<HttpResponse> {
    let payroll = hr_service
        .list_deleted_payroll(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(payroll))
}

/// Permanently destroy payroll (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/payroll/{id}/destroy", tag = "HR",
    params(("id" = i64, Path, description = "Payroll ID")),
    responses((status = 204, description = "Payroll permanently deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_payroll(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    hr_service
        .destroy_payroll(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct CalculatePayrollRequest {
    pub employee_id: i64,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}
