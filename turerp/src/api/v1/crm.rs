//! CRM API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::domain::crm::model::{
    CampaignStatus, CreateCampaign, CreateLead, CreateOpportunity, CreateTicket, LeadStatus,
    OpportunityStatus, TicketStatus,
};
use crate::domain::crm::service::CrmService;
use crate::error::ApiResult;
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
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let lead = crm_service.create_lead(create).await?;
    Ok(HttpResponse::Created().json(lead))
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
) -> ApiResult<HttpResponse> {
    let leads = crm_service
        .get_leads_by_tenant(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(leads))
}

/// Get lead by ID
#[utoipa::path(
    get, path = "/api/v1/crm/leads/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Lead ID")),
    responses((status = 200, description = "Lead found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_lead(
    _auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let lead = crm_service.get_lead(*path).await?;
    Ok(HttpResponse::Ok().json(lead))
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
) -> ApiResult<HttpResponse> {
    let leads = crm_service
        .get_leads_by_status(auth_user.0.tenant_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(leads))
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
) -> ApiResult<HttpResponse> {
    let lead = crm_service
        .update_lead_status(*path, payload.into_inner().status)
        .await?;
    Ok(HttpResponse::Ok().json(lead))
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
) -> ApiResult<HttpResponse> {
    let lead = crm_service
        .convert_lead_to_customer(*path, payload.customer_id)
        .await?;
    Ok(HttpResponse::Ok().json(lead))
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
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let opportunity = crm_service.create_opportunity(create).await?;
    Ok(HttpResponse::Created().json(opportunity))
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
) -> ApiResult<HttpResponse> {
    let opportunities = crm_service
        .get_opportunities_by_tenant(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(opportunities))
}

/// Get opportunity by ID
#[utoipa::path(
    get, path = "/api/v1/crm/opportunities/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Opportunity ID")),
    responses((status = 200, description = "Opportunity found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_opportunity(
    _auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let opportunity = crm_service.get_opportunity(*path).await?;
    Ok(HttpResponse::Ok().json(opportunity))
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
) -> ApiResult<HttpResponse> {
    let opportunities = crm_service
        .get_opportunities_by_status(auth_user.0.tenant_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(opportunities))
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
) -> ApiResult<HttpResponse> {
    let opportunity = crm_service
        .update_opportunity_status(*path, payload.into_inner().status)
        .await?;
    Ok(HttpResponse::Ok().json(opportunity))
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
) -> ApiResult<HttpResponse> {
    let value = crm_service
        .get_sales_pipeline_value(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "pipeline_value": value })))
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
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let campaign = crm_service.create_campaign(create).await?;
    Ok(HttpResponse::Created().json(campaign))
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
) -> ApiResult<HttpResponse> {
    let campaigns = crm_service
        .get_campaigns_by_tenant(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(campaigns))
}

/// Get campaign by ID
#[utoipa::path(
    get, path = "/api/v1/crm/campaigns/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Campaign ID")),
    responses((status = 200, description = "Campaign found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_campaign(
    _auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let campaign = crm_service.get_campaign(*path).await?;
    Ok(HttpResponse::Ok().json(campaign))
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
) -> ApiResult<HttpResponse> {
    let campaigns = crm_service
        .get_campaigns_by_status(auth_user.0.tenant_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(campaigns))
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
) -> ApiResult<HttpResponse> {
    let campaign = crm_service
        .update_campaign_status(*path, payload.into_inner().status)
        .await?;
    Ok(HttpResponse::Ok().json(campaign))
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
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let ticket = crm_service.create_ticket(create).await?;
    Ok(HttpResponse::Created().json(ticket))
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
) -> ApiResult<HttpResponse> {
    let tickets = crm_service
        .get_tickets_by_tenant(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(tickets))
}

/// Get ticket by ID
#[utoipa::path(
    get, path = "/api/v1/crm/tickets/{id}", tag = "CRM",
    params(("id" = i64, Path, description = "Ticket ID")),
    responses((status = 200, description = "Ticket found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_ticket(
    _auth_user: AuthUser,
    crm_service: web::Data<CrmService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let ticket = crm_service.get_ticket(*path).await?;
    Ok(HttpResponse::Ok().json(ticket))
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
) -> ApiResult<HttpResponse> {
    let tickets = crm_service
        .get_tickets_by_status(auth_user.0.tenant_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(tickets))
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
) -> ApiResult<HttpResponse> {
    let ticket = crm_service
        .update_ticket_status(*path, payload.into_inner().status)
        .await?;
    Ok(HttpResponse::Ok().json(ticket))
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
) -> ApiResult<HttpResponse> {
    let ticket = crm_service.resolve_ticket(*path).await?;
    Ok(HttpResponse::Ok().json(ticket))
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
) -> ApiResult<HttpResponse> {
    let count = crm_service
        .get_open_tickets_count(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "open_tickets_count": count })))
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
    .service(web::resource("/v1/crm/leads/{id}").route(web::get().to(get_lead)))
    .service(web::resource("/v1/crm/leads/{id}/status").route(web::put().to(update_lead_status)))
    .service(web::resource("/v1/crm/leads/{id}/convert").route(web::post().to(convert_lead)))
    .service(
        web::resource("/v1/crm/opportunities")
            .route(web::get().to(get_opportunities))
            .route(web::post().to(create_opportunity)),
    )
    .service(
        web::resource("/v1/crm/opportunities/status/{status}")
            .route(web::get().to(get_opportunities_by_status)),
    )
    .service(web::resource("/v1/crm/opportunities/{id}").route(web::get().to(get_opportunity)))
    .service(
        web::resource("/v1/crm/opportunities/{id}/status")
            .route(web::put().to(update_opportunity_status)),
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
    .service(web::resource("/v1/crm/campaigns/{id}").route(web::get().to(get_campaign)))
    .service(
        web::resource("/v1/crm/campaigns/{id}/status").route(web::put().to(update_campaign_status)),
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
    .service(web::resource("/v1/crm/tickets/{id}").route(web::get().to(get_ticket)))
    .service(
        web::resource("/v1/crm/tickets/{id}/status").route(web::put().to(update_ticket_status)),
    )
    .service(web::resource("/v1/crm/tickets/{id}/resolve").route(web::post().to(resolve_ticket)));
}
