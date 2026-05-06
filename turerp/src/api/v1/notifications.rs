//! Notification API endpoints (v1)

use crate::common::{
    MessageResponse, NotificationChannel, NotificationPriority, NotificationRequest,
    NotificationService,
};
#[allow(unused_imports)]
use crate::domain::notification::model::{NotificationPreferenceResponse, NotificationResponse};
use crate::error::ApiError;
use crate::middleware::AdminUser;
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Send notification request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SendNotificationRequest {
    pub user_id: Option<i64>,
    pub channel: String,
    pub priority: Option<String>,
    pub template_key: String,
    pub template_vars: serde_json::Value,
    pub recipient: String,
}

/// In-app notification response
#[derive(Debug, Serialize, ToSchema)]
pub struct InAppNotificationResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub read: bool,
    pub created_at: String,
    pub link: Option<String>,
}

/// Unread count response
#[derive(Debug, Serialize, ToSchema)]
pub struct UnreadCountResponse {
    pub count: u64,
}

/// Mark read response
#[derive(Debug, Serialize, ToSchema)]
pub struct MarkReadResponse {
    pub marked: u64,
}

/// History query params
#[derive(Debug, Deserialize)]
pub struct HistoryQueryParams {
    pub channel: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default = "default_offset")]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

fn default_offset() -> i64 {
    0
}

/// Preference update request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePreferenceRequest {
    pub channel: String,
    pub notification_type: String,
    pub enabled: bool,
}

/// Send a notification
#[utoipa::path(
    post,
    path = "/api/v1/notifications/send",
    tag = "Notifications",
    request_body = SendNotificationRequest,
    responses(
        (status = 201, description = "Notification sent", body = NotificationResponse),
        (status = 400, description = "Invalid request"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn send_notification(
    admin_user: AdminUser,
    body: web::Json<SendNotificationRequest>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let channel = match body.channel.to_lowercase().as_str() {
        "email" => NotificationChannel::Email,
        "sms" => NotificationChannel::Sms,
        "inapp" => NotificationChannel::InApp,
        _ => {
            return Err(ApiError::Validation(
                "Invalid channel. Use: email, sms, inapp".to_string(),
            ))
        }
    };
    let priority = match body.priority.as_deref() {
        Some("low") => NotificationPriority::Low,
        Some("high") => NotificationPriority::High,
        Some("urgent") => NotificationPriority::Urgent,
        _ => NotificationPriority::Normal,
    };
    let request = NotificationRequest {
        tenant_id: admin_user.0.tenant_id,
        user_id: body.user_id,
        channel,
        priority,
        template_key: body.template_key.clone(),
        template_vars: body.template_vars.clone(),
        recipient: body.recipient.clone(),
    };
    let notification = service.send(request).await?;
    Ok(HttpResponse::Created().json(notification))
}

/// Get notification history
#[utoipa::path(
    get,
    path = "/api/v1/notifications/history",
    tag = "Notifications",
    params(
        ("channel" = Option<String>, Query, description = "Filter by channel: email, sms, inapp"),
        ("limit" = Option<i64>, Query, description = "Page size (default 20)"),
        ("offset" = Option<i64>, Query, description = "Offset (default 0)")
    ),
    responses((status = 200, description = "Notification history", body = crate::common::PaginatedResult<NotificationResponse>)),
    security(("bearer_auth" = []))
)]
pub async fn get_notification_history(
    admin_user: AdminUser,
    query: web::Query<HistoryQueryParams>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let channel = query
        .channel
        .as_ref()
        .map(|c| c.parse::<NotificationChannel>())
        .transpose()
        .map_err(|e: String| ApiError::Validation(e))?;

    let history = service
        .get_history(
            admin_user.0.tenant_id,
            Some(admin_user.0.user_id()?),
            channel,
            query.limit,
            query.offset,
        )
        .await?;

    Ok(HttpResponse::Ok().json(history))
}

