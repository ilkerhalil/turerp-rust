//! Webhook API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::webhook::model::{CreateWebhook, UpdateWebhook, WebhookResponse};
use crate::domain::webhook::service::WebhookService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Create a webhook endpoint (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/webhooks",
    tag = "Webhooks",
    request_body = CreateWebhook,
    responses(
        (status = 201, description = "Webhook created successfully", body = WebhookResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin access required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_webhook(
    admin_user: AdminUser,
    service: web::Data<WebhookService>,
    payload: web::Json<CreateWebhook>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .create_webhook(admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(wh) => {
            let resp: WebhookResponse = wh.into();
            Ok(HttpResponse::Created().json(resp))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List webhooks endpoint (authenticated, paginated)
#[utoipa::path(
    get,
    path = "/api/v1/webhooks",
    tag = "Webhooks",
    params(PaginationParams),
    responses(
        (status = 200, description = "List of webhooks", body = Vec<WebhookResponse>),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_webhooks(
    _auth_user: AuthUser,
    service: web::Data<WebhookService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.list_webhooks(_auth_user.0.tenant_id).await {
        Ok(list) => {
            let resp: Vec<WebhookResponse> = list.into_iter().map(Into::into).collect();
            Ok(HttpResponse::Ok().json(resp))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a webhook by ID (authenticated)
#[utoipa::path(
    get,
    path = "/api/v1/webhooks/{id}",
    tag = "Webhooks",
    params(("id" = i64, Path, description = "Webhook ID")),
    responses(
        (status = 200, description = "Webhook found", body = WebhookResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Webhook not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_webhook(
    _auth_user: AuthUser,
    service: web::Data<WebhookService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.get_webhook(*path, _auth_user.0.tenant_id).await {
        Ok(Some(wh)) => {
            let resp: WebhookResponse = wh.into();
            Ok(HttpResponse::Ok().json(resp))
        }
        Ok(None) => Ok(ApiError::NotFound(format!("Webhook {} not found", *path))
            .to_http_response(i18n, locale.as_str())),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update a webhook (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/webhooks/{id}",
    tag = "Webhooks",
    params(("id" = i64, Path, description = "Webhook ID")),
    request_body = UpdateWebhook,
    responses(
        (status = 200, description = "Webhook updated", body = WebhookResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin access required"),
        (status = 404, description = "Webhook not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_webhook(
    admin_user: AdminUser,
    service: web::Data<WebhookService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateWebhook>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .update_webhook(*path, admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(wh) => {
            let resp: WebhookResponse = wh.into();
            Ok(HttpResponse::Ok().json(resp))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete a webhook (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/webhooks/{id}",
    tag = "Webhooks",
    params(("id" = i64, Path, description = "Webhook ID")),
    responses(
        (status = 200, description = "Webhook deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin access required"),
        (status = 404, description = "Webhook not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_webhook(
    admin_user: AdminUser,
    service: web::Data<WebhookService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .delete_webhook(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "webhook.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Test a webhook by sending a synthetic event (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/{id}/test",
    tag = "Webhooks",
    params(("id" = i64, Path, description = "Webhook ID")),
    responses(
        (status = 200, description = "Test event sent", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin access required"),
        (status = 404, description = "Webhook not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn test_webhook(
    admin_user: AdminUser,
    service: web::Data<WebhookService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = admin_user.0.tenant_id;

    match service.get_webhook(*path, tenant_id).await {
        Ok(Some(_)) => {
            let event = crate::common::events::DomainEvent::Custom {
                name: "webhook.test".to_string(),
                tenant_id,
                payload: serde_json::json!({
                    "message": "This is a test event",
                    "webhook_id": *path,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
                .to_string(),
            };
            match service.trigger(&event).await {
                Ok(()) => {
                    let msg = i18n.t(locale.as_str(), "webhook.test_sent");
                    Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
                }
                Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
            }
        }
        Ok(None) => Ok(ApiError::NotFound(format!("Webhook {} not found", *path))
            .to_http_response(i18n, locale.as_str())),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List deliveries for a webhook (authenticated, paginated)
#[utoipa::path(
    get,
    path = "/api/v1/webhooks/{id}/deliveries",
    tag = "Webhooks",
    params(
        ("id" = i64, Path, description = "Webhook ID"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Webhook deliveries", body = crate::common::PaginatedResult< crate::domain::webhook::model::WebhookDeliveryResponse >),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Webhook not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_deliveries(
    _auth_user: AuthUser,
    service: web::Data<WebhookService>,
    path: web::Path<i64>,
    pagination: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    match service
        .list_deliveries(
            *path,
            _auth_user.0.tenant_id,
            pagination.page as i64,
            pagination.per_page as i64,
        )
        .await
    {
        Ok(result) => {
            let mapped: crate::common::PaginatedResult<
                crate::domain::webhook::model::WebhookDeliveryResponse,
            > = result.map(Into::into);
            Ok(HttpResponse::Ok().json(mapped))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Retry a failed webhook delivery (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/deliveries/{id}/retry",
    tag = "Webhooks",
    params(("id" = i64, Path, description = "Delivery ID")),
    responses(
        (status = 200, description = "Delivery retry initiated", body = crate::domain::webhook::model::WebhookDeliveryResponse),
        (status = 400, description = "Max retries exceeded"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin access required"),
        (status = 404, description = "Delivery not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn retry_delivery(
    admin_user: AdminUser,
    service: web::Data<WebhookService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.retry_delivery(*path, admin_user.0.tenant_id).await {
        Ok(delivery) => {
            let resp: crate::domain::webhook::model::WebhookDeliveryResponse = delivery.into();
            Ok(HttpResponse::Ok().json(resp))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure webhook routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/webhooks")
            .route(web::get().to(list_webhooks))
            .route(web::post().to(create_webhook)),
    )
    .service(
        web::resource("/v1/webhooks/{id}")
            .route(web::get().to(get_webhook))
            .route(web::put().to(update_webhook))
            .route(web::delete().to(delete_webhook)),
    )
    .service(web::resource("/v1/webhooks/{id}/test").route(web::post().to(test_webhook)))
    .service(web::resource("/v1/webhooks/{id}/deliveries").route(web::get().to(list_deliveries)))
    .service(
        web::resource("/v1/webhooks/deliveries/{id}/retry").route(web::post().to(retry_delivery)),
    );
}
