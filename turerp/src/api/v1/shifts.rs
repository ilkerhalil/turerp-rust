//! Shift Planning API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::shift::model::{
    ClockInRequest, ClockOutRequest, CreateShift, CreateShiftAssignment, ShiftReportQuery,
    ShiftResponse, UpdateShift,
};
use crate::domain::shift::service::ShiftService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

/// Create shift (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/shifts", tag = "Shift Planning",
    request_body = CreateShift,
    responses((status = 201, description = "Shift created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_shift(
    admin_user: AdminUser,
    shift_service: web::Data<ShiftService>,
    payload: web::Json<CreateShift>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        shift_service.create_shift(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get all shifts
#[utoipa::path(
    get, path = "/api/v1/shifts", tag = "Shift Planning",
    responses((status = 200, description = "List of shifts")),
    security(("bearer_auth" = []))
)]
pub async fn get_shifts(
    auth_user: AuthUser,
    shift_service: web::Data<ShiftService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        shift_service.get_shifts_paginated(auth_user.0.tenant_id, query.page, query.per_page),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get shift by ID
#[utoipa::path(
    get, path = "/api/v1/shifts/{id}", tag = "Shift Planning",
    params(("id" = i64, Path, description = "Shift ID")),
    responses((status = 200, description = "Shift found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_shift(
    auth_user: AuthUser,
    shift_service: web::Data<ShiftService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        shift_service.get_shift(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Update shift (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/shifts/{id}", tag = "Shift Planning",
    params(("id" = i64, Path, description = "Shift ID")),
    request_body = UpdateShift,
    responses((status = 200, description = "Shift updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn update_shift(
    admin_user: AdminUser,
    shift_service: web::Data<ShiftService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateShift>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        shift_service.update_shift(*path, admin_user.0.tenant_id, payload.into_inner()),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Delete shift (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/shifts/{id}", tag = "Shift Planning",
    params(("id" = i64, Path, description = "Shift ID")),
    responses((status = 204, description = "Shift deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn delete_shift(
    admin_user: AdminUser,
    shift_service: web::Data<ShiftService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    shift_service
        .delete_shift(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Soft-delete a shift (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/shifts/{id}/soft-delete", tag = "Shift Planning",
    params(("id" = i64, Path, description = "Shift ID")),
    responses((status = 204, description = "Shift soft-deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_shift(
    admin_user: AdminUser,
    shift_service: web::Data<ShiftService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let user_id: i64 = admin_user.0.user_id()?;
    shift_service
        .soft_delete_shift(*path, admin_user.0.tenant_id, user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Restore a soft-deleted shift (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/shifts/{id}/restore", tag = "Shift Planning",
    params(("id" = i64, Path, description = "Shift ID")),
    responses((status = 200, description = "Shift restored"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_shift(
    admin_user: AdminUser,
    shift_service: web::Data<ShiftService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let shift = shift_service
        .restore_shift(*path, admin_user.0.tenant_id)
        .await?;
    let response: ShiftResponse = shift;
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted shifts (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/shifts/deleted", tag = "Shift Planning",
    responses((status = 200, description = "List of soft-deleted shifts"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_shifts(
    admin_user: AdminUser,
    shift_service: web::Data<ShiftService>,
) -> ApiResult<HttpResponse> {
    let shifts = shift_service
        .list_deleted_shifts(admin_user.0.tenant_id)
        .await?;
    let responses: Vec<ShiftResponse> = shifts.into_iter().collect();
    Ok(HttpResponse::Ok().json(responses))
}

/// Permanently destroy a shift (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/shifts/{id}/destroy", tag = "Shift Planning",
    params(("id" = i64, Path, description = "Shift ID")),
    responses((status = 204, description = "Shift permanently deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_shift(
    admin_user: AdminUser,
    shift_service: web::Data<ShiftService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    shift_service
        .destroy_shift(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Assign employee to shift (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/shifts/assignments", tag = "Shift Planning",
    request_body = CreateShiftAssignment,
    responses((status = 201, description = "Assignment created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_assignment(
    admin_user: AdminUser,
    shift_service: web::Data<ShiftService>,
    payload: web::Json<CreateShiftAssignment>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        shift_service.assign_employee(admin_user.0.tenant_id, payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get assignments by employee
#[utoipa::path(
    get, path = "/api/v1/shifts/assignments/employee/{employee_id}", tag = "Shift Planning",
    params(("employee_id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Shift assignments")),
    security(("bearer_auth" = []))
)]
pub async fn get_assignments_by_employee(
    auth_user: AuthUser,
    shift_service: web::Data<ShiftService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let assignments = shift_service
        .get_assignments_by_employee(auth_user.0.tenant_id, *path)
        .await?;
    Ok(HttpResponse::Ok().json(assignments))
}

/// Get assignments by shift
#[utoipa::path(
    get, path = "/api/v1/shifts/{shift_id}/assignments", tag = "Shift Planning",
    params(("shift_id" = i64, Path, description = "Shift ID")),
    responses((status = 200, description = "Shift assignments")),
    security(("bearer_auth" = []))
)]
pub async fn get_assignments_by_shift(
    auth_user: AuthUser,
    shift_service: web::Data<ShiftService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let assignments = shift_service
        .get_assignments_by_shift(auth_user.0.tenant_id, *path)
        .await?;
    Ok(HttpResponse::Ok().json(assignments))
}

/// Remove assignment (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/shifts/assignments/{id}", tag = "Shift Planning",
    params(("id" = i64, Path, description = "Assignment ID")),
    responses((status = 204, description = "Assignment removed"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn delete_assignment(
    admin_user: AdminUser,
    shift_service: web::Data<ShiftService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    shift_service
        .remove_assignment(admin_user.0.tenant_id, *path)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Clock in
#[utoipa::path(
    post, path = "/api/v1/shifts/attendance/clock-in", tag = "Shift Planning",
    request_body = ClockInRequest,
    responses((status = 201, description = "Clocked in")),
    security(("bearer_auth" = []))
)]
pub async fn clock_in(
    auth_user: AuthUser,
    shift_service: web::Data<ShiftService>,
    payload: web::Json<ClockInRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        shift_service.clock_in(auth_user.0.tenant_id, payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Clock out
#[utoipa::path(
    post, path = "/api/v1/shifts/attendance/clock-out", tag = "Shift Planning",
    request_body = ClockOutRequest,
    responses((status = 200, description = "Clocked out")),
    security(("bearer_auth" = []))
)]
pub async fn clock_out(
    auth_user: AuthUser,
    shift_service: web::Data<ShiftService>,
    payload: web::Json<ClockOutRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        shift_service.clock_out(auth_user.0.tenant_id, payload.into_inner()),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get attendance by employee
#[utoipa::path(
    get, path = "/api/v1/shifts/attendance/employee/{employee_id}", tag = "Shift Planning",
    params(("employee_id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Attendance records")),
    security(("bearer_auth" = []))
)]
pub async fn get_attendance_by_employee(
    auth_user: AuthUser,
    shift_service: web::Data<ShiftService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let records = shift_service
        .get_attendance_by_employee(auth_user.0.tenant_id, *path)
        .await?;
    Ok(HttpResponse::Ok().json(records))
}

/// Generate shift report
#[utoipa::path(
    post, path = "/api/v1/shifts/reports", tag = "Shift Planning",
    request_body = ShiftReportQuery,
    responses((status = 200, description = "Shift report generated")),
    security(("bearer_auth" = []))
)]
pub async fn generate_report(
    auth_user: AuthUser,
    shift_service: web::Data<ShiftService>,
    payload: web::Json<ShiftReportQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        shift_service.generate_shift_report(auth_user.0.tenant_id, payload.into_inner()),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Calculate overtime
#[utoipa::path(
    post, path = "/api/v1/shifts/overtime", tag = "Shift Planning",
    request_body = CalculateOvertimeRequest,
    responses((status = 200, description = "Overtime calculated")),
    security(("bearer_auth" = []))
)]
pub async fn calculate_overtime(
    auth_user: AuthUser,
    shift_service: web::Data<ShiftService>,
    payload: web::Json<CalculateOvertimeRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        shift_service.calculate_overtime(
            auth_user.0.tenant_id,
            payload.employee_id,
            payload.period_start,
            payload.period_end,
            payload.expected_hours_per_day,
            payload.overtime_rate,
        ),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct CalculateOvertimeRequest {
    pub employee_id: i64,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
    pub expected_hours_per_day: rust_decimal::Decimal,
    pub overtime_rate: rust_decimal::Decimal,
}

/// Configure Shift Planning routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/shifts")
            .route(web::get().to(get_shifts))
            .route(web::post().to(create_shift)),
    )
    .service(web::resource("/v1/shifts/deleted").route(web::get().to(list_deleted_shifts)))
    .service(
        web::resource("/v1/shifts/{id}")
            .route(web::get().to(get_shift))
            .route(web::put().to(update_shift))
            .route(web::delete().to(delete_shift)),
    )
    .service(
        web::resource("/v1/shifts/{id}/soft-delete").route(web::delete().to(soft_delete_shift)),
    )
    .service(web::resource("/v1/shifts/{id}/restore").route(web::put().to(restore_shift)))
    .service(web::resource("/v1/shifts/{id}/destroy").route(web::delete().to(destroy_shift)))
    .service(web::resource("/v1/shifts/assignments").route(web::post().to(create_assignment)))
    .service(
        web::resource("/v1/shifts/assignments/employee/{employee_id}")
            .route(web::get().to(get_assignments_by_employee)),
    )
    .service(
        web::resource("/v1/shifts/{shift_id}/assignments")
            .route(web::get().to(get_assignments_by_shift)),
    )
    .service(
        web::resource("/v1/shifts/assignments/{id}").route(web::delete().to(delete_assignment)),
    )
    .service(web::resource("/v1/shifts/attendance/clock-in").route(web::post().to(clock_in)))
    .service(web::resource("/v1/shifts/attendance/clock-out").route(web::post().to(clock_out)))
    .service(
        web::resource("/v1/shifts/attendance/employee/{employee_id}")
            .route(web::get().to(get_attendance_by_employee)),
    )
    .service(web::resource("/v1/shifts/reports").route(web::post().to(generate_report)))
    .service(web::resource("/v1/shifts/overtime").route(web::post().to(calculate_overtime)));
}