/// Get in-app notifications for current user
#[utoipa::path(
    get,
    path = "/api/v1/notifications/in-app",
    tag = "Notifications",
    params(("unread_only" = Option<bool>, Query, description = "Only unread notifications")),
    responses((status = 200, description = "List of in-app notifications", body = Vec<InAppNotificationResponse>)),
    security(("bearer_auth" = []))
)]
pub async fn get_in_app_notifications(
    admin_user: AdminUser,
    query: web::Query<InAppQueryParams>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let notifications = service
        .get_in_app_notifications(
            admin_user.0.tenant_id,
            admin_user.0.user_id()?,
            query.unread_only.unwrap_or(false),
        )
        .await?;
    let responses: Vec<InAppNotificationResponse> = notifications
        .iter()
        .map(|n| InAppNotificationResponse {
            id: n.id,
            tenant_id: n.tenant_id,
            user_id: n.user_id,
            title: n.title.clone(),
            message: n.message.clone(),
            notification_type: n.notification_type.clone(),
            read: n.read,
            created_at: n.created_at.to_rfc3339(),
            link: n.link.clone(),
        })
        .collect();
    Ok(HttpResponse::Ok().json(responses))
}

/// Get unread notification count
#[utoipa::path(
    get,
    path = "/api/v1/notifications/unread-count",
    tag = "Notifications",
    responses((status = 200, description = "Unread count", body = UnreadCountResponse)),
    security(("bearer_auth" = []))
)]
pub async fn get_unread_count(
    admin_user: AdminUser,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let count = service
        .unread_count(admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await?;
    Ok(HttpResponse::Ok().json(UnreadCountResponse { count }))
}

/// Mark a notification as read
#[utoipa::path(
    put,
    path = "/api/v1/notifications/{id}/read",
    tag = "Notifications",
    params(("id" = i64, Path, description = "Notification ID")),
    responses((status = 200, description = "Notification marked as read", body = MessageResponse)),
    security(("bearer_auth" = []))
)]
pub async fn mark_notification_read(
    admin_user: AdminUser,
    path: web::Path<i64>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    service.mark_as_read(id, admin_user.0.tenant_id).await?;
    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Notification marked as read".to_string(),
    }))
}

/// Mark all notifications as read
#[utoipa::path(
    put,
    path = "/api/v1/notifications/read-all",
    tag = "Notifications",
    responses((status = 200, description = "All notifications marked as read", body = MarkReadResponse)),
    security(("bearer_auth" = []))
)]
pub async fn mark_all_read(
    admin_user: AdminUser,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let count = service
        .mark_all_as_read(admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await?;
    Ok(HttpResponse::Ok().json(MarkReadResponse { marked: count }))
}

/// Retry a failed notification
#[utoipa::path(
    post,
    path = "/api/v1/notifications/{id}/retry",
    tag = "Notifications",
    params(("id" = i64, Path, description = "Notification ID")),
    responses((status = 200, description = "Notification retried", body = MessageResponse)),
    security(("bearer_auth" = []))
)]
pub async fn retry_notification(
    admin_user: AdminUser,
    path: web::Path<i64>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    service.retry(id, admin_user.0.tenant_id).await?;
    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Notification queued for retry".to_string(),
    }))
}

