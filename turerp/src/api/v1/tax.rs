//! Tax Engine API endpoints (v1)

use actix_web::web;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::ToSchema;

pub mod periods;
pub mod rates;

/// Request body for calculating tax
#[derive(Debug, Deserialize, ToSchema)]
pub struct CalculateTaxRequest {
    pub amount: Decimal,
    pub tax_type: String,
    pub date: NaiveDate,
    pub inclusive: Option<bool>,
}

/// Request body for calculating invoice taxes
#[derive(Debug, Deserialize, ToSchema)]
pub struct CalculateInvoiceTaxRequest {
    pub invoice_id: i64,
}

/// Request body for bulk restore operations
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkRestoreRequest {
    pub ids: Vec<i64>,
}

/// Query params for getting the effective tax rate
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct EffectiveRateQuery {
    pub tax_type: String,
    pub date: NaiveDate,
}

/// Configure tax engine routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/tax/rates")
            .route(web::get().to(rates::list_tax_rates))
            .route(web::post().to(rates::create_tax_rate)),
    )
    .service(
        web::resource("/v1/tax/rates/effective").route(web::get().to(rates::get_effective_rate)),
    )
    .service(
        web::resource("/v1/tax/rates/deleted").route(web::get().to(rates::list_deleted_tax_rates)),
    )
    .service(
        web::resource("/v1/tax/rates/bulk-restore")
            .route(web::post().to(rates::bulk_restore_tax_rates)),
    )
    .service(
        web::resource("/v1/tax/rates/{id}")
            .route(web::get().to(rates::get_tax_rate))
            .route(web::put().to(rates::update_tax_rate))
            .route(web::delete().to(rates::delete_tax_rate)),
    )
    .service(
        web::resource("/v1/tax/rates/{id}/restore").route(web::put().to(rates::restore_tax_rate)),
    )
    .service(
        web::resource("/v1/tax/rates/{id}/destroy")
            .route(web::delete().to(rates::destroy_tax_rate)),
    )
    .service(web::resource("/v1/tax/calculate").route(web::post().to(rates::calculate_tax)))
    .service(
        web::resource("/v1/tax/calculate-invoice")
            .route(web::post().to(rates::calculate_invoice_tax)),
    )
    .service(
        web::resource("/v1/tax/periods")
            .route(web::get().to(periods::list_tax_periods))
            .route(web::post().to(periods::create_tax_period)),
    )
    .service(
        web::resource("/v1/tax/periods/bulk-restore")
            .route(web::post().to(periods::bulk_restore_tax_periods)),
    )
    .service(
        web::resource("/v1/tax/periods/deleted")
            .route(web::get().to(periods::list_deleted_tax_periods)),
    )
    .service(
        web::resource("/v1/tax/periods/{id}")
            .route(web::get().to(periods::get_tax_period))
            .route(web::delete().to(periods::delete_tax_period)),
    )
    .service(
        web::resource("/v1/tax/periods/{id}/calculate")
            .route(web::post().to(periods::calculate_tax_period)),
    )
    .service(
        web::resource("/v1/tax/periods/{id}/file").route(web::post().to(periods::file_tax_period)),
    )
    .service(
        web::resource("/v1/tax/periods/{id}/restore")
            .route(web::put().to(periods::restore_tax_period)),
    )
    .service(
        web::resource("/v1/tax/periods/{id}/destroy")
            .route(web::delete().to(periods::destroy_tax_period)),
    );
}
