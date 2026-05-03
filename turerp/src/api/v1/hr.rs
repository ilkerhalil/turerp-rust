//! HR (Human Resources) API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::hr::model::{
    CreateAttendance, CreateEmployee, CreateLeaveRequest, EmployeeStatus,
};
use crate::domain::hr::service::HrService;
use crate::error::ApiResult;
use crate::middleware::{AdminUser, AuthUser};

/// Create employee (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/employees", tag = "HR",
    request_body = CreateEmployee,
    responses((status = 201, description = "Employee created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_employee(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    payload: web::Json<CreateEmployee>,
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let employee = hr_service.create_employee(create).await?;
    Ok(HttpResponse::Created().json(employee))
}

/// Get all employees
#[utoipa::path(
    get, path = "/api/v1/hr/employees", tag = "HR",
    responses((status = 200, description = "List of employees")),
    security(("bearer_auth" = []))
)]
pub async fn get_employees(
    auth_user: AuthUser,
    hr_service: web::Data<HrService>,
    query: web::Query<PaginationParams>,
) -> ApiResult<HttpResponse> {
    let result = hr_service
        .get_employees_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Get employee by ID
#[utoipa::path(
    get, path = "/api/v1/hr/employees/{id}", tag = "HR",
    params(("id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Employee found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_employee(
    _auth_user: AuthUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let employee = hr_service.get_employee(*path).await?;
    Ok(HttpResponse::Ok().json(employee))
}

/// Update employee status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/hr/employees/{id}/status", tag = "HR",
    params(("id" = i64, Path, description = "Employee ID")),
    request_body = UpdateEmployeeStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_employee_status(
    _admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateEmployeeStatusRequest>,
) -> ApiResult<HttpResponse> {
    let employee = hr_service
        .update_employee_status(*path, payload.into_inner().status)
        .await?;
    Ok(HttpResponse::Ok().json(employee))
}

/// Terminate employee (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/employees/{id}/terminate", tag = "HR",
    params(("id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Employee terminated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn terminate_employee(
    _admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let employee = hr_service.terminate_employee(*path).await?;
    Ok(HttpResponse::Ok().json(employee))
}

/// Record attendance (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/attendance", tag = "HR",
    request_body = CreateAttendance,
    responses((status = 201, description = "Attendance recorded"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn record_attendance(
    _admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    payload: web::Json<CreateAttendance>,
) -> ApiResult<HttpResponse> {
    let attendance = hr_service.record_attendance(payload.into_inner()).await?;
    Ok(HttpResponse::Created().json(attendance))
}

/// Get attendance by employee
#[utoipa::path(
    get, path = "/api/v1/hr/attendance/employee/{employee_id}", tag = "HR",
    params(("employee_id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Attendance records")),
    security(("bearer_auth" = []))
)]
pub async fn get_attendance_by_employee(
    _auth_user: AuthUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let attendance = hr_service.get_attendance_by_employee(*path).await?;
    Ok(HttpResponse::Ok().json(attendance))
}

/// Create leave request
#[utoipa::path(
    post, path = "/api/v1/hr/leave-requests", tag = "HR",
    request_body = CreateLeaveRequest,
    responses((status = 201, description = "Leave request created")),
    security(("bearer_auth" = []))
)]
pub async fn create_leave_request(
    _auth_user: AuthUser,
    hr_service: web::Data<HrService>,
    payload: web::Json<CreateLeaveRequest>,
) -> ApiResult<HttpResponse> {
    let request = hr_service
        .create_leave_request(payload.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(request))
}

/// Get leave requests by employee
#[utoipa::path(
    get, path = "/api/v1/hr/leave-requests/employee/{employee_id}", tag = "HR",
    params(("employee_id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Leave requests")),
    security(("bearer_auth" = []))
)]
pub async fn get_leave_requests_by_employee(
    _auth_user: AuthUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let requests = hr_service.get_leave_requests_by_employee(*path).await?;
    Ok(HttpResponse::Ok().json(requests))
}

