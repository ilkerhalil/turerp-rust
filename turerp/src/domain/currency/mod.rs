//! Currency domain module

pub mod model;
pub mod repository;
pub mod service;

pub mod postgres_repository;

pub use model::{
    ConversionResult, CreateCurrency, CreateExchangeRate, Currency, CurrencyResponse, ExchangeRate,
    ExchangeRateResponse, UpdateCurrency, UpdateExchangeRate,
};
pub use repository::{
    BoxCurrencyRepository, BoxExchangeRateRepository, CurrencyRepository, ExchangeRateRepository,
    InMemoryCurrencyRepository, InMemoryExchangeRateRepository,
};
pub use service::CurrencyService;

pub use postgres_repository::{PostgresCurrencyRepository, PostgresExchangeRateRepository};
