//! HR (Human Resources) API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::hr::model::{
    CreateAttendance, CreateEmployee, CreateLeaveRequest, EmployeeResponse, EmployeeStatus,
};
use crate::domain::hr::service::HrService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match hr_service.create_employee(create).await {
        Ok(employee) => Ok(HttpResponse::Created().json(employee)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match hr_service
        .get_employees_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get employee by ID
#[utoipa::path(
    get, path = "/api/v1/hr/employees/{id}", tag = "HR",
    params(("id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Employee found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_employee(
    auth_user: AuthUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match hr_service.get_employee(*path, auth_user.0.tenant_id).await {
        Ok(employee) => Ok(HttpResponse::Ok().json(employee)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match hr_service
        .update_employee_status(*path, payload.into_inner().status)
        .await
    {
        Ok(employee) => Ok(HttpResponse::Ok().json(employee)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match hr_service.terminate_employee(*path).await {
        Ok(employee) => Ok(HttpResponse::Ok().json(employee)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft-delete an employee (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/employees/{id}/soft-delete", tag = "HR",
    params(("id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Employee soft-deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_employee(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let user_id: i64 = admin_user.0.sub.parse().unwrap_or(0);
    hr_service
        .soft_delete_employee(*path, admin_user.0.tenant_id, user_id)
        .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Employee soft-deleted"})))
}

/// Restore a soft-deleted employee (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/hr/employees/{id}/restore", tag = "HR",
    params(("id" = i64, Path, description = "Employee ID")),
    responses((status = 200, description = "Employee restored"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_employee(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let employee = hr_service
        .restore_employee(*path, admin_user.0.tenant_id)
        .await?;
    let response: EmployeeResponse = employee.into();
    Ok(HttpResponse::Ok().json(response))
}

/// List soft-deleted employees (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/hr/employees/deleted", tag = "HR",
    responses((status = 200, description = "List of soft-deleted employees"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_employees(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
) -> ApiResult<HttpResponse> {
    let employees = hr_service
        .list_deleted_employees(admin_user.0.tenant_id)
        .await?;
    let responses: Vec<EmployeeResponse> =
        employees.into_iter().map(EmployeeResponse::from).collect();
    Ok(HttpResponse::Ok().json(responses))
}

/// Permanently destroy an employee (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/employees/{id}/destroy", tag = "HR",
    params(("id" = i64, Path, description = "Employee ID")),
    responses((status = 204, description = "Employee permanently deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_employee(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    hr_service
        .destroy_employee(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match hr_service.record_attendance(payload.into_inner()).await {
        Ok(attendance) => Ok(HttpResponse::Created().json(attendance)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match hr_service.get_attendance_by_employee(*path).await {
        Ok(attendance) => Ok(HttpResponse::Ok().json(attendance)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft-delete attendance (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/attendance/{id}/soft-delete", tag = "HR",
    params(("id" = i64, Path, description = "Attendance ID")),
    responses((status = 200, description = "Attendance soft-deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_attendance(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let user_id: i64 = admin_user.0.sub.parse().unwrap_or(0);
    hr_service.soft_delete_attendance(*path, user_id).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Attendance soft-deleted"})))
}

/// Restore a soft-deleted attendance (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/hr/attendance/{id}/restore", tag = "HR",
    params(("id" = i64, Path, description = "Attendance ID")),
    responses((status = 200, description = "Attendance restored"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_attendance(
    _admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let attendance = hr_service.restore_attendance(*path).await?;
    Ok(HttpResponse::Ok().json(attendance))
}

/// List soft-deleted attendance (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/hr/attendance/deleted", tag = "HR",
    responses((status = 200, description = "List of soft-deleted attendance records"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_attendance(
    _admin_user: AdminUser,
    hr_service: web::Data<HrService>,
) -> ApiResult<HttpResponse> {
    let attendance = hr_service.list_deleted_attendance().await?;
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
    _admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    hr_service.destroy_attendance(*path).await?;
    Ok(HttpResponse::NoContent().finish())
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match hr_service.create_leave_request(payload.into_inner()).await {
        Ok(request) => Ok(HttpResponse::Created().json(request)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match hr_service.get_leave_requests_by_employee(*path).await {
        Ok(requests) => Ok(HttpResponse::Ok().json(requests)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Approve leave request (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/leave-requests/{id}/approve", tag = "HR",
    params(("id" = i64, Path, description = "Leave request ID")),
    responses((status = 200, description = "Leave request approved"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn approve_leave_request(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let approver_id: i64 = admin_user
        .0
        .sub
        .parse()
        .map_err(|_| crate::error::ApiError::InvalidToken("Invalid user ID in token".into()))?;
    match hr_service.approve_leave_request(*path, approver_id).await {
        Ok(request) => Ok(HttpResponse::Ok().json(request)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let approver_id: i64 = admin_user
        .0
        .sub
        .parse()
        .map_err(|_| crate::error::ApiError::InvalidToken("Invalid user ID in token".into()))?;
    match hr_service.reject_leave_request(*path, approver_id).await {
        Ok(request) => Ok(HttpResponse::Ok().json(request)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft-delete a leave request (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/leave-requests/{id}/soft-delete", tag = "HR",
    params(("id" = i64, Path, description = "Leave request ID")),
    responses((status = 200, description = "Leave request soft-deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_leave_request(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let user_id: i64 = admin_user.0.sub.parse().unwrap_or(0);
    hr_service.soft_delete_leave_request(*path, user_id).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Leave request soft-deleted"})))
}

/// Restore a soft-deleted leave request (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/hr/leave-requests/{id}/restore", tag = "HR",
    params(("id" = i64, Path, description = "Leave request ID")),
    responses((status = 200, description = "Leave request restored"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_leave_request(
    _admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let request = hr_service.restore_leave_request(*path).await?;
    Ok(HttpResponse::Ok().json(request))
}

/// List soft-deleted leave requests (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/hr/leave-requests/deleted", tag = "HR",
    responses((status = 200, description = "List of soft-deleted leave requests"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_leave_requests(
    _admin_user: AdminUser,
    hr_service: web::Data<HrService>,
) -> ApiResult<HttpResponse> {
    let requests = hr_service.list_deleted_leave_requests().await?;
    Ok(HttpResponse::Ok().json(requests))
}

/// Permanently destroy a leave request (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/leave-requests/{id}/destroy", tag = "HR",
    params(("id" = i64, Path, description = "Leave request ID")),
    responses((status = 204, description = "Leave request permanently deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_leave_request(
    _admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    hr_service.destroy_leave_request(*path).await?;
    Ok(HttpResponse::NoContent().finish())
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match hr_service.get_leave_types(auth_user.0.tenant_id).await {
        Ok(types) => Ok(HttpResponse::Ok().json(types)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft-delete a leave type (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/leave-types/{id}/soft-delete", tag = "HR",
    params(("id" = i64, Path, description = "Leave type ID")),
    responses((status = 200, description = "Leave type soft-deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_leave_type(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let user_id: i64 = admin_user.0.sub.parse().unwrap_or(0);
    hr_service
        .soft_delete_leave_type(*path, admin_user.0.tenant_id, user_id)
        .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Leave type soft-deleted"})))
}

/// Restore a soft-deleted leave type (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/hr/leave-types/{id}/restore", tag = "HR",
    params(("id" = i64, Path, description = "Leave type ID")),
    responses((status = 200, description = "Leave type restored"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_leave_type(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let leave_type = hr_service
        .restore_leave_type(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(leave_type))
}

/// List soft-deleted leave types (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/hr/leave-types/deleted", tag = "HR",
    responses((status = 200, description = "List of soft-deleted leave types"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_leave_types(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
) -> ApiResult<HttpResponse> {
    let leave_types = hr_service
        .list_deleted_leave_types(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(leave_types))
}

/// Permanently destroy a leave type (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/leave-types/{id}/destroy", tag = "HR",
    params(("id" = i64, Path, description = "Leave type ID")),
    responses((status = 204, description = "Leave type permanently deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_leave_type(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    hr_service
        .destroy_leave_type(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

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
    match hr_service
        .calculate_payroll(
            admin_user.0.tenant_id,
            payload.employee_id,
            payload.period_start,
            payload.period_end,
        )
        .await
    {
        Ok(payroll) => Ok(HttpResponse::Ok().json(payroll)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match hr_service.get_payroll_by_employee(*path).await {
        Ok(payroll) => Ok(HttpResponse::Ok().json(payroll)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
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
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match hr_service.mark_payroll_paid(*path).await {
        Ok(payroll) => Ok(HttpResponse::Ok().json(payroll)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft-delete payroll (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/payroll/{id}/soft-delete", tag = "HR",
    params(("id" = i64, Path, description = "Payroll ID")),
    responses((status = 200, description = "Payroll soft-deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_payroll(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let user_id: i64 = admin_user.0.sub.parse().unwrap_or(0);
    hr_service
        .soft_delete_payroll(*path, admin_user.0.tenant_id, user_id)
        .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Payroll soft-deleted"})))
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
    .service(web::resource("/v1/hr/employees/deleted").route(web::get().to(list_deleted_employees)))
    .service(web::resource("/v1/hr/employees/{id}").route(web::get().to(get_employee)))
    .service(
        web::resource("/v1/hr/employees/{id}/status").route(web::put().to(update_employee_status)),
    )
    .service(
        web::resource("/v1/hr/employees/{id}/terminate").route(web::post().to(terminate_employee)),
    )
    .service(
        web::resource("/v1/hr/employees/{id}/soft-delete")
            .route(web::delete().to(soft_delete_employee)),
    )
    .service(web::resource("/v1/hr/employees/{id}/restore").route(web::put().to(restore_employee)))
    .service(
        web::resource("/v1/hr/employees/{id}/destroy").route(web::delete().to(destroy_employee)),
    )
    .service(web::resource("/v1/hr/attendance").route(web::post().to(record_attendance)))
    .service(
        web::resource("/v1/hr/attendance/employee/{employee_id}")
            .route(web::get().to(get_attendance_by_employee)),
    )
    .service(
        web::resource("/v1/hr/attendance/deleted").route(web::get().to(list_deleted_attendance)),
    )
    .service(
        web::resource("/v1/hr/attendance/{id}/soft-delete")
            .route(web::delete().to(soft_delete_attendance)),
    )
    .service(
        web::resource("/v1/hr/attendance/{id}/restore").route(web::put().to(restore_attendance)),
    )
    .service(
        web::resource("/v1/hr/attendance/{id}/destroy").route(web::delete().to(destroy_attendance)),
    )
    .service(web::resource("/v1/hr/leave-requests").route(web::post().to(create_leave_request)))
    .service(
        web::resource("/v1/hr/leave-requests/deleted")
            .route(web::get().to(list_deleted_leave_requests)),
    )
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
    .service(
        web::resource("/v1/hr/leave-requests/{id}/soft-delete")
            .route(web::delete().to(soft_delete_leave_request)),
    )
    .service(
        web::resource("/v1/hr/leave-requests/{id}/restore")
            .route(web::put().to(restore_leave_request)),
    )
    .service(
        web::resource("/v1/hr/leave-requests/{id}/destroy")
            .route(web::delete().to(destroy_leave_request)),
    )
    .service(web::resource("/v1/hr/leave-types").route(web::get().to(get_leave_types)))
    .service(
        web::resource("/v1/hr/leave-types/deleted").route(web::get().to(list_deleted_leave_types)),
    )
    .service(
        web::resource("/v1/hr/leave-types/{id}/soft-delete")
            .route(web::delete().to(soft_delete_leave_type)),
    )
    .service(
        web::resource("/v1/hr/leave-types/{id}/restore").route(web::put().to(restore_leave_type)),
    )
    .service(
        web::resource("/v1/hr/leave-types/{id}/destroy")
            .route(web::delete().to(destroy_leave_type)),
    )
    .service(web::resource("/v1/hr/payroll/calculate").route(web::post().to(calculate_payroll)))
    .service(
        web::resource("/v1/hr/payroll/employee/{employee_id}")
            .route(web::get().to(get_payroll_by_employee)),
    )
    .service(web::resource("/v1/hr/payroll/deleted").route(web::get().to(list_deleted_payroll)))
    .service(web::resource("/v1/hr/payroll/{id}/paid").route(web::post().to(mark_payroll_paid)))
    .service(
        web::resource("/v1/hr/payroll/{id}/soft-delete")
            .route(web::delete().to(soft_delete_payroll)),
    )
    .service(web::resource("/v1/hr/payroll/{id}/restore").route(web::put().to(restore_payroll)))
    .service(web::resource("/v1/hr/payroll/{id}/destroy").route(web::delete().to(destroy_payroll)));
}
