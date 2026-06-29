//! SGK (Social Security) payroll API handlers

use actix_web::{web, HttpResponse};

use crate::domain::hr::sgk::ebildirge::EmployerInfo;
use crate::domain::hr::sgk::model::{CreateEmployeeBonus, CreateSgkEmployeeRegistration};
use crate::domain::hr::sgk::service::SgkPayrollService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

/// Register employee with SGK (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/sgk/register", tag = "HR",
    request_body = CreateSgkEmployeeRegistration,
    responses((status = 201, description = "SGK registration created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn register_sgk_employee(
    admin_user: AdminUser,
    sgk_service: web::Data<SgkPayrollService>,
    payload: web::Json<CreateSgkEmployeeRegistration>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    // Force the auth-derived tenant onto the body so a tenant admin cannot
    // create an SGK registration attributed to another tenant via a
    // client-supplied `tenant_id` field.
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        sgk_service.register_employee(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Calculate SGK-compliant payroll (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/sgk/calculate", tag = "HR",
    request_body = CalculateSgkPayrollRequest,
    responses((status = 200, description = "Payroll calculated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn calculate_sgk_payroll(
    admin_user: AdminUser,
    sgk_service: web::Data<SgkPayrollService>,
    payload: web::Json<CalculateSgkPayrollRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        sgk_service.calculate_sgk_payroll(
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

/// Add a bonus to an employee (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/hr/sgk/bonus", tag = "HR",
    request_body = CreateEmployeeBonus,
    responses((status = 201, description = "Bonus added"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn add_bonus(
    admin_user: AdminUser,
    sgk_service: web::Data<SgkPayrollService>,
    payload: web::Json<CreateEmployeeBonus>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    // Force the auth-derived tenant onto the body so a tenant admin cannot
    // create a bonus attributed to another tenant via a client-supplied
    // `tenant_id` field.
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        sgk_service.add_bonus(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get payroll summary for a period (requires auth)
#[utoipa::path(
    get, path = "/api/v1/hr/sgk/summary/{year}/{month}", tag = "HR",
    params(("year" = i32, Path, description = "Year"), ("month" = i32, Path, description = "Month 1-12")),
    responses((status = 200, description = "Payroll summary")),
    security(("bearer_auth" = []))
)]
pub async fn get_payroll_summary(
    auth_user: AuthUser,
    sgk_service: web::Data<SgkPayrollService>,
    path: web::Path<(i32, i32)>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let (year, month) = path.into_inner();
    json_resp!(
        sgk_service.get_payroll_summary(auth_user.0.tenant_id, year, month),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Generate e-Bildirge XML for a month (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/hr/sgk/ebildirge/{year}/{month}", tag = "HR",
    params(("year" = i32, Path, description = "Year"), ("month" = i32, Path, description = "Month 1-12")),
    request_body = GenerateEbildirgeRequest,
    responses((status = 200, description = "e-Bildirge XML generated", content_type = "application/xml")),
    security(("bearer_auth" = []))
)]
pub async fn generate_ebildirge(
    admin_user: AdminUser,
    sgk_service: web::Data<SgkPayrollService>,
    path: web::Path<(i32, i32)>,
    payload: web::Json<GenerateEbildirgeRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let (year, month) = path.into_inner();
    match sgk_service
        .generate_ebildirge(
            admin_user.0.tenant_id,
            year,
            month,
            payload.employer_info.clone(),
        )
        .await
    {
        Ok(xml) => Ok(HttpResponse::Ok().content_type("application/xml").body(xml)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct CalculateSgkPayrollRequest {
    pub employee_id: i64,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Deserialize, utoipa::ToSchema, Clone)]
pub struct GenerateEbildirgeRequest {
    pub employer_info: EmployerInfo,
}
