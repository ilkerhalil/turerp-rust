//! Currency API endpoints (v1)

use actix_web::{web, HttpResponse};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::common::pagination::{default_page, default_per_page, PaginationParams};
use crate::common::PaginatedResult;
use crate::domain::currency::model::{
    CreateCurrency, CreateExchangeRate, CurrencyResponse, ExchangeRateResponse, UpdateCurrency,
};
use crate::domain::currency::service::CurrencyService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Query parameters for listing currencies
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListCurrenciesQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    pub active_only: Option<bool>,
}

impl From<ListCurrenciesQuery> for PaginationParams {
    fn from(q: ListCurrenciesQuery) -> Self {
        Self {
            page: q.page,
            per_page: q.per_page,
        }
    }
}

/// Query parameters for listing exchange rates
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListExchangeRatesQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    pub currency: Option<String>,
    pub date: Option<NaiveDate>,
}

impl From<ListExchangeRatesQuery> for PaginationParams {
    fn from(q: ListExchangeRatesQuery) -> Self {
        Self {
            page: q.page,
            per_page: q.per_page,
        }
    }
}

/// Query parameters for converting currency
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct ConvertQuery {
    pub amount: Decimal,
    pub from: String,
    pub to: String,
    pub date: Option<NaiveDate>,
}

/// Query parameters for getting effective rate
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct EffectiveRateQuery {
    pub from: String,
    pub to: String,
    pub date: Option<NaiveDate>,
}

// ---------------------------------------------------------------------------
// Currency endpoints
// ---------------------------------------------------------------------------

