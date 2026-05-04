//! Tax Engine domain module
//!
//! Provides Turkish tax management including KDV (VAT), OIV, BSMV, stopaj,
//! and corporate/income tax period tracking with calculation support.

pub mod calculator;
pub mod model;
pub mod repository;
pub mod service;

pub use calculator::{
    get_calculator, BsmvCalculator, DamgaCalculator, KdvCalculator, OivCalculator,
    StopajCalculator, TaxCalculator,
};
pub use model::{
    CreateTaxPeriod, CreateTaxRate, TaxCalculationResult, TaxPeriod, TaxPeriodDetail,
    TaxPeriodResponse, TaxPeriodStatus, TaxRate, TaxRateResponse, TaxType, UpdateTaxRate,
};
pub use repository::{
    BoxTaxPeriodRepository, BoxTaxRateRepository, InMemoryTaxPeriodRepository,
    InMemoryTaxRateRepository, TaxPeriodRepository, TaxRateRepository,
};
pub use service::TaxService;
