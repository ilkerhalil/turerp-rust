//! Dashboard API endpoints (v1)

use actix_web::{web, HttpResponse, ResponseError};

use crate::domain::dashboard::model::{CreateWidgetConfig, DashboardFilter, KpiName};
use crate::domain::dashboard::service::DashboardService;
use crate::error::{ApiError, ApiResult};
use crate::middleware::{AdminUser, AuthUser};

/// Query parameters for chart endpoints
#[derive(Debug, serde::Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ChartPeriodQuery {
    #[serde(default = "default_period")]
    pub period: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_period() -> String {
    "month".to_string()
}

fn default_limit() -> i64 {
    5
}

/// Get all KPIs
#[utoipa::path(
    get,
    path = "/api/v1/dashboard/kpis",
    tag = "Dashboard",
    request_body = DashboardFilter,
    responses(
        (status = 200, description = "All KPIs", body = crate::domain::dashboard::model::KpiResponse),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_all_kpis(
    auth_user: AuthUser,
    dashboard_service: web::Data<DashboardService>,
    filter: web::Json<DashboardFilter>,
) -> ApiResult<HttpResponse> {
    match dashboard_service
        .get_all_kpis(auth_user.0.tenant_id, &filter)
        .await
    {
        Ok(kpis) => Ok(HttpResponse::Ok().json(kpis)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Get a single KPI by name
#[utoipa::path(
    get,
    path = "/api/v1/dashboard/kpis/{name}",
    tag = "Dashboard",
    params(
        ("name" = String, Path, description = "KPI name (revenue, profit, cash_flow, stock_value, customer_count)")
    ),
    request_body = DashboardFilter,
    responses(
        (status = 200, description = "Single KPI", body = crate::domain::dashboard::model::KpiWidget),
        (status = 400, description = "Invalid KPI name"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_single_kpi(
    auth_user: AuthUser,
    dashboard_service: web::Data<DashboardService>,
    path: web::Path<String>,
    filter: web::Json<DashboardFilter>,
) -> ApiResult<HttpResponse> {
    let name = match path.parse::<KpiName>() {
        Ok(n) => n,
        Err(e) => return Ok(ApiError::BadRequest(e).error_response()),
    };

    match dashboard_service
        .get_kpi_widget(auth_user.0.tenant_id, name, &filter)
        .await
    {
        Ok(kpi) => Ok(HttpResponse::Ok().json(kpi)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Get sales time-series chart data
#[utoipa::path(
    get,
    path = "/api/v1/dashboard/charts/sales",
    tag = "Dashboard",
    params(ChartPeriodQuery),
    responses(
        (status = 200, description = "Sales chart data", body = crate::domain::dashboard::model::ChartData),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_sales_chart(
    auth_user: AuthUser,
    dashboard_service: web::Data<DashboardService>,
    query: web::Query<ChartPeriodQuery>,
) -> ApiResult<HttpResponse> {
    match dashboard_service
        .get_sales_chart(auth_user.0.tenant_id, &query.period)
        .await
    {
        Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Get revenue by category pie chart data
#[utoipa::path(
    get,
    path = "/api/v1/dashboard/charts/revenue-by-category",
    tag = "Dashboard",
    responses(
        (status = 200, description = "Revenue by category chart data", body = crate::domain::dashboard::model::ChartData),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_revenue_by_category_chart(
    auth_user: AuthUser,
    dashboard_service: web::Data<DashboardService>,
) -> ApiResult<HttpResponse> {
    match dashboard_service
        .get_revenue_by_category_chart(auth_user.0.tenant_id)
        .await
    {
        Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Get top products bar chart data
#[utoipa::path(
    get,
    path = "/api/v1/dashboard/charts/top-products",
    tag = "Dashboard",
    params(ChartPeriodQuery),
    responses(
        (status = 200, description = "Top products chart data", body = crate::domain::dashboard::model::ChartData),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_top_products_chart(
    auth_user: AuthUser,
    dashboard_service: web::Data<DashboardService>,
    query: web::Query<ChartPeriodQuery>,
) -> ApiResult<HttpResponse> {
    match dashboard_service
        .get_top_products_chart(auth_user.0.tenant_id, query.limit)
        .await
    {
        Ok(chart) => Ok(HttpResponse::Ok().json(chart)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Save a widget configuration (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/dashboard/widgets",
    tag = "Dashboard",
    request_body = CreateWidgetConfig,
    responses(
        (status = 201, description = "Widget created", body = crate::domain::dashboard::model::DashboardWidgetConfig),
        (status = 403, description = "Forbidden")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_widget(
    admin_user: AdminUser,
    dashboard_service: web::Data<DashboardService>,
    payload: web::Json<CreateWidgetConfig>,
) -> ApiResult<HttpResponse> {
    match dashboard_service
        .save_widget(admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(widget) => Ok(HttpResponse::Created().json(widget)),
        Err(e) => Ok(e.error_response()),
    }
}

/// List saved widget configurations
#[utoipa::path(
    get,
    path = "/api/v1/dashboard/widgets",
    tag = "Dashboard",
    responses(
        (status = 200, description = "List of widgets", body = Vec<crate::domain::dashboard::model::DashboardWidgetConfig>),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_widgets(
    auth_user: AuthUser,
    dashboard_service: web::Data<DashboardService>,
) -> ApiResult<HttpResponse> {
    match dashboard_service.list_widgets(auth_user.0.tenant_id).await {
        Ok(widgets) => Ok(HttpResponse::Ok().json(widgets)),
        Err(e) => Ok(e.error_response()),
    }
}

/// Delete a widget configuration (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/dashboard/widgets/{id}",
    tag = "Dashboard",
    params(("id" = i64, Path, description = "Widget ID")),
    responses(
        (status = 204, description = "Widget deleted"),
        (status = 404, description = "Widget not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_widget(
    admin_user: AdminUser,
    dashboard_service: web::Data<DashboardService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    match dashboard_service
        .delete_widget(admin_user.0.tenant_id, *path)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.error_response()),
    }
}

/// Configure dashboard routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/v1/dashboard/kpis").route(web::get().to(get_all_kpis)))
        .service(web::resource("/v1/dashboard/kpis/{name}").route(web::get().to(get_single_kpi)))
        .service(web::resource("/v1/dashboard/charts/sales").route(web::get().to(get_sales_chart)))
        .service(
            web::resource("/v1/dashboard/charts/revenue-by-category")
                .route(web::get().to(get_revenue_by_category_chart)),
        )
        .service(
            web::resource("/v1/dashboard/charts/top-products")
                .route(web::get().to(get_top_products_chart)),
        )
        .service(
            web::resource("/v1/dashboard/widgets")
                .route(web::get().to(list_widgets))
                .route(web::post().to(create_widget)),
        )
        .service(
            web::resource("/v1/dashboard/widgets/{id}").route(web::delete().to(delete_widget)),
        );
}
