//! Ticket handlers

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::crm::model::{CreateTicket, TicketStatus};
use crate::domain::crm::service::CrmService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

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
    json_resp!(
        crm_service.create_ticket(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
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
    json_resp!(
        crm_service.get_tickets_paginated(auth_user.0.tenant_id, query.page, query.per_page),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    json_resp!(
        crm_service.get_ticket(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    json_resp!(
        crm_service.get_tickets_by_status_paginated(
            auth_user.0.tenant_id,
            path.into_inner(),
            query.page,
            query.per_page,
        ),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    json_resp!(
        crm_service.update_ticket_status(*path, payload.into_inner().status),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    json_resp!(
        crm_service.resolve_ticket(*path),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
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
    let user_id: i64 = admin_user.0.user_id()?;
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
pub struct UpdateTicketStatusRequest {
    pub status: TicketStatus,
}
