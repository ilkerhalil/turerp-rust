//! Chart of Accounts API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::common::pagination::{default_page, default_per_page, PaginationParams};
use crate::common::MessageResponse;
use crate::domain::chart_of_accounts::model::{
    AccountGroup, CreateChartAccount, UpdateChartAccount,
};
use crate::domain::chart_of_accounts::service::ChartOfAccountsService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Query parameters for listing chart accounts
#[derive(Debug, Deserialize)]
pub struct ListChartAccountsQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    pub group: Option<AccountGroup>,
}

impl From<ListChartAccountsQuery> for PaginationParams {
    fn from(q: ListChartAccountsQuery) -> Self {
        Self {
            page: q.page,
            per_page: q.per_page,
        }
    }
}

/// Create chart account (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/chart-of-accounts", tag = "Chart of Accounts",
    request_body = CreateChartAccount,
    responses((status = 201, description = "Chart account created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_chart_account(
    admin_user: AdminUser,
    chart_of_accounts_service: web::Data<ChartOfAccountsService>,
    payload: web::Json<CreateChartAccount>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match chart_of_accounts_service
        .create_account(create, admin_user.0.tenant_id)
        .await
    {
        Ok(account) => Ok(HttpResponse::Created().json(account)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List chart accounts (paginated, optional group filter)
#[utoipa::path(
    get, path = "/api/v1/chart-of-accounts", tag = "Chart of Accounts",
    params(
        PaginationParams,
        ("group" = Option<AccountGroup>, Query, description = "Filter by account group"),
    ),
    responses((status = 200, description = "List of chart accounts")),
    security(("bearer_auth" = []))
)]
pub async fn list_chart_accounts(
    auth_user: AuthUser,
    chart_of_accounts_service: web::Data<ChartOfAccountsService>,
    query: web::Query<ListChartAccountsQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let q = query.into_inner();
    match chart_of_accounts_service
        .list_accounts(auth_user.0.tenant_id, q.group, q.into())
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get chart account by code
#[utoipa::path(
    get, path = "/api/v1/chart-of-accounts/{code}", tag = "Chart of Accounts",
    params(("code" = String, Path, description = "Account code")),
    responses((status = 200, description = "Chart account found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_chart_account_by_code(
    auth_user: AuthUser,
    chart_of_accounts_service: web::Data<ChartOfAccountsService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let code = path.into_inner();
    // Look up by code: find the account with matching code
    match chart_of_accounts_service
        .get_account_by_code(&code, auth_user.0.tenant_id)
        .await
    {
        Ok(account) => Ok(HttpResponse::Ok().json(account)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update chart account (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/chart-of-accounts/{code}", tag = "Chart of Accounts",
    params(("code" = String, Path, description = "Account code")),
    request_body = UpdateChartAccount,
    responses((status = 200, description = "Chart account updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_chart_account(
    admin_user: AdminUser,
    chart_of_accounts_service: web::Data<ChartOfAccountsService>,
    path: web::Path<String>,
    payload: web::Json<UpdateChartAccount>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let code = path.into_inner();
    let update = payload.into_inner();
    match chart_of_accounts_service
        .update_account_by_code(&code, admin_user.0.tenant_id, update)
        .await
    {
        Ok(account) => Ok(HttpResponse::Ok().json(account)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete chart account (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/chart-of-accounts/{code}", tag = "Chart of Accounts",
    params(("code" = String, Path, description = "Account code")),
    responses((status = 200, description = "Chart account soft deleted", body = MessageResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_chart_account(
    admin_user: AdminUser,
    chart_of_accounts_service: web::Data<ChartOfAccountsService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let code = path.into_inner();
    let user_id: i64 = admin_user.0.sub.parse().unwrap_or(0);
    match chart_of_accounts_service
        .delete_account_by_code(&code, admin_user.0.tenant_id, user_id)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "chart_account.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get account tree (hierarchical)
#[utoipa::path(
    get, path = "/api/v1/chart-of-accounts/tree", tag = "Chart of Accounts",
    responses((status = 200, description = "Account tree")),
    security(("bearer_auth" = []))
)]
pub async fn get_account_tree(
    auth_user: AuthUser,
    chart_of_accounts_service: web::Data<ChartOfAccountsService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match chart_of_accounts_service
        .get_tree(auth_user.0.tenant_id)
        .await
    {
        Ok(tree) => Ok(HttpResponse::Ok().json(tree)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get children of a chart account by parent code
#[utoipa::path(
    get, path = "/api/v1/chart-of-accounts/{code}/children", tag = "Chart of Accounts",
    params(("code" = String, Path, description = "Parent account code")),
    responses((status = 200, description = "List of child accounts"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_chart_account_children(
    auth_user: AuthUser,
    chart_of_accounts_service: web::Data<ChartOfAccountsService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let code = path.into_inner();
    match chart_of_accounts_service
        .get_children(&code, auth_user.0.tenant_id)
        .await
    {
        Ok(children) => Ok(HttpResponse::Ok().json(children)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Recalculate balance of a chart account (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/chart-of-accounts/{code}/recalculate", tag = "Chart of Accounts",
    params(("code" = String, Path, description = "Account code")),
    responses((status = 200, description = "Balance recalculated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn recalculate_chart_account_balance(
    admin_user: AdminUser,
    chart_of_accounts_service: web::Data<ChartOfAccountsService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let code = path.into_inner();
    match chart_of_accounts_service
        .recalculate_balance_by_code(&code, admin_user.0.tenant_id)
        .await
    {
        Ok(account) => Ok(HttpResponse::Ok().json(account)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get trial balance
#[utoipa::path(
    get, path = "/api/v1/chart-of-accounts/trial-balance", tag = "Chart of Accounts",
    responses((status = 200, description = "Trial balance")),
    security(("bearer_auth" = []))
)]
pub async fn get_trial_balance(
    auth_user: AuthUser,
    chart_of_accounts_service: web::Data<ChartOfAccountsService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match chart_of_accounts_service
        .get_trial_balance(auth_user.0.tenant_id)
        .await
    {
        Ok(balance) => Ok(HttpResponse::Ok().json(balance)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure chart of accounts routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/chart-of-accounts")
            .route(web::get().to(list_chart_accounts))
            .route(web::post().to(create_chart_account)),
    )
    .service(web::resource("/v1/chart-of-accounts/tree").route(web::get().to(get_account_tree)))
    .service(
        web::resource("/v1/chart-of-accounts/trial-balance")
            .route(web::get().to(get_trial_balance)),
    )
    .service(
        web::resource("/v1/chart-of-accounts/{code}")
            .route(web::get().to(get_chart_account_by_code))
            .route(web::put().to(update_chart_account))
            .route(web::delete().to(soft_delete_chart_account)),
    )
    .service(
        web::resource("/v1/chart-of-accounts/{code}/children")
            .route(web::get().to(get_chart_account_children)),
    )
    .service(
        web::resource("/v1/chart-of-accounts/{code}/recalculate")
            .route(web::post().to(recalculate_chart_account_balance)),
    );
}