/// Approve leave request (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/leave-requests/{id}/approve", tag = "HR",
    params(("id" = i64, Path, description = "Leave request ID")),
    request_body = ApproveRequest,
    responses((status = 200, description = "Leave request approved"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn approve_leave_request(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let approver_id: i64 = admin_user
        .0
        .sub
        .parse()
        .map_err(|_| crate::error::ApiError::InvalidToken("Invalid user ID in token".into()))?;
    let request = hr_service.approve_leave_request(*path, approver_id).await?;
    Ok(HttpResponse::Ok().json(request))
}

/// Reject leave request (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/leave-requests/{id}/reject", tag = "HR",
    params(("id" = i64, Path, description = "Leave request ID")),
    responses((status = 200, description = "Leave request rejected"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn reject_leave_request(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let approver_id: i64 = admin_user
        .0
        .sub
        .parse()
        .map_err(|_| crate::error::ApiError::InvalidToken("Invalid user ID in token".into()))?;
    let request = hr_service.reject_leave_request(*path, approver_id).await?;
    Ok(HttpResponse::Ok().json(request))
}

/// Get leave types
#[utoipa::path(
    get, path = "/api/v1/hr/leave-types", tag = "HR",
    responses((status = 200, description = "List of leave types")),
    security(("bearer_auth" = []))
)]
pub async fn get_leave_types(
    auth_user: AuthUser,
    hr_service: web::Data<HrService>,
) -> ApiResult<HttpResponse> {
    let types = hr_service.get_leave_types(auth_user.0.tenant_id).await?;
    Ok(HttpResponse::Ok().json(types))
}

/// Calculate payroll (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/payroll/calculate", tag = "HR",
    request_body = CalculatePayrollRequest,
    responses((status = 200, description = "Payroll calculated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn calculate_payroll(
    _admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    payload: web::Json<CalculatePayrollRequest>,
) -> ApiResult<HttpResponse> {
    let payroll = hr_service
        .calculate_payroll(
            payload.employee_id,
            payload.period_start,
            payload.period_end,
        )
        .await?;
    Ok(HttpResponse::Ok().json(payroll))
}

/// Get payroll by employee
#[utoipa::path(
    get, path = "/api/v1/hr/payroll/employee/{employee_id}", tag = "HR",
    params(("employee_id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Payroll records")),
    security(("bearer_auth" = []))
)]
pub async fn get_payroll_by_employee(
    _auth_user: AuthUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let payroll = hr_service.get_payroll_by_employee(*path).await?;
    Ok(HttpResponse::Ok().json(payroll))
}

/// Mark payroll as paid (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/payroll/{id}/paid", tag = "HR",
    params(("id" = i64, Path, description = "Payroll ID")),
    responses((status = 200, description = "Payroll marked as paid"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn mark_payroll_paid(
    _admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let payroll = hr_service.mark_payroll_paid(*path).await?;
    Ok(HttpResponse::Ok().json(payroll))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateEmployeeStatusRequest {
    pub status: EmployeeStatus,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct ApproveRequest {
    pub approver_id: Option<i64>,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct CalculatePayrollRequest {
    pub employee_id: i64,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

/// Configure HR routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/hr/employees")
            .route(web::get().to(get_employees))
            .route(web::post().to(create_employee)),
    )
    .service(web::resource("/v1/hr/employees/{id}").route(web::get().to(get_employee)))
    .service(
        web::resource("/v1/hr/employees/{id}/status").route(web::put().to(update_employee_status)),
    )
    .service(
        web::resource("/v1/hr/employees/{id}/terminate").route(web::post().to(terminate_employee)),
    )
    .service(web::resource("/v1/hr/attendance").route(web::post().to(record_attendance)))
    .service(
        web::resource("/v1/hr/attendance/employee/{employee_id}")
            .route(web::get().to(get_attendance_by_employee)),
    )
    .service(web::resource("/v1/hr/leave-requests").route(web::post().to(create_leave_request)))
    .service(
        web::resource("/v1/hr/leave-requests/employee/{employee_id}")
            .route(web::get().to(get_leave_requests_by_employee)),
    )
    .service(
        web::resource("/v1/hr/leave-requests/{id}/approve")
            .route(web::post().to(approve_leave_request)),
    )
    .service(
        web::resource("/v1/hr/leave-requests/{id}/reject")
            .route(web::post().to(reject_leave_request)),
    )
    .service(web::resource("/v1/hr/leave-types").route(web::get().to(get_leave_types)))
    .service(web::resource("/v1/hr/payroll/calculate").route(web::post().to(calculate_payroll)))
    .service(
        web::resource("/v1/hr/payroll/employee/{employee_id}")
            .route(web::get().to(get_payroll_by_employee)),
    )
    .service(web::resource("/v1/hr/payroll/{id}/paid").route(web::post().to(mark_payroll_paid)));
}
