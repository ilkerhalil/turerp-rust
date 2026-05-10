//! Forecasting API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::domain::forecasting::model::{ForecastRequest, ReorderRequest, StockAlertRequest};
use crate::domain::forecasting::service::ForecastingService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::AuthUser;

/// Generate demand forecast for a product
#[utoipa::path(
    post,
    path = "/api/v1/forecasting/demand",
    tag = "Forecasting",
    request_body = ForecastRequest,
    responses(
        (status = 200, description = "Demand forecast generated"),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Product not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn forecast_demand(
    auth_user: AuthUser,
    forecasting_service: web::Data<ForecastingService>,
    payload: web::Json<ForecastRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match forecasting_service
        .forecast_demand(auth_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(forecast) => Ok(HttpResponse::Ok().json(forecast)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get reorder suggestions for products
#[utoipa::path(
    post,
    path = "/api/v1/forecasting/reorder",
    tag = "Forecasting",
    request_body = ReorderRequest,
    responses(
        (status = 200, description = "Reorder suggestions generated"),
        (status = 400, description = "Invalid request")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_reorder_suggestions(
    auth_user: AuthUser,
    forecasting_service: web::Data<ForecastingService>,
    payload: web::Json<ReorderRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match forecasting_service
        .get_reorder_suggestions(auth_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(suggestions) => Ok(HttpResponse::Ok().json(suggestions)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get stock level alerts
#[utoipa::path(
    post,
    path = "/api/v1/forecasting/alerts",
    tag = "Forecasting",
    request_body = StockAlertRequest,
    responses(
        (status = 200, description = "Stock alerts generated"),
        (status = 400, description = "Invalid request")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_stock_alerts(
    auth_user: AuthUser,
    forecasting_service: web::Data<ForecastingService>,
    payload: web::Json<StockAlertRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match forecasting_service
        .get_stock_alerts(auth_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(alerts) => Ok(HttpResponse::Ok().json(alerts)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get comprehensive forecast report for all products
#[utoipa::path(
    get,
    path = "/api/v1/forecasting/report",
    tag = "Forecasting",
    params(
        ("warehouse_id" = Option<i64>, Query, description = "Optional warehouse filter")
    ),
    responses(
        (status = 200, description = "Forecast report generated"),
        (status = 400, description = "Invalid request")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_forecast_report(
    auth_user: AuthUser,
    forecasting_service: web::Data<ForecastingService>,
    query: web::Query<ForecastReportQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match forecasting_service
        .get_forecast_report(auth_user.0.tenant_id, query.warehouse_id)
        .await
    {
        Ok(report) => Ok(HttpResponse::Ok().json(report)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Query parameters for forecast report
#[derive(serde::Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct ForecastReportQuery {
    pub warehouse_id: Option<i64>,
}

/// Configure forecasting routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/v1/forecasting/demand").route(web::post().to(forecast_demand)))
        .service(
            web::resource("/v1/forecasting/reorder").route(web::post().to(get_reorder_suggestions)),
        )
        .service(web::resource("/v1/forecasting/alerts").route(web::post().to(get_stock_alerts)))
        .service(web::resource("/v1/forecasting/report").route(web::get().to(get_forecast_report)));
}
