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
            .route("/{code}", web::put().to(update_currency)),
    )
    .service(
        web::scope("/exchange-rates")
            .route("", web::get().to(list_exchange_rates))
            .route("", web::post().to(create_exchange_rate))
            .route("/convert", web::get().to(convert_amount))
            .route("/effective", web::get().to(get_effective_rate)),
    );
}
