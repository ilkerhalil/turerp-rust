//! Opportunity handlers

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::crm::model::{CreateOpportunity, OpportunityStatus};
use crate::domain::crm::service::CrmService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Create opportunity (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/crm/opportunities", tag = "CRM",
    request_body = CreateOpportunity,
    responses((status = 201, description = "Opportunity created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_opportunity(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    payload: web::Json<CreateOpportunity>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match crm_service.create_opportunity(create).await {
        Ok(opportunity) => Ok(HttpResponse::Created().json(opportunity)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all opportunities
#[utoipa::path(
    get, path = "/api/v1/crm/opportunities", tag = "CRM",
    responses((status = 200, description = "List of opportunities")),
    security(("bearer_auth" = []))
)]
pub async fn get_opportunities(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .get_opportunities_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get opportunity by ID
#[utoipa::path(
    get, path = "/api/v1/crm/opportunities/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Opportunity ID")),
    responses((status = 200, description = "Opportunity found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_opportunity(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .get_opportunity(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(opportunity) => Ok(HttpResponse::Ok().json(opportunity)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get opportunities by status
#[utoipa::path(
    get, path = "/api/v1/crm/opportunities/status/{status}", tag = "CRM",
    params(("status" = OpportunityStatus, Path, description = "Opportunity status")),
    responses((status = 200, description = "Opportunities by status")),
    security(("bearer_auth" = []))
)]
pub async fn get_opportunities_by_status(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<OpportunityStatus>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .get_opportunities_by_status_paginated(
            auth_user.0.tenant_id,
            path.into_inner(),
            query.page,
            query.per_page,
        )
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update opportunity status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/crm/opportunities/{id}/status", tag = "CRM",
    params(("id" = i64, Path, description = "Opportunity ID")),
    request_body = UpdateOpportunityStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_opportunity_status(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateOpportunityStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .update_opportunity_status(*path, admin_user.0.tenant_id, payload.into_inner().status)
        .await
    {
        Ok(opportunity) => Ok(HttpResponse::Ok().json(opportunity)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get sales pipeline value
#[utoipa::path(
    get, path = "/api/v1/crm/pipeline-value", tag = "CRM",
    responses((status = 200, description = "Total pipeline value")),
    security(("bearer_auth" = []))
)]
pub async fn get_pipeline_value(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .get_sales_pipeline_value(auth_user.0.tenant_id)
        .await
    {
        Ok(value) => Ok(HttpResponse::Ok().json(serde_json::json!({ "pipeline_value": value }))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft-delete an opportunity (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/crm/opportunities/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Opportunity ID")),
    responses((status = 200, description = "Opportunity soft-deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_opportunity(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let user_id: i64 = admin_user.0.user_id()?;
    match crm_service
        .soft_delete_opportunity(*path, admin_user.0.tenant_id, user_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Opportunity soft-deleted".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted opportunity (admin only)
#[utoipa::path(
    put, path = "/api/v1/crm/opportunities/{id}/restore", tag = "CRM",
    params(("id" = i64, Path, description = "Opportunity ID")),
    responses((status = 200, description = "Opportunity restored"), (status = 404, description = "Not found or not deleted")),
    security(("bearer_auth" = []))
)]
pub async fn restore_opportunity(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let opportunity = crm_service
        .restore_opportunity(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(opportunity))
}

/// List soft-deleted opportunities (admin only)
#[utoipa::path(
    get, path = "/api/v1/crm/opportunities/deleted", tag = "CRM",
    responses((status = 200, description = "List of deleted opportunities")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_opportunities(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
) -> ApiResult<HttpResponse> {
    let opportunities = crm_service
        .list_deleted_opportunities(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(opportunities))
}

/// Permanently destroy an opportunity (admin only)
#[utoipa::path(
    delete, path = "/api/v1/crm/opportunities/{id}/destroy", tag = "CRM",
    params(("id" = i64, Path, description = "Opportunity ID")),
    responses((status = 204, description = "Opportunity permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_opportunity(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    crm_service
        .destroy_opportunity(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateOpportunityStatusRequest {
    pub status: OpportunityStatus,
}
