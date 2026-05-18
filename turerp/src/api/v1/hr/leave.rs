//! Leave request and leave type handlers

use actix_web::{web, HttpResponse};

use crate::domain::hr::model::CreateLeaveRequest;
use crate::domain::hr::service::HrService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

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
    json_resp!(
        hr_service.create_leave_request(payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
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
    json_resp!(
        hr_service.get_leave_requests_by_employee(*path),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    let approver_id: i64 = admin_user.0.user_id()?;
    json_resp!(
        hr_service.approve_leave_request(*path, approver_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    let approver_id: i64 = admin_user.0.user_id()?;
    json_resp!(
        hr_service.reject_leave_request(*path, approver_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Soft-delete a leave request (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/leave-requests/{id}/soft-delete", tag = "HR",
    params(("id" = i64, Path, description = "Leave request ID")),
    responses((status = 204, description = "Leave request soft-deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_leave_request(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let user_id: i64 = admin_user.0.user_id()?;
    hr_service.soft_delete_leave_request(*path, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
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
    json_resp!(
        hr_service.get_leave_types(auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Soft-delete a leave type (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/hr/leave-types/{id}/soft-delete", tag = "HR",
    params(("id" = i64, Path, description = "Leave type ID")),
    responses((status = 204, description = "Leave type soft-deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_leave_type(
    admin_user: AdminUser,
    hr_service: web::Data<HrService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let user_id: i64 = admin_user.0.user_id()?;
    hr_service
        .soft_delete_leave_type(*path, admin_user.0.tenant_id, user_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
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
