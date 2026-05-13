//! Push notification API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::notifications::{NotificationService, PushMessage};
use crate::error::ApiError;
use crate::middleware::auth::{AdminUser, AuthUser};

/// Register push token request
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterPushTokenRequest {
    pub device_type: String,
    pub token: String,
}

/// Unregister push token request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UnregisterPushTokenRequest {
    pub device_type: String,
}

/// Send push notification request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SendPushRequest {
    pub user_id: i64,
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
    pub badge: Option<i32>,
    pub sound: Option<String>,
}

/// Broadcast push notification request
#[derive(Debug, Deserialize, ToSchema)]
pub struct BroadcastPushRequest {
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
    pub badge: Option<i32>,
    pub sound: Option<String>,
}

/// Push token response
#[derive(Debug, Serialize, ToSchema)]
pub struct PushTokenResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub device_type: String,
    pub token: String,
    pub is_active: bool,
    pub created_at: String,
}

/// Sent to response for broadcast
#[derive(Debug, Serialize, ToSchema)]
pub struct BroadcastResponse {
    pub sent_to_count: usize,
    pub user_ids: Vec<i64>,
}

/// Register a push token for the current user
#[utoipa::path(
    post,
    path = "/api/v1/notifications/push/register",
    tag = "Push Notifications",
    request_body = RegisterPushTokenRequest,
    responses(
        (status = 201, description = "Push token registered", body = PushTokenResponse),
        (status = 400, description = "Invalid request"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn register_push_token(
    auth_user: AuthUser,
    body: web::Json<RegisterPushTokenRequest>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user.0.user_id()?;
    let result = service
        .register_push_token(
            auth_user.0.tenant_id,
            user_id,
            body.device_type.clone(),
            body.token.clone(),
        )
        .await?;
    Ok(HttpResponse::Created().json(PushTokenResponse {
        id: result.id,
        tenant_id: result.tenant_id,
        user_id: result.user_id,
        device_type: result.device_type.to_string(),
        token: result.token,
        is_active: result.is_active,
        created_at: result.created_at.to_rfc3339(),
    }))
}

/// Unregister a push token for the current user
#[utoipa::path(
    delete,
    path = "/api/v1/notifications/push/unregister",
    tag = "Push Notifications",
    request_body = UnregisterPushTokenRequest,
    responses(
        (status = 204, description = "Push token unregistered"),
        (status = 404, description = "Token not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn unregister_push_token(
    auth_user: AuthUser,
    body: web::Json<UnregisterPushTokenRequest>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    service
        .unregister_push_token(
            auth_user.0.tenant_id,
            auth_user.0.user_id()?,
            body.device_type.clone(),
        )
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Send a push notification to a specific user (admin)
#[utoipa::path(
    post,
    path = "/api/v1/notifications/push/send",
    tag = "Push Notifications",
    request_body = SendPushRequest,
    responses(
        (status = 200, description = "Push notification sent"),
        (status = 404, description = "User has no active tokens"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn send_push(
    _admin: AdminUser,
    body: web::Json<SendPushRequest>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let message = PushMessage {
        title: body.title.clone(),
        body: body.body.clone(),
        data: body.data.clone(),
        badge: body.badge,
        sound: body.sound.clone(),
    };
    service
        .send_push(_admin.0.tenant_id, body.user_id, message)
        .await?;
    Ok(HttpResponse::Ok().finish())
}

/// Broadcast a push notification to all active users (admin)
#[utoipa::path(
    post,
    path = "/api/v1/notifications/push/broadcast",
    tag = "Push Notifications",
    request_body = BroadcastPushRequest,
    responses(
        (status = 200, description = "Push broadcast sent", body = BroadcastResponse),
    ),
    security(("bearer_auth" = []))
)]
pub async fn broadcast_push(
    _admin: AdminUser,
    body: web::Json<BroadcastPushRequest>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let message = PushMessage {
        title: body.title.clone(),
        body: body.body.clone(),
        data: body.data.clone(),
        badge: body.badge,
        sound: body.sound.clone(),
    };
    let user_ids = service.broadcast_push(_admin.0.tenant_id, message).await?;
    Ok(HttpResponse::Ok().json(BroadcastResponse {
        sent_to_count: user_ids.len(),
        user_ids,
    }))
}

/// List push tokens for the current user
#[utoipa::path(
    get,
    path = "/api/v1/notifications/push/tokens",
    tag = "Push Notifications",
    responses(
        (status = 200, description = "List of push tokens", body = Vec<PushTokenResponse>),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_user_push_tokens(
    auth_user: AuthUser,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let tokens = service
        .get_user_push_tokens(auth_user.0.tenant_id, auth_user.0.user_id()?)
        .await?;
    let responses: Vec<PushTokenResponse> = tokens
        .into_iter()
        .map(|t| PushTokenResponse {
            id: t.id,
            tenant_id: t.tenant_id,
            user_id: t.user_id,
            device_type: t.device_type.to_string(),
            token: t.token,
            is_active: t.is_active,
            created_at: t.created_at.to_rfc3339(),
        })
        .collect();
    Ok(HttpResponse::Ok().json(responses))
}

/// Configure push notification routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/notifications/push")
            .route("/register", web::post().to(register_push_token))
            .route("/unregister", web::delete().to(unregister_push_token))
            .route("/send", web::post().to(send_push))
            .route("/broadcast", web::post().to(broadcast_push))
            .route("/tokens", web::get().to(get_user_push_tokens)),
    );
}
