//! Lead handlers

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::crm::model::{CreateLead, LeadStatus};
use crate::domain::crm::service::CrmService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Create lead (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/crm/leads", tag = "CRM",
    request_body = CreateLead,
    responses((status = 201, description = "Lead created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_lead(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    payload: web::Json<CreateLead>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match crm_service.create_lead(create).await {
        Ok(lead) => Ok(HttpResponse::Created().json(lead)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all leads
#[utoipa::path(
    get, path = "/api/v1/crm/leads", tag = "CRM",
    responses((status = 200, description = "List of leads")),
    security(("bearer_auth" = []))
)]
pub async fn get_leads(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .get_leads_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get lead by ID
#[utoipa::path(
    get, path = "/api/v1/crm/leads/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Lead ID")),
    responses((status = 200, description = "Lead found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_lead(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service.get_lead(*path, auth_user.0.tenant_id).await {
        Ok(lead) => Ok(HttpResponse::Ok().json(lead)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get leads by status
#[utoipa::path(
    get, path = "/api/v1/crm/leads/status/{status}", tag = "CRM",
    params(("status" = LeadStatus, Path, description = "Lead status")),
    responses((status = 200, description = "Leads by status")),
    security(("bearer_auth" = []))
)]
pub async fn get_leads_by_status(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<LeadStatus>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .get_leads_by_status_paginated(
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

/// Update lead status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/crm/leads/{id}/status", tag = "CRM",
    params(("id" = i64, Path, description = "Lead ID")),
    request_body = UpdateLeadStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_lead_status(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateLeadStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .update_lead_status(*path, admin_user.0.tenant_id, payload.into_inner().status)
        .await
    {
        Ok(lead) => Ok(HttpResponse::Ok().json(lead)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Convert lead to customer (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/crm/leads/{id}/convert", tag = "CRM",
    params(("id" = i64, Path, description = "Lead ID")),
    request_body = ConvertLeadRequest,
    responses((status = 200, description = "Lead converted"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn convert_lead(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    payload: web::Json<ConvertLeadRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .convert_lead_to_customer(*path, admin_user.0.tenant_id, payload.customer_id)
        .await
    {
        Ok(lead) => Ok(HttpResponse::Ok().json(lead)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft-delete a lead (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/crm/leads/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Lead ID")),
    responses((status = 200, description = "Lead soft-deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_lead(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let user_id: i64 = admin_user.0.user_id()?;
    match crm_service
        .soft_delete_lead(*path, admin_user.0.tenant_id, user_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Lead soft-deleted".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted lead (admin only)
#[utoipa::path(
    put, path = "/api/v1/crm/leads/{id}/restore", tag = "CRM",
    params(("id" = i64, Path, description = "Lead ID")),
    responses((status = 200, description = "Lead restored"), (status = 404, description = "Not found or not deleted")),
    security(("bearer_auth" = []))
)]
pub async fn restore_lead(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let lead = crm_service
        .restore_lead(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(lead))
}

/// List soft-deleted leads (admin only)
#[utoipa::path(
    get, path = "/api/v1/crm/leads/deleted", tag = "CRM",
    responses((status = 200, description = "List of deleted leads")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_leads(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
) -> ApiResult<HttpResponse> {
    let leads = crm_service
        .list_deleted_leads(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(leads))
}

/// Permanently destroy a lead (admin only)
#[utoipa::path(
    delete, path = "/api/v1/crm/leads/{id}/destroy", tag = "CRM",
    params(("id" = i64, Path, description = "Lead ID")),
    responses((status = 204, description = "Lead permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_lead(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    crm_service
        .destroy_lead(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateLeadStatusRequest {
    pub status: LeadStatus,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct ConvertLeadRequest {
    pub customer_id: i64,
}
