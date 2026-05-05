//! Currency domain models
//!
//! Provides types for multi-currency support including currencies
//! and exchange rates with tenant isolation.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Currency entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Currency {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub symbol: String,
    pub decimal_places: i32,
    pub is_active: bool,
    pub is_base: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Exchange rate entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExchangeRate {
    pub id: i64,
    pub tenant_id: i64,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub effective_date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

/// Result of a currency conversion
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConversionResult {
    pub amount: Decimal,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub converted_amount: Decimal,
    pub effective_date: NaiveDate,
}

// ---- DTOs ----

/// Create a new currency
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCurrency {
    pub code: String,
    pub name: String,
    pub symbol: String,
    #[serde(default = "default_decimal_places")]
    pub decimal_places: i32,
    #[serde(default = "default_is_active")]
    pub is_active: bool,
    #[serde(default)]
    pub is_base: bool,
}

fn default_decimal_places() -> i32 {
    2
}

fn default_is_active() -> bool {
    true
}

impl CreateCurrency {
    /// Validate the create currency request
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        let code = self.code.trim().to_uppercase();
        if code.is_empty() {
            errors.push("Currency code is required".to_string());
        } else if code.len() != 3 {
            errors.push("Currency code must be exactly 3 characters (ISO 4217)".to_string());
        } else if !code.chars().all(|c| c.is_ascii_alphabetic()) {
            errors.push("Currency code must contain only letters".to_string());
        }

        if self.name.trim().is_empty() {
            errors.push("Currency name is required".to_string());
        }
        if self.name.len() > 100 {
            errors.push("Currency name must be at most 100 characters".to_string());
        }

        if self.symbol.trim().is_empty() {
            errors.push("Currency symbol is required".to_string());
        }

        if self.decimal_places < 0 || self.decimal_places > 8 {
            errors.push("Decimal places must be between 0 and 8".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update an existing currency
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateCurrency {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub decimal_places: Option<i32>,
    #[serde(default)]
    pub is_active: Option<bool>,
    #[serde(default)]
    pub is_base: Option<bool>,
}

/// Create a new exchange rate
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateExchangeRate {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub effective_date: NaiveDate,
}

impl CreateExchangeRate {
    /// Validate the create exchange rate request
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        let from = self.from_currency.trim().to_uppercase();
        let to = self.to_currency.trim().to_uppercase();

        if from.is_empty() || from.len() != 3 || !from.chars().all(|c| c.is_ascii_alphabetic()) {
            errors.push("from_currency must be a valid 3-letter currency code".to_string());
        }
        if to.is_empty() || to.len() != 3 || !to.chars().all(|c| c.is_ascii_alphabetic()) {
            errors.push("to_currency must be a valid 3-letter currency code".to_string());
        }
        if from == to {
            errors.push("from_currency and to_currency must be different".to_string());
        }
        if self.rate <= Decimal::ZERO {
            errors.push("Exchange rate must be positive".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update an existing exchange rate
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateExchangeRate {
    #[serde(default)]
    pub rate: Option<Decimal>,
    #[serde(default)]
    pub effective_date: Option<NaiveDate>,
}

/// Currency response for API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CurrencyResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub symbol: String,
    pub decimal_places: i32,
    pub is_active: bool,
    pub is_base: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Currency> for CurrencyResponse {
    fn from(currency: Currency) -> Self {
        Self {
            id: currency.id,
            tenant_id: currency.tenant_id,
            code: currency.code,
            name: currency.name,
            symbol: currency.symbol,
            decimal_places: currency.decimal_places,
            is_active: currency.is_active,
            is_base: currency.is_base,
            created_at: currency.created_at,
            updated_at: currency.updated_at,
        }
    }
}

/// Exchange rate response for API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExchangeRateResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub effective_date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

impl From<ExchangeRate> for ExchangeRateResponse {
    fn from(rate: ExchangeRate) -> Self {
        Self {
            id: rate.id,
            tenant_id: rate.tenant_id,
            from_currency: rate.from_currency,
            to_currency: rate.to_currency,
            rate: rate.rate,
            effective_date: rate.effective_date,
            created_at: rate.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal_macros::dec;

    #[test]
    fn test_create_currency_validation() {
        let valid = CreateCurrency {
            code: "USD".to_string(),
            name: "US Dollar".to_string(),
            symbol: "$".to_string(),
            decimal_places: 2,
            is_active: true,
            is_base: false,
        };
        assert!(valid.validate().is_ok());

        let empty_code = CreateCurrency {
            code: "".to_string(),
            name: "Test".to_string(),
            symbol: "T".to_string(),
            decimal_places: 2,
            is_active: true,
            is_base: false,
        };
        assert!(empty_code.validate().is_err());

        let short_code = CreateCurrency {
            code: "US".to_string(),
            name: "Test".to_string(),
            symbol: "T".to_string(),
            decimal_places: 2,
            is_active: true,
            is_base: false,
        };
        assert!(short_code.validate().is_err());

        let invalid_decimals = CreateCurrency {
            code: "EUR".to_string(),
            name: "Euro".to_string(),
            symbol: "€".to_string(),
            decimal_places: 10,
            is_active: true,
            is_base: false,
        };
        assert!(invalid_decimals.validate().is_err());
    }

    #[test]
    fn test_create_exchange_rate_validation() {
        let valid = CreateExchangeRate {
            from_currency: "USD".to_string(),
            to_currency: "EUR".to_string(),
            rate: dec!(0.85),
            effective_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        assert!(valid.validate().is_ok());

        let same_currency = CreateExchangeRate {
            from_currency: "USD".to_string(),
            to_currency: "USD".to_string(),
            rate: dec!(1.0),
            effective_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        assert!(same_currency.validate().is_err());

        let zero_rate = CreateExchangeRate {
            from_currency: "USD".to_string(),
            to_currency: "EUR".to_string(),
            rate: Decimal::ZERO,
            effective_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        assert!(zero_rate.validate().is_err());
    }

    #[test]
    fn test_currency_response_from_currency() {
        let currency = Currency {
            id: 1,
            tenant_id: 1,
            code: "TRY".to_string(),
            name: "Turkish Lira".to_string(),
            symbol: "₺".to_string(),
            decimal_places: 2,
            is_active: true,
            is_base: true,
            created_at: Utc::now(),
            updated_at: None,
        };

        let response = CurrencyResponse::from(currency);
        assert_eq!(response.code, "TRY");
        assert!(response.is_base);
    }

    #[test]
    fn test_conversion_result() {
        let result = ConversionResult {
            amount: dec!(100),
            from_currency: "USD".to_string(),
            to_currency: "EUR".to_string(),
            rate: dec!(0.85),
            converted_amount: dec!(85),
            effective_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        assert_eq!(result.converted_amount, dec!(85));
    }
}
