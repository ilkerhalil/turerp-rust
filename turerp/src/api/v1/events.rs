//! Event/Outbox API endpoints (v1)

use crate::common::{DomainEvent, EventBus};
use crate::error::ApiError;
use crate::middleware::AdminUser;
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Publish event request
#[derive(Debug, Deserialize, ToSchema)]
pub struct PublishEventRequest {
    pub name: String,
    pub payload: String,
}

/// Outbox event response
#[derive(Debug, Serialize, ToSchema)]
pub struct OutboxEventResponse {
    pub id: i64,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: i64,
    pub tenant_id: i64,
    pub status: String,
    pub attempts: u32,
    pub last_error: Option<String>,
    pub created_at: String,
}

/// Dead letter entry response
#[derive(Debug, Serialize, ToSchema)]
pub struct DeadLetterResponse {
    pub id: i64,
    pub original_event_id: i64,
    pub event_type: String,
    pub tenant_id: i64,
    pub error: String,
    pub attempts: u32,
    pub created_at: String,
}

/// Process outbox response
#[derive(Debug, Serialize, ToSchema)]
pub struct ProcessOutboxResponse {
    pub processed: u64,
}

/// Publish a custom domain event
#[utoipa::path(
    post,
    path = "/api/v1/events/publish",
    tag = "Events",
    request_body = PublishEventRequest,
    responses((status = 201, description = "Event published")),
    security(("bearer_auth" = []))
)]
pub async fn publish_event(
    admin_user: AdminUser,
    body: web::Json<PublishEventRequest>,
    bus: web::Data<dyn EventBus>,
) -> Result<HttpResponse, ApiError> {
    let event = DomainEvent::Custom {
        name: body.name.clone(),
        tenant_id: admin_user.0.tenant_id,
        payload: body.payload.clone(),
    };
    let id = bus.publish(event).await.map_err(ApiError::Internal)?;
    Ok(HttpResponse::Created().json(serde_json::json!({"id": id, "message": "Event published"})))
}

/// Process pending outbox events
#[utoipa::path(
    post,
    path = "/api/v1/events/outbox/process",
    tag = "Events",
    responses((status = 200, description = "Outbox processed", body = ProcessOutboxResponse)),
    security(("bearer_auth" = []))
)]
pub async fn process_outbox(
    _admin_user: AdminUser,
    bus: web::Data<dyn EventBus>,
) -> Result<HttpResponse, ApiError> {
    let count = bus.process_outbox(100).await.map_err(ApiError::Internal)?;
    Ok(HttpResponse::Ok().json(ProcessOutboxResponse { processed: count }))
}

/// Get pending outbox events
#[utoipa::path(
    get,
    path = "/api/v1/events/outbox/pending",
    tag = "Events",
    responses((status = 200, description = "List of pending events")),
    security(("bearer_auth" = []))
)]
pub async fn get_pending_events(
    _admin_user: AdminUser,
    bus: web::Data<dyn EventBus>,
) -> Result<HttpResponse, ApiError> {
    let events = bus.get_pending(50).await.map_err(ApiError::Internal)?;
    let responses: Vec<OutboxEventResponse> = events
        .iter()
        .map(|e| OutboxEventResponse {
            id: e.id,
            event_type: e.event.event_type().to_string(),
            aggregate_type: e.aggregate_type.clone(),
            aggregate_id: e.aggregate_id,
            tenant_id: e.tenant_id,
            status: format!("{:?}", e.status),
            attempts: e.attempts,
            last_error: e.last_error.clone(),
            created_at: e.created_at.to_rfc3339(),
        })
        .collect();
    Ok(HttpResponse::Ok().json(responses))
}

/// Get dead letter events
#[utoipa::path(
    get,
    path = "/api/v1/events/dead-letters",
    tag = "Events",
    responses((status = 200, description = "List of dead letter entries")),
    security(("bearer_auth" = []))
)]
pub async fn get_dead_letters(
    admin_user: AdminUser,
    bus: web::Data<dyn EventBus>,
) -> Result<HttpResponse, ApiError> {
    let entries = bus
        .get_dead_letters(admin_user.0.tenant_id)
        .await
        .map_err(ApiError::Internal)?;
    let responses: Vec<DeadLetterResponse> = entries
        .iter()
        .map(|e| DeadLetterResponse {
            id: e.id,
            original_event_id: e.aggregate_id,
            event_type: e.original_event.event_type().to_string(),
            tenant_id: e.tenant_id,
            error: e.error.clone(),
            attempts: e.original_attempts,
            created_at: e.dead_lettered_at.to_rfc3339(),
        })
        .collect();
    Ok(HttpResponse::Ok().json(responses))
}

/// Retry a dead letter event
#[utoipa::path(
    post,
    path = "/api/v1/events/dead-letters/{id}/retry",
    tag = "Events",
    params(("id" = i64, Path, description = "Dead letter entry ID")),
    responses((status = 200, description = "Event retried")),
    security(("bearer_auth" = []))
)]
pub async fn retry_dead_letter(
    _admin_user: AdminUser,
    path: web::Path<i64>,
    bus: web::Data<dyn EventBus>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    bus.retry_dead_letter(id)
        .await
        .map_err(ApiError::Internal)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Event queued for retry"})))
}

/// Configure event routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/events")
            .route("/publish", web::post().to(publish_event))
            .route("/outbox/process", web::post().to(process_outbox))
            .route("/outbox/pending", web::get().to(get_pending_events))
            .route("/dead-letters", web::get().to(get_dead_letters))
            .route(
                "/dead-letters/{id}/retry",
                web::post().to(retry_dead_letter),
            ),
    );
}