/// Soft delete a notification
#[utoipa::path(
    delete,
    path = "/api/v1/notifications/{id}",
    tag = "Notifications",
    params(("id" = i64, Path, description = "Notification ID")),
    responses(
        (status = 204, description = "Notification soft deleted"),
        (status = 404, description = "Notification not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_notification(
    admin_user: AdminUser,
    path: web::Path<i64>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let deleted_by = admin_user
        .0
        .sub
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("Invalid user ID in token".to_string()))?;
    service
        .soft_delete(id, admin_user.0.tenant_id, deleted_by)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Restore a soft-deleted notification
#[utoipa::path(
    post,
    path = "/api/v1/notifications/{id}/restore",
    tag = "Notifications",
    params(("id" = i64, Path, description = "Notification ID")),
    responses(
        (status = 200, description = "Notification restored", body = MessageResponse),
        (status = 404, description = "Deleted notification not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn restore_notification(
    admin_user: AdminUser,
    path: web::Path<i64>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    service.restore(id, admin_user.0.tenant_id).await?;
    Ok(HttpResponse::Ok().json(MessageResponse {
        message: "Notification restored".to_string(),
    }))
}

/// List deleted notifications
#[utoipa::path(
    get,
    path = "/api/v1/notifications/deleted",
    tag = "Notifications",
    responses((status = 200, description = "List of deleted notifications", body = Vec<NotificationResponse>)),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_notifications(
    admin_user: AdminUser,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let notifications = service.find_deleted(admin_user.0.tenant_id).await?;
    Ok(HttpResponse::Ok().json(notifications))
}

/// Permanently destroy a soft-deleted notification
#[utoipa::path(
    delete,
    path = "/api/v1/notifications/{id}/destroy",
    tag = "Notifications",
    params(("id" = i64, Path, description = "Notification ID")),
    responses(
        (status = 204, description = "Notification permanently destroyed"),
        (status = 404, description = "Deleted notification not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn destroy_notification(
    admin_user: AdminUser,
    path: web::Path<i64>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    service.destroy(id, admin_user.0.tenant_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Get notification preferences
#[utoipa::path(
    get,
    path = "/api/v1/notifications/preferences",
    tag = "Notifications",
    responses((status = 200, description = "User notification preferences", body = Vec<NotificationPreferenceResponse>)),
    security(("bearer_auth" = []))
)]
pub async fn get_preferences(
    admin_user: AdminUser,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let prefs = service
        .get_preferences(admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await?;
    Ok(HttpResponse::Ok().json(prefs))
}

/// Update notification preferences
#[utoipa::path(
    put,
    path = "/api/v1/notifications/preferences",
    tag = "Notifications",
    request_body = Vec<UpdatePreferenceRequest>,
    responses((status = 200, description = "Preferences updated", body = Vec<NotificationPreferenceResponse>)),
    security(("bearer_auth" = []))
)]
pub async fn update_preferences(
    admin_user: AdminUser,
    body: web::Json<Vec<UpdatePreferenceRequest>>,
    service: web::Data<dyn NotificationService>,
) -> Result<HttpResponse, ApiError> {
    let prefs = body
        .into_inner()
        .into_iter()
        .map(|p| crate::common::UpdatePreference {
            channel: p.channel,
            notification_type: p.notification_type,
            enabled: p.enabled,
        })
        .collect();

    let updated = service
        .update_preferences(admin_user.0.tenant_id, admin_user.0.user_id()?, prefs)
        .await?;
    Ok(HttpResponse::Ok().json(updated))
}

#[derive(Debug, Deserialize)]
pub struct InAppQueryParams {
    unread_only: Option<bool>,
}

/// Configure notification routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/notifications")
            .route("/send", web::post().to(send_notification))
            .route("/history", web::get().to(get_notification_history))
            .route("/in-app", web::get().to(get_in_app_notifications))
            .route("/unread-count", web::get().to(get_unread_count))
            .route("/read-all", web::put().to(mark_all_read))
            .route("/deleted", web::get().to(list_deleted_notifications))
            .route("/preferences", web::get().to(get_preferences))
            .route("/preferences", web::put().to(update_preferences))
            .route("/{id}", web::delete().to(soft_delete_notification))
            .route("/{id}/read", web::put().to(mark_notification_read))
            .route("/{id}/retry", web::post().to(retry_notification))
            .route("/{id}/restore", web::post().to(restore_notification))
            .route("/{id}/destroy", web::delete().to(destroy_notification)),
    );
}
