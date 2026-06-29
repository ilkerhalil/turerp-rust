//! Campaign handlers

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::crm::model::{CampaignStatus, CreateCampaign};
use crate::domain::crm::service::CrmService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Create campaign (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/crm/campaigns", tag = "CRM",
    request_body = CreateCampaign,
    responses((status = 201, description = "Campaign created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_campaign(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    payload: web::Json<CreateCampaign>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match crm_service.create_campaign(create).await {
        Ok(campaign) => Ok(HttpResponse::Created().json(campaign)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all campaigns
#[utoipa::path(
    get, path = "/api/v1/crm/campaigns", tag = "CRM",
    responses((status = 200, description = "List of campaigns")),
    security(("bearer_auth" = []))
)]
pub async fn get_campaigns(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .get_campaigns_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get campaign by ID
#[utoipa::path(
    get, path = "/api/v1/crm/campaigns/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Campaign ID")),
    responses((status = 200, description = "Campaign found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_campaign(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service.get_campaign(*path, auth_user.0.tenant_id).await {
        Ok(campaign) => Ok(HttpResponse::Ok().json(campaign)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get campaigns by status
#[utoipa::path(
    get, path = "/api/v1/crm/campaigns/status/{status}", tag = "CRM",
    params(("status" = CampaignStatus, Path, description = "Campaign status")),
    responses((status = 200, description = "Campaigns by status")),
    security(("bearer_auth" = []))
)]
pub async fn get_campaigns_by_status(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<CampaignStatus>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .get_campaigns_by_status_paginated(
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

/// Update campaign status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/crm/campaigns/{id}/status", tag = "CRM",
    params(("id" = i64, Path, description = "Campaign ID")),
    request_body = UpdateCampaignStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_campaign_status(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateCampaignStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .update_campaign_status(*path, admin_user.0.tenant_id, payload.into_inner().status)
        .await
    {
        Ok(campaign) => Ok(HttpResponse::Ok().json(campaign)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft-delete a campaign (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/crm/campaigns/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Campaign ID")),
    responses((status = 200, description = "Campaign soft-deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_campaign(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let user_id: i64 = admin_user.0.user_id()?;
    match crm_service
        .soft_delete_campaign(*path, admin_user.0.tenant_id, user_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Campaign soft-deleted".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted campaign (admin only)
#[utoipa::path(
    put, path = "/api/v1/crm/campaigns/{id}/restore", tag = "CRM",
    params(("id" = i64, Path, description = "Campaign ID")),
    responses((status = 200, description = "Campaign restored"), (status = 404, description = "Not found or not deleted")),
    security(("bearer_auth" = []))
)]
pub async fn restore_campaign(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let campaign = crm_service
        .restore_campaign(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(campaign))
}

/// List soft-deleted campaigns (admin only)
#[utoipa::path(
    get, path = "/api/v1/crm/campaigns/deleted", tag = "CRM",
    responses((status = 200, description = "List of deleted campaigns")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_campaigns(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
) -> ApiResult<HttpResponse> {
    let campaigns = crm_service
        .list_deleted_campaigns(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(campaigns))
}

/// Permanently destroy a campaign (admin only)
#[utoipa::path(
    delete, path = "/api/v1/crm/campaigns/{id}/destroy", tag = "CRM",
    params(("id" = i64, Path, description = "Campaign ID")),
    responses((status = 204, description = "Campaign permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_campaign(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    crm_service
        .destroy_campaign(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateCampaignStatusRequest {
    pub status: CampaignStatus,
}
