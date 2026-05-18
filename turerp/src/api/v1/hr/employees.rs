//! Employee handlers

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::hr::model::{CreateEmployee, EmployeeResponse};
use crate::domain::hr::service::HrService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
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
    json_resp!(
        hr_service.create_employee(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
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
    json_resp!(
        hr_service.get_employees_paginated(auth_user.0.tenant_id, query.page, query.per_page),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    json_resp!(
        hr_service.get_employee(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    json_resp!(
        hr_service.update_employee_status(*path, payload.into_inner().status),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    json_resp!(
        hr_service.terminate_employee(*path),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Soft-delete an employee (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/employees/{id}/soft-delete", tag = "HR",
    params(("id" = i64, Path, description = "Employee ID")),
    responses((status = 204, description = "Employee soft-deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_employee(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let user_id: i64 = admin_user.0.user_id()?;
    hr_service
        .soft_delete_employee(*path, admin_user.0.tenant_id, user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
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

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateEmployeeStatusRequest {
    pub status: crate::domain::hr::model::EmployeeStatus,
}
