//! CRM API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::crm::model::{
    CampaignStatus, CreateCampaign, CreateLead, CreateOpportunity, CreateTicket, LeadStatus,
    OpportunityStatus, TicketStatus,
};
use crate::domain::crm::service::CrmService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

// --- Leads ---

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
    _admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateLeadStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .update_lead_status(*path, payload.into_inner().status)
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
    _admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    payload: web::Json<ConvertLeadRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .convert_lead_to_customer(*path, payload.customer_id)
        .await
    {
        Ok(lead) => Ok(HttpResponse::Ok().json(lead)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Opportunities ---

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
    _admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateOpportunityStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .update_opportunity_status(*path, payload.into_inner().status)
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

// --- Campaigns ---

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
    _admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateCampaignStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .update_campaign_status(*path, payload.into_inner().status)
        .await
    {
        Ok(campaign) => Ok(HttpResponse::Ok().json(campaign)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Tickets ---

/// Create ticket
#[utoipa::path(
    post, path = "/api/v1/crm/tickets", tag = "CRM",
    request_body = CreateTicket,
    responses((status = 201, description = "Ticket created")),
    security(("bearer_auth" = []))
)]
pub async fn create_ticket(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    payload: web::Json<CreateTicket>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match crm_service.create_ticket(create).await {
        Ok(ticket) => Ok(HttpResponse::Created().json(ticket)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all tickets
#[utoipa::path(
    get, path = "/api/v1/crm/tickets", tag = "CRM",
    responses((status = 200, description = "List of tickets")),
    security(("bearer_auth" = []))
)]
pub async fn get_tickets(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .get_tickets_paginated(auth_user.0.tenant_id, query.page, query.per_page)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get ticket by ID
#[utoipa::path(
    get, path = "/api/v1/crm/tickets/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Ticket ID")),
    responses((status = 200, description = "Ticket found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_ticket(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service.get_ticket(*path, auth_user.0.tenant_id).await {
        Ok(ticket) => Ok(HttpResponse::Ok().json(ticket)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get tickets by status
#[utoipa::path(
    get, path = "/api/v1/crm/tickets/status/{status}", tag = "CRM",
    params(("status" = TicketStatus, Path, description = "Ticket status")),
    responses((status = 200, description = "Tickets by status")),
    security(("bearer_auth" = []))
)]
pub async fn get_tickets_by_status(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<TicketStatus>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .get_tickets_by_status_paginated(
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

/// Update ticket status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/crm/tickets/{id}/status", tag = "CRM",
    params(("id" = i64, Path, description = "Ticket ID")),
    request_body = UpdateTicketStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_ticket_status(
    _admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateTicketStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .update_ticket_status(*path, payload.into_inner().status)
        .await
    {
        Ok(ticket) => Ok(HttpResponse::Ok().json(ticket)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Resolve ticket (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/crm/tickets/{id}/resolve", tag = "CRM",
    params(("id" = i64, Path, description = "Ticket ID")),
    responses((status = 200, description = "Ticket resolved"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn resolve_ticket(
    _admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service.resolve_ticket(*path).await {
        Ok(ticket) => Ok(HttpResponse::Ok().json(ticket)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get open tickets count
#[utoipa::path(
    get, path = "/api/v1/crm/tickets/open-count", tag = "CRM",
    responses((status = 200, description = "Open tickets count")),
    security(("bearer_auth" = []))
)]
pub async fn get_open_tickets_count(
    auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match crm_service
        .get_open_tickets_count(auth_user.0.tenant_id)
        .await
    {
        Ok(count) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({ "open_tickets_count": count })))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Soft-delete / Restore / Destroy endpoints ---

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
    let user_id: i64 = admin_user.0.sub.parse().unwrap_or(0);
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
    let user_id: i64 = admin_user.0.sub.parse().unwrap_or(0);
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
    let user_id: i64 = admin_user.0.sub.parse().unwrap_or(0);
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

/// Soft-delete a ticket (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/crm/tickets/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Ticket ID")),
    responses((status = 200, description = "Ticket soft-deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_ticket(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let user_id: i64 = admin_user.0.sub.parse().unwrap_or(0);
    match crm_service
        .soft_delete_ticket(*path, admin_user.0.tenant_id, user_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Ticket soft-deleted".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted ticket (admin only)
#[utoipa::path(
    put, path = "/api/v1/crm/tickets/{id}/restore", tag = "CRM",
    params(("id" = i64, Path, description = "Ticket ID")),
    responses((status = 200, description = "Ticket restored"), (status = 404, description = "Not found or not deleted")),
    security(("bearer_auth" = []))
)]
pub async fn restore_ticket(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let ticket = crm_service
        .restore_ticket(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(ticket))
}

/// List soft-deleted tickets (admin only)
#[utoipa::path(
    get, path = "/api/v1/crm/tickets/deleted", tag = "CRM",
    responses((status = 200, description = "List of deleted tickets")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_tickets(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
) -> ApiResult<HttpResponse> {
    let tickets = crm_service
        .list_deleted_tickets(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(tickets))
}

/// Permanently destroy a ticket (admin only)
#[utoipa::path(
    delete, path = "/api/v1/crm/tickets/{id}/destroy", tag = "CRM",
    params(("id" = i64, Path, description = "Ticket ID")),
    responses((status = 204, description = "Ticket permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_ticket(
    admin_user: AdminUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    crm_service
        .destroy_ticket(*path, admin_user.0.tenant_id)
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

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateOpportunityStatusRequest {
    pub status: OpportunityStatus,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateCampaignStatusRequest {
    pub status: CampaignStatus,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateTicketStatusRequest {
    pub status: TicketStatus,
}

/// Configure CRM routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/crm/leads")
            .route(web::get().to(get_leads))
            .route(web::post().to(create_lead)),
    )
    .service(
        web::resource("/v1/crm/leads/status/{status}").route(web::get().to(get_leads_by_status)),
    )
    .service(web::resource("/v1/crm/leads/deleted").route(web::get().to(list_deleted_leads)))
    .service(
        web::resource("/v1/crm/leads/{id}")
            .route(web::get().to(get_lead))
            .route(web::delete().to(soft_delete_lead)),
    )
    .service(web::resource("/v1/crm/leads/{id}/status").route(web::put().to(update_lead_status)))
    .service(web::resource("/v1/crm/leads/{id}/convert").route(web::post().to(convert_lead)))
    .service(web::resource("/v1/crm/leads/{id}/restore").route(web::put().to(restore_lead)))
    .service(web::resource("/v1/crm/leads/{id}/destroy").route(web::delete().to(destroy_lead)))
    .service(
        web::resource("/v1/crm/opportunities")
            .route(web::get().to(get_opportunities))
            .route(web::post().to(create_opportunity)),
    )
    .service(
        web::resource("/v1/crm/opportunities/status/{status}")
            .route(web::get().to(get_opportunities_by_status)),
    )
    .service(
        web::resource("/v1/crm/opportunities/deleted")
            .route(web::get().to(list_deleted_opportunities)),
    )
    .service(
        web::resource("/v1/crm/opportunities/{id}")
            .route(web::get().to(get_opportunity))
            .route(web::delete().to(soft_delete_opportunity)),
    )
    .service(
        web::resource("/v1/crm/opportunities/{id}/status")
            .route(web::put().to(update_opportunity_status)),
    )
    .service(
        web::resource("/v1/crm/opportunities/{id}/restore")
            .route(web::put().to(restore_opportunity)),
    )
    .service(
        web::resource("/v1/crm/opportunities/{id}/destroy")
            .route(web::delete().to(destroy_opportunity)),
    )
    .service(web::resource("/v1/crm/pipeline-value").route(web::get().to(get_pipeline_value)))
    .service(
        web::resource("/v1/crm/campaigns")
            .route(web::get().to(get_campaigns))
            .route(web::post().to(create_campaign)),
    )
    .service(
        web::resource("/v1/crm/campaigns/status/{status}")
            .route(web::get().to(get_campaigns_by_status)),
    )
    .service(
        web::resource("/v1/crm/campaigns/deleted").route(web::get().to(list_deleted_campaigns)),
    )
    .service(
        web::resource("/v1/crm/campaigns/{id}")
            .route(web::get().to(get_campaign))
            .route(web::delete().to(soft_delete_campaign)),
    )
    .service(
        web::resource("/v1/crm/campaigns/{id}/status").route(web::put().to(update_campaign_status)),
    )
    .service(web::resource("/v1/crm/campaigns/{id}/restore").route(web::put().to(restore_campaign)))
    .service(
        web::resource("/v1/crm/campaigns/{id}/destroy").route(web::delete().to(destroy_campaign)),
    )
    .service(
        web::resource("/v1/crm/tickets")
            .route(web::get().to(get_tickets))
            .route(web::post().to(create_ticket)),
    )
    .service(
        web::resource("/v1/crm/tickets/status/{status}")
            .route(web::get().to(get_tickets_by_status)),
    )
    .service(
        web::resource("/v1/crm/tickets/open-count").route(web::get().to(get_open_tickets_count)),
    )
    .service(web::resource("/v1/crm/tickets/deleted").route(web::get().to(list_deleted_tickets)))
    .service(
        web::resource("/v1/crm/tickets/{id}")
            .route(web::get().to(get_ticket))
            .route(web::delete().to(soft_delete_ticket)),
    )
    .service(
        web::resource("/v1/crm/tickets/{id}/status").route(web::put().to(update_ticket_status)),
    )
    .service(web::resource("/v1/crm/tickets/{id}/resolve").route(web::post().to(resolve_ticket)))
    .service(web::resource("/v1/crm/tickets/{id}/restore").route(web::put().to(restore_ticket)))
    .service(web::resource("/v1/crm/tickets/{id}/destroy").route(web::delete().to(destroy_ticket)));
}