/// List currencies (requires authentication)
#[utoipa::path(
    get, path = "/api/v1/currencies", tag = "Currency",
    params(ListCurrenciesQuery),
    responses((status = 200, description = "Paginated list of currencies", body = PaginatedResult<CurrencyResponse>)),
    security(("bearer_auth" = []))
)]
pub async fn list_currencies(
    auth_user: AuthUser,
    currency_service: web::Data<CurrencyService>,
    query: web::Query<ListCurrenciesQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let active_only = query.active_only;
    let pagination: PaginationParams = query.into_inner().into();
    if let Err(e) = pagination.validate() {
        let err = crate::error::ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    match currency_service
        .list_currencies(auth_user.0.tenant_id, active_only, pagination)
        .await
    {
        Ok(result) => {
            let mapped = PaginatedResult::new(
                result
                    .items
                    .into_iter()
                    .map(CurrencyResponse::from)
                    .collect(),
                result.page,
                result.per_page,
                result.total,
            );
            Ok(HttpResponse::Ok().json(mapped))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Create currency (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/currencies", tag = "Currency",
    request_body = CreateCurrency,
    responses((status = 201, description = "Currency created", body = CurrencyResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_currency(
    admin_user: AdminUser,
    currency_service: web::Data<CurrencyService>,
    payload: web::Json<CreateCurrency>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match currency_service
        .create_currency(payload.into_inner(), admin_user.0.tenant_id)
        .await
    {
        Ok(currency) => Ok(HttpResponse::Created().json(CurrencyResponse::from(currency))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get currency by code (requires authentication)
#[utoipa::path(
    get, path = "/api/v1/currencies/{code}", tag = "Currency",
    params(("code" = String, Path, description = "Currency code (e.g. USD)")),
    responses((status = 200, description = "Currency found", body = CurrencyResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_currency(
    auth_user: AuthUser,
    currency_service: web::Data<CurrencyService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match currency_service
        .get_currency_by_code(&path.into_inner(), auth_user.0.tenant_id)
        .await
    {
        Ok(currency) => Ok(HttpResponse::Ok().json(CurrencyResponse::from(currency))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update currency (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/currencies/{code}", tag = "Currency",
    params(("code" = String, Path, description = "Currency code")),
    request_body = UpdateCurrency,
    responses((status = 200, description = "Currency updated", body = CurrencyResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn update_currency(
    admin_user: AdminUser,
    currency_service: web::Data<CurrencyService>,
    path: web::Path<String>,
    payload: web::Json<UpdateCurrency>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let code = path.into_inner();
    let currency = match currency_service
        .get_currency_by_code(&code, admin_user.0.tenant_id)
        .await
    {
        Ok(c) => c,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    match currency_service
        .update_currency(currency.id, admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(currency) => Ok(HttpResponse::Ok().json(CurrencyResponse::from(currency))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete currency (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/currencies/{code}/soft", tag = "Currency",
    params(("code" = String, Path, description = "Currency code")),
    responses((status = 204, description = "Currency soft deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_currency(
    admin_user: AdminUser,
    currency_service: web::Data<CurrencyService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let code = path.into_inner();
    let currency = match currency_service
        .get_currency_by_code(&code, admin_user.0.tenant_id)
        .await
    {
        Ok(c) => c,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    let deleted_by = admin_user.0.sub.parse::<i64>().unwrap_or(0);
    match currency_service
        .soft_delete_currency(currency.id, admin_user.0.tenant_id, deleted_by)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore soft-deleted currency (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/currencies/{code}/restore", tag = "Currency",
    params(("code" = String, Path, description = "Currency code")),
    responses((status = 204, description = "Currency restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_currency(
    admin_user: AdminUser,
    currency_service: web::Data<CurrencyService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let code = path.into_inner();
    let currency = match currency_service
        .get_currency_by_code(&code, admin_user.0.tenant_id)
        .await
    {
        Ok(c) => c,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    match currency_service
        .restore_currency(currency.id, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List deleted currencies (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/currencies/deleted", tag = "Currency",
    responses((status = 200, description = "List of deleted currencies", body = Vec<CurrencyResponse>)),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_currencies(
    admin_user: AdminUser,
    currency_service: web::Data<CurrencyService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match currency_service
        .list_deleted_currencies(admin_user.0.tenant_id)
        .await
    {
        Ok(currencies) => {
            let mapped: Vec<CurrencyResponse> =
                currencies.into_iter().map(CurrencyResponse::from).collect();
            Ok(HttpResponse::Ok().json(mapped))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Permanently destroy a soft-deleted currency (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/currencies/{code}/destroy", tag = "Currency",
    params(("code" = String, Path, description = "Currency code")),
    responses((status = 204, description = "Currency permanently destroyed"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_currency(
    admin_user: AdminUser,
    currency_service: web::Data<CurrencyService>,
    path: web::Path<String>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let code = path.into_inner();
    let currency = match currency_service
        .get_currency_by_code(&code, admin_user.0.tenant_id)
        .await
    {
        Ok(c) => c,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    match currency_service
        .destroy_currency(currency.id, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// ---------------------------------------------------------------------------
// Exchange rate endpoints
// ---------------------------------------------------------------------------

/// List exchange rates (requires authentication)
#[utoipa::path(
    get, path = "/api/v1/exchange-rates", tag = "Currency",
    params(ListExchangeRatesQuery),
    responses((status = 200, description = "Paginated list of exchange rates", body = PaginatedResult<ExchangeRateResponse>)),
    security(("bearer_auth" = []))
)]
pub async fn list_exchange_rates(
    auth_user: AuthUser,
    currency_service: web::Data<CurrencyService>,
    query: web::Query<ListExchangeRatesQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let currency = query.currency.clone();
    let date = query.date;
    let pagination: PaginationParams = query.into_inner().into();
    if let Err(e) = pagination.validate() {
        let err = crate::error::ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    match currency_service
        .list_exchange_rates(auth_user.0.tenant_id, currency, date, pagination)
        .await
    {
        Ok(result) => {
            let mapped = PaginatedResult::new(
                result
                    .items
                    .into_iter()
                    .map(ExchangeRateResponse::from)
                    .collect(),
                result.page,
                result.per_page,
                result.total,
            );
            Ok(HttpResponse::Ok().json(mapped))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Create exchange rate (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/exchange-rates", tag = "Currency",
    request_body = CreateExchangeRate,
    responses((status = 201, description = "Exchange rate created", body = ExchangeRateResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_exchange_rate(
    admin_user: AdminUser,
    currency_service: web::Data<CurrencyService>,
    payload: web::Json<CreateExchangeRate>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match currency_service
        .create_exchange_rate(payload.into_inner(), admin_user.0.tenant_id)
        .await
    {
        Ok(rate) => Ok(HttpResponse::Created().json(ExchangeRateResponse::from(rate))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Convert amount between currencies (requires authentication)
#[utoipa::path(
    get, path = "/api/v1/exchange-rates/convert", tag = "Currency",
    params(ConvertQuery),
    responses((status = 200, description = "Conversion result", body = ConversionResult)),
    security(("bearer_auth" = []))
)]
pub async fn convert_amount(
    auth_user: AuthUser,
    currency_service: web::Data<CurrencyService>,
    query: web::Query<ConvertQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let q = query.into_inner();
    let date = q
        .date
        .unwrap_or_else(|| chrono::Local::now().naive_local().date());
    match currency_service
        .convert(q.amount, &q.from, &q.to, date, auth_user.0.tenant_id)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get effective exchange rate (requires authentication)
#[utoipa::path(
    get, path = "/api/v1/exchange-rates/effective", tag = "Currency",
    params(EffectiveRateQuery),
    responses((status = 200, description = "Effective rate found", body = ExchangeRateResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_effective_rate(
    auth_user: AuthUser,
    currency_service: web::Data<CurrencyService>,
    query: web::Query<EffectiveRateQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let q = query.into_inner();
    let date = q
        .date
        .unwrap_or_else(|| chrono::Local::now().naive_local().date());
    match currency_service
        .get_effective_rate(&q.from, &q.to, date, auth_user.0.tenant_id)
        .await
    {
        Ok(rate) => Ok(HttpResponse::Ok().json(ExchangeRateResponse::from(rate))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete exchange rate (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/exchange-rates/{id}/soft", tag = "Currency",
    params(("id" = i64, Path, description = "Exchange rate ID")),
    responses((status = 204, description = "Exchange rate soft deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_exchange_rate(
    admin_user: AdminUser,
    currency_service: web::Data<CurrencyService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let deleted_by = admin_user.0.sub.parse::<i64>().unwrap_or(0);
    match currency_service
        .soft_delete_exchange_rate(id, admin_user.0.tenant_id, deleted_by)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore soft-deleted exchange rate (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/exchange-rates/{id}/restore", tag = "Currency",
    params(("id" = i64, Path, description = "Exchange rate ID")),
    responses((status = 204, description = "Exchange rate restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_exchange_rate(
    admin_user: AdminUser,
    currency_service: web::Data<CurrencyService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match currency_service
        .restore_exchange_rate(id, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List deleted exchange rates (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/exchange-rates/deleted", tag = "Currency",
    responses((status = 200, description = "List of deleted exchange rates", body = Vec<ExchangeRateResponse>)),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_exchange_rates(
    admin_user: AdminUser,
    currency_service: web::Data<CurrencyService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match currency_service
        .list_deleted_exchange_rates(admin_user.0.tenant_id)
        .await
    {
        Ok(rates) => {
            let mapped: Vec<ExchangeRateResponse> =
                rates.into_iter().map(ExchangeRateResponse::from).collect();
            Ok(HttpResponse::Ok().json(mapped))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Permanently destroy a soft-deleted exchange rate (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/exchange-rates/{id}/destroy", tag = "Currency",
    params(("id" = i64, Path, description = "Exchange rate ID")),
    responses((status = 204, description = "Exchange rate permanently destroyed"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_exchange_rate(
    admin_user: AdminUser,
    currency_service: web::Data<CurrencyService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match currency_service
        .destroy_exchange_rate(id, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// ---------------------------------------------------------------------------
// Route configuration
// ---------------------------------------------------------------------------

/// Configure currency routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/currencies")
            .route("", web::get().to(list_currencies))
            .route("", web::post().to(create_currency))
            .route("/{code}", web::get().to(get_currency))
            .route("/{code}", web::put().to(update_currency))
            .route("/{code}/soft", web::delete().to(soft_delete_currency))
            .route("/{code}/restore", web::post().to(restore_currency))
            .route("/deleted", web::get().to(list_deleted_currencies))
            .route("/{code}/destroy", web::delete().to(destroy_currency)),
    )
    .service(
        web::scope("/exchange-rates")
            .route("", web::get().to(list_exchange_rates))
            .route("", web::post().to(create_exchange_rate))
            .route("/convert", web::get().to(convert_amount))
            .route("/effective", web::get().to(get_effective_rate))
            .route("/{id}/soft", web::delete().to(soft_delete_exchange_rate))
            .route("/{id}/restore", web::post().to(restore_exchange_rate))
            .route("/deleted", web::get().to(list_deleted_exchange_rates))
            .route("/{id}/destroy", web::delete().to(destroy_exchange_rate)),
    );
}
