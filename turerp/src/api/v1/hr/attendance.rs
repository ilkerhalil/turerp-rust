//! Attendance handlers

use actix_web::{web, HttpResponse};

use crate::domain::hr::model::CreateAttendance;
use crate::domain::hr::service::HrService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

/// Record attendance (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/attendance", tag = "HR",
    request_body = CreateAttendance,
    responses((status = 201, description = "Attendance recorded"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn record_attendance(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    payload: web::Json<CreateAttendance>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    // Force the auth-derived tenant onto the body so a tenant admin cannot
    // record attendance attributed to another tenant via a client-supplied
    // `tenant_id` field.
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        hr_service.record_attendance(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get attendance by employee
#[utoipa::path(
    get, path = "/api/v1/hr/attendance/employee/{employee_id}", tag = "HR",
    params(("employee_id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Attendance records")),
    security(("bearer_auth" = []))
)]
pub async fn get_attendance_by_employee(
    auth_user: AuthUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        hr_service.get_attendance_by_employee(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Soft-delete attendance (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/attendance/{id}/soft-delete", tag = "HR",
    params(("id" = i64, Path, description = "Attendance ID")),
    responses((status = 204, description = "Attendance soft-deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_attendance(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let user_id: i64 = admin_user.0.user_id()?;
    hr_service
        .soft_delete_attendance(*path, admin_user.0.tenant_id, user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Restore a soft-deleted attendance (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/hr/attendance/{id}/restore", tag = "HR",
    params(("id" = i64, Path, description = "Attendance ID")),
    responses((status = 200, description = "Attendance restored"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_attendance(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let attendance = hr_service
        .restore_attendance(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(attendance))
}

/// List soft-deleted attendance (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/hr/attendance/deleted", tag = "HR",
    responses((status = 200, description = "List of soft-deleted attendance records"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_attendance(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
) -> ApiResult<HttpResponse> {
    let attendance = hr_service
        .list_deleted_attendance(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(attendance))
}

/// Permanently destroy attendance (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/attendance/{id}/destroy", tag = "HR",
    params(("id" = i64, Path, description = "Attendance ID")),
    responses((status = 204, description = "Attendance permanently deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_attendance(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    hr_service
        .destroy_attendance(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
