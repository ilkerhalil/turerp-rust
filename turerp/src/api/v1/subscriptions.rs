//! Subscription / SaaS Billing API endpoints (v1)

use actix_web::{web, HttpResponse, ResponseError};
use chrono::{NaiveDate, Utc};

use crate::common::MessageResponse;
use crate::domain::subscription::model::{
    CalculateProrationRequest, CancelSubscriptionRequest, CreatePlan, CreateSubscription,
    RecordUsageRequest, UpdatePlan, UpdateSubscription,
};
use crate::domain::subscription::service::SubscriptionService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::{AdminUser, AuthUser};

// --- Plans (admin only) ---

/// Create a subscription plan (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/subscription-plans",
    tag = "Subscriptions",
    request_body = CreatePlan,
    responses(
        (status = 201, description = "Plan created successfully"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_plan(
    admin_user: AdminUser,
    subscription_service: web::Data<SubscriptionService>,
    payload: web::Json<CreatePlan>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    json_resp!(
        subscription_service.create_plan(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get subscription plan by ID (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/subscription-plans/{id}",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Plan ID")),
    responses(
        (status = 200, description = "Plan found"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Plan not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_plan(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        subscription_service.get_plan(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// List all subscription plans (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/subscription-plans",
    tag = "Subscriptions",
    responses(
        (status = 200, description = "List of plans"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_plans(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
) -> ApiResult<HttpResponse> {
    match subscription_service.list_plans(auth_user.0.tenant_id).await {
        Ok(plans) => Ok(HttpResponse::Ok().json(plans)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Update a subscription plan (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/subscription-plans/{id}",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Plan ID")),
    request_body = UpdatePlan,
    responses(
        (status = 200, description = "Plan updated"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Plan not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_plan(
    admin_user: AdminUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    payload: web::Json<UpdatePlan>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        subscription_service.update_plan(*path, admin_user.0.tenant_id, payload.into_inner()),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Delete a subscription plan (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/subscription-plans/{id}",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Plan ID")),
    responses(
        (status = 200, description = "Plan deleted", body = MessageResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Plan not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_plan(
    admin_user: AdminUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match subscription_service
        .delete_plan(*path, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "plan.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Subscriptions ---

/// Create a subscription (requires authentication)
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions",
    tag = "Subscriptions",
    request_body = CreateSubscription,
    responses(
        (status = 201, description = "Subscription created successfully"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_subscription(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    payload: web::Json<CreateSubscription>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = auth_user.0.tenant_id;
    json_resp!(
        subscription_service.create_subscription(create),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get subscription by ID (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/{id}",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    responses(
        (status = 200, description = "Subscription found"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Subscription not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_subscription(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        subscription_service.get_subscription(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// List all subscriptions (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions",
    tag = "Subscriptions",
    responses(
        (status = 200, description = "List of subscriptions"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_subscriptions(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
) -> ApiResult<HttpResponse> {
    match subscription_service
        .list_subscriptions(auth_user.0.tenant_id)
        .await
    {
        Ok(subs) => Ok(HttpResponse::Ok().json(subs)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Update a subscription (requires authentication)
#[utoipa::path(
    put,
    path = "/api/v1/subscriptions/{id}",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    request_body = UpdateSubscription,
    responses(
        (status = 200, description = "Subscription updated"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Subscription not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_subscription(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateSubscription>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        subscription_service.update_subscription(
            *path,
            auth_user.0.tenant_id,
            payload.into_inner()
        ),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Delete a subscription (requires authentication)
#[utoipa::path(
    delete,
    path = "/api/v1/subscriptions/{id}",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    responses(
        (status = 200, description = "Subscription deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Subscription not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_subscription(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match subscription_service
        .delete_subscription(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "subscription.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Special endpoints ---

/// Renew a subscription
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/{id}/renew",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    responses(
        (status = 200, description = "Subscription renewed"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Subscription not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn renew_subscription(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        subscription_service.renew_subscription(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get subscriptions due for billing
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/due-for-billing",
    tag = "Subscriptions",
    params(("date" = String, Query, description = "Billing cutoff date (YYYY-MM-DD)")),
    responses(
        (status = 200, description = "List of subscriptions due for billing"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn due_for_billing(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    query: web::Query<DueForBillingQuery>,
) -> ApiResult<HttpResponse> {
    let date = query.date.unwrap_or_else(|| Utc::now().date_naive());
    match subscription_service
        .due_for_billing(auth_user.0.tenant_id, date)
        .await
    {
        Ok(subs) => Ok(HttpResponse::Ok().json(subs)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Get invoices for a subscription
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/{id}/invoices",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    responses(
        (status = 200, description = "List of subscription invoices"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Subscription not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_subscription_invoices(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        subscription_service.get_invoices(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Calculate proration for plan change
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/{id}/proration",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    request_body = CalculateProrationRequest,
    responses(
        (status = 200, description = "Proration calculated"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Subscription or plan not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn calculate_proration(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    payload: web::Json<CalculateProrationRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        subscription_service.calculate_proration(
            *path,
            auth_user.0.tenant_id,
            payload.new_plan_id,
            payload.effective_date,
        ),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Handle dunning retry for failed payment
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/{id}/dunning/retry",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    request_body = DunningRetryRequest,
    responses(
        (status = 200, description = "Dunning retry processed"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn handle_dunning_retry(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    payload: web::Json<DunningRetryRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        subscription_service.handle_dunning(*path, auth_user.0.tenant_id, payload.invoice_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Get dunning status for a subscription
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/{id}/dunning",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    responses(
        (status = 200, description = "Dunning entries found"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_dunning_status(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    match subscription_service
        .get_dunning_status(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(entries) => Ok(HttpResponse::Ok().json(entries)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Convert trial subscription to paid
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/{id}/trial/convert",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    responses(
        (status = 200, description = "Trial converted to paid"),
        (status = 400, description = "Subscription is not in trial"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Subscription not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn convert_trial(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        subscription_service.process_trial_conversion(*path, auth_user.0.tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// List trial subscriptions
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/trials",
    tag = "Subscriptions",
    responses(
        (status = 200, description = "List of trial subscriptions"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_trial_subscriptions(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
) -> ApiResult<HttpResponse> {
    match subscription_service
        .list_trial_subscriptions(auth_user.0.tenant_id)
        .await
    {
        Ok(subs) => Ok(HttpResponse::Ok().json(subs)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Record usage for metered billing
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/{id}/usage",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    request_body = RecordUsageRequest,
    responses(
        (status = 201, description = "Usage recorded"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn record_usage(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    payload: web::Json<RecordUsageRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        subscription_service.record_usage(*path, auth_user.0.tenant_id, payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get usage records for a subscription
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/{id}/usage",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    responses(
        (status = 200, description = "List of usage records"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_usage_records(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    match subscription_service
        .get_usage_records(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(records) => Ok(HttpResponse::Ok().json(records)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Cancel a subscription
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/{id}/cancel",
    tag = "Subscriptions",
    params(("id" = i64, Path, description = "Subscription ID")),
    request_body = CancelSubscriptionRequest,
    responses(
        (status = 200, description = "Subscription cancelled"),
        (status = 400, description = "Already cancelled or expired"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Subscription not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn cancel_subscription(
    auth_user: AuthUser,
    subscription_service: web::Data<SubscriptionService>,
    path: web::Path<i64>,
    payload: web::Json<CancelSubscriptionRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    json_resp!(
        subscription_service.cancel_subscription(
            *path,
            auth_user.0.tenant_id,
            payload.into_inner()
        ),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Request body for dunning retry
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct DunningRetryRequest {
    pub invoice_id: i64,
}

/// Query parameters for due-for-billing endpoint
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct DueForBillingQuery {
    pub date: Option<NaiveDate>,
}

/// Configure subscription routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    // Plans
    cfg.service(
        web::resource("/v1/subscription-plans")
            .route(web::get().to(list_plans))
            .route(web::post().to(create_plan)),
    )
    .service(
        web::resource("/v1/subscription-plans/{id}")
            .route(web::get().to(get_plan))
            .route(web::put().to(update_plan))
            .route(web::delete().to(delete_plan)),
    )
    // Subscriptions
    .service(
        web::resource("/v1/subscriptions")
            .route(web::get().to(list_subscriptions))
            .route(web::post().to(create_subscription)),
    )
    .service(
        web::resource("/v1/subscriptions/due-for-billing").route(web::get().to(due_for_billing)),
    )
    .service(
        web::resource("/v1/subscriptions/{id}")
            .route(web::get().to(get_subscription))
            .route(web::put().to(update_subscription))
            .route(web::delete().to(delete_subscription)),
    )
    .service(
        web::resource("/v1/subscriptions/{id}/renew").route(web::post().to(renew_subscription)),
    )
    .service(
        web::resource("/v1/subscriptions/{id}/invoices")
            .route(web::get().to(get_subscription_invoices)),
    )
    .service(
        web::resource("/v1/subscriptions/{id}/proration")
            .route(web::post().to(calculate_proration)),
    )
    .service(
        web::resource("/v1/subscriptions/{id}/dunning")
            .route(web::get().to(get_dunning_status))
            .route(web::post().to(handle_dunning_retry)),
    )
    .service(
        web::resource("/v1/subscriptions/{id}/trial/convert").route(web::post().to(convert_trial)),
    )
    .service(
        web::resource("/v1/subscriptions/trials").route(web::get().to(list_trial_subscriptions)),
    )
    .service(
        web::resource("/v1/subscriptions/{id}/usage")
            .route(web::get().to(get_usage_records))
            .route(web::post().to(record_usage)),
    )
    .service(
        web::resource("/v1/subscriptions/{id}/cancel").route(web::post().to(cancel_subscription)),
    );
}
