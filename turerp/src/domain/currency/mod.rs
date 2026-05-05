//! Currency domain module

pub mod model;
pub mod repository;
pub mod service;

#[cfg(feature = "postgres")]
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

#[cfg(feature = "postgres")]
pub use postgres_repository::{PostgresCurrencyRepository, PostgresExchangeRateRepository};
