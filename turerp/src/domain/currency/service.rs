//! Currency service — business logic for currency and exchange rate management

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::currency::model::{
    ConversionResult, CreateCurrency, CreateExchangeRate, Currency, ExchangeRate, UpdateCurrency,
    UpdateExchangeRate,
};
use crate::domain::currency::repository::{BoxCurrencyRepository, BoxExchangeRateRepository};
use crate::error::ApiError;

/// Service for managing currencies and exchange rates
#[derive(Clone)]
pub struct CurrencyService {
    currency_repo: BoxCurrencyRepository,
    rate_repo: BoxExchangeRateRepository,
}

impl CurrencyService {
    pub fn new(currency_repo: BoxCurrencyRepository, rate_repo: BoxExchangeRateRepository) -> Self {
        Self {
            currency_repo,
            rate_repo,
        }
    }

    // ---- Currency Operations ----

    /// Create a new currency
    pub async fn create_currency(
        &self,
        create: CreateCurrency,
        tenant_id: i64,
    ) -> Result<Currency, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // If this currency is being set as base, ensure no other base currency exists
        if create.is_base {
            if let Some(existing) = self.currency_repo.find_base(tenant_id).await? {
                if existing.code != create.code.trim().to_uppercase() {
                    return Err(ApiError::Conflict(format!(
                        "Base currency already exists: {}. Unset it first.",
                        existing.code
                    )));
                }
            }
        }

        self.currency_repo.create(create, tenant_id).await
    }

    /// Get a currency by ID
    pub async fn get_currency(&self, id: i64, tenant_id: i64) -> Result<Currency, ApiError> {
        self.currency_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Currency {} not found", id)))
    }

    /// Get a currency by code
    pub async fn get_currency_by_code(
        &self,
        code: &str,
        tenant_id: i64,
    ) -> Result<Currency, ApiError> {
        self.currency_repo
            .find_by_code(code, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Currency {} not found", code)))
    }

    /// Get the base currency for a tenant
    pub async fn get_base_currency(&self, tenant_id: i64) -> Result<Currency, ApiError> {
        self.currency_repo
            .find_base(tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("No base currency configured".to_string()))
    }

    /// List currencies with optional active filter and pagination
    pub async fn list_currencies(
        &self,
        tenant_id: i64,
        active_only: Option<bool>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<Currency>, ApiError> {
        self.currency_repo
            .find_all(tenant_id, active_only, params)
            .await
    }

    /// Update a currency
    pub async fn update_currency(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCurrency,
    ) -> Result<Currency, ApiError> {
        if let Some(is_base) = update.is_base {
            if is_base {
                let existing = self.currency_repo.find_base(tenant_id).await?;
                if let Some(base) = existing {
                    if base.id != id {
                        return Err(ApiError::Conflict(format!(
                            "Base currency already exists: {}. Unset it first.",
                            base.code
                        )));
                    }
                }
            }
        }
        if let Some(decimal_places) = update.decimal_places {
            if !(0..=8).contains(&decimal_places) {
                return Err(ApiError::Validation(
                    "Decimal places must be between 0 and 8".to_string(),
                ));
            }
        }
        self.currency_repo.update(id, tenant_id, update).await
    }

    /// Delete a currency
    pub async fn delete_currency(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.currency_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Currency {} not found", id)))?;
        self.currency_repo.delete(id, tenant_id).await
    }

    /// Soft delete a currency
    pub async fn soft_delete_currency(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.currency_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    /// Restore a soft-deleted currency
    pub async fn restore_currency(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.currency_repo.restore(id, tenant_id).await
    }

    /// List deleted currencies for a tenant
    pub async fn list_deleted_currencies(&self, tenant_id: i64) -> Result<Vec<Currency>, ApiError> {
        self.currency_repo.find_deleted(tenant_id).await
    }

    /// Permanently destroy a soft-deleted currency
    pub async fn destroy_currency(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.currency_repo.destroy(id, tenant_id).await
    }

    // ---- Exchange Rate Operations ----

    /// Create a new exchange rate
    pub async fn create_exchange_rate(
        &self,
        create: CreateExchangeRate,
        tenant_id: i64,
    ) -> Result<ExchangeRate, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Verify both currencies exist
        self.get_currency_by_code(&create.from_currency, tenant_id)
            .await?;
        self.get_currency_by_code(&create.to_currency, tenant_id)
            .await?;

        self.rate_repo.create(create, tenant_id).await
    }

    /// Get an exchange rate by ID
    pub async fn get_exchange_rate(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<ExchangeRate, ApiError> {
        self.rate_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Exchange rate {} not found", id)))
    }

    /// Get the effective exchange rate for a pair on a given date
    pub async fn get_effective_rate(
        &self,
        from: &str,
        to: &str,
        date: NaiveDate,
        tenant_id: i64,
    ) -> Result<ExchangeRate, ApiError> {
        self.get_effective_rate_inner(from, to, date, tenant_id)
            .await
    }

    async fn get_effective_rate_inner(
        &self,
        from: &str,
        to: &str,
        date: NaiveDate,
        tenant_id: i64,
    ) -> Result<ExchangeRate, ApiError> {
        let from = from.trim().to_uppercase();
        let to = to.trim().to_uppercase();

        // Direct rate lookup
        let direct = self
            .rate_repo
            .find_effective_rate(&from, &to, date, tenant_id)
            .await?;
        if let Some(rate) = direct {
            return Ok(rate);
        }

        // If from == to, return a 1:1 rate (no conversion needed)
        if from == to {
            let now = chrono::Utc::now();
            return Ok(ExchangeRate {
                id: 0,
                tenant_id,
                from_currency: from.clone(),
                to_currency: to.clone(),
                rate: Decimal::ONE,
                effective_date: date,
                created_at: now,
                deleted_at: None,
                deleted_by: None,
            });
        }

        // Try inverse rate
        let inverse = self
            .rate_repo
            .find_effective_rate(&to, &from, date, tenant_id)
            .await?;
        if let Some(rate) = inverse {
            return Ok(ExchangeRate {
                id: 0,
                tenant_id,
                from_currency: from.clone(),
                to_currency: to.clone(),
                rate: Decimal::ONE / rate.rate,
                effective_date: date,
                created_at: rate.created_at,
                deleted_at: None,
                deleted_by: None,
            });
        }

        // Try via base currency
        if let Ok(base) = self.get_base_currency(tenant_id).await {
            if from != base.code && to != base.code {
                let from_to_base =
                    Box::pin(self.get_effective_rate_inner(&from, &base.code, date, tenant_id))
                        .await?;
                let base_to_target =
                    Box::pin(self.get_effective_rate_inner(&base.code, &to, date, tenant_id))
                        .await?;

                return Ok(ExchangeRate {
                    id: 0,
                    tenant_id,
                    from_currency: from.clone(),
                    to_currency: to.clone(),
                    rate: from_to_base.rate * base_to_target.rate,
                    effective_date: date,
                    created_at: chrono::Utc::now(),
                    deleted_at: None,
                    deleted_by: None,
                });
            }
        }

        Err(ApiError::NotFound(format!(
            "No effective exchange rate found for {} to {} on {}",
            from, to, date
        )))
    }

    /// List exchange rates with optional currency and date filters
    pub async fn list_exchange_rates(
        &self,
        tenant_id: i64,
        currency: Option<String>,
        date: Option<NaiveDate>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ExchangeRate>, ApiError> {
        self.rate_repo
            .find_all(tenant_id, currency, date, params)
            .await
    }

    /// List all rates effective on a specific date
    pub async fn list_effective_on(
        &self,
        tenant_id: i64,
        date: NaiveDate,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ExchangeRate>, ApiError> {
        self.rate_repo
            .list_effective_on(tenant_id, date, params)
            .await
    }

    /// Update an exchange rate
    pub async fn update_exchange_rate(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateExchangeRate,
    ) -> Result<ExchangeRate, ApiError> {
        if let Some(rate) = update.rate {
            if rate <= Decimal::ZERO {
                return Err(ApiError::Validation(
                    "Exchange rate must be positive".to_string(),
                ));
            }
        }
        self.rate_repo.update(id, tenant_id, update).await
    }

    /// Delete an exchange rate
    pub async fn delete_exchange_rate(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.rate_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Exchange rate {} not found", id)))?;
        self.rate_repo.delete(id, tenant_id).await
    }

    /// Soft delete an exchange rate
    pub async fn soft_delete_exchange_rate(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.rate_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Restore a soft-deleted exchange rate
    pub async fn restore_exchange_rate(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.rate_repo.restore(id, tenant_id).await
    }

    /// List deleted exchange rates for a tenant
    pub async fn list_deleted_exchange_rates(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<ExchangeRate>, ApiError> {
        self.rate_repo.find_deleted(tenant_id).await
    }

    /// Permanently destroy a soft-deleted exchange rate
    pub async fn destroy_exchange_rate(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.rate_repo.destroy(id, tenant_id).await
    }

    // ---- Conversion Operations ----

    /// Convert an amount from one currency to another using the effective rate
    pub async fn convert(
        &self,
        amount: Decimal,
        from: &str,
        to: &str,
        date: NaiveDate,
        tenant_id: i64,
    ) -> Result<ConversionResult, ApiError> {
        if amount < Decimal::ZERO {
            return Err(ApiError::Validation(
                "Amount cannot be negative".to_string(),
            ));
        }

        let rate = self.get_effective_rate(from, to, date, tenant_id).await?;
        let converted = (amount * rate.rate).round_dp(2);

        Ok(ConversionResult {
            amount,
            from_currency: from.trim().to_uppercase(),
            to_currency: to.trim().to_uppercase(),
            rate: rate.rate,
            converted_amount: converted,
            effective_date: date,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::currency::repository::{
        InMemoryCurrencyRepository, InMemoryExchangeRateRepository,
    };
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn make_service() -> CurrencyService {
        let currency_repo = Arc::new(InMemoryCurrencyRepository::new());
        let rate_repo = Arc::new(InMemoryExchangeRateRepository::new());
        CurrencyService::new(currency_repo, rate_repo)
    }

    #[tokio::test]
    async fn test_create_and_get_currency() {
        let svc = make_service();

        let create = CreateCurrency {
            code: "USD".to_string(),
            name: "US Dollar".to_string(),
            symbol: "$".to_string(),
            decimal_places: 2,
            is_active: true,
            is_base: false,
        };

        let currency = svc.create_currency(create, 1).await.unwrap();
        assert_eq!(currency.code, "USD");

        let found = svc.get_currency(currency.id, 1).await.unwrap();
        assert_eq!(found.id, currency.id);

        let by_code = svc.get_currency_by_code("usd", 1).await.unwrap();
        assert_eq!(by_code.code, "USD");
    }

    #[tokio::test]
    async fn test_base_currency_conflict() {
        let svc = make_service();

        let base = CreateCurrency {
            code: "TRY".to_string(),
            name: "Turkish Lira".to_string(),
            symbol: "₺".to_string(),
            decimal_places: 2,
            is_active: true,
            is_base: true,
        };
        svc.create_currency(base, 1).await.unwrap();

        let another_base = CreateCurrency {
            code: "USD".to_string(),
            name: "US Dollar".to_string(),
            symbol: "$".to_string(),
            decimal_places: 2,
            is_active: true,
            is_base: true,
        };
        let result = svc.create_currency(another_base, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_convert_currency() {
        let svc = make_service();

        // Create currencies
        svc.create_currency(
            CreateCurrency {
                code: "USD".to_string(),
                name: "US Dollar".to_string(),
                symbol: "$".to_string(),
                decimal_places: 2,
                is_active: true,
                is_base: false,
            },
            1,
        )
        .await
        .unwrap();

        svc.create_currency(
            CreateCurrency {
                code: "EUR".to_string(),
                name: "Euro".to_string(),
                symbol: "€".to_string(),
                decimal_places: 2,
                is_active: true,
                is_base: false,
            },
            1,
        )
        .await
        .unwrap();

        // Create exchange rate
        svc.create_exchange_rate(
            CreateExchangeRate {
                from_currency: "USD".to_string(),
                to_currency: "EUR".to_string(),
                rate: dec!(0.85),
                effective_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            },
            1,
        )
        .await
        .unwrap();

        let result = svc
            .convert(
                dec!(100),
                "USD",
                "EUR",
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                1,
            )
            .await
            .unwrap();

        assert_eq!(result.amount, dec!(100));
        assert_eq!(result.converted_amount, dec!(85));
        assert_eq!(result.rate, dec!(0.85));
    }

    #[tokio::test]
    async fn test_convert_same_currency() {
        let svc = make_service();

        svc.create_currency(
            CreateCurrency {
                code: "USD".to_string(),
                name: "US Dollar".to_string(),
                symbol: "$".to_string(),
                decimal_places: 2,
                is_active: true,
                is_base: false,
            },
            1,
        )
        .await
        .unwrap();

        let result = svc
            .convert(
                dec!(100),
                "USD",
                "USD",
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                1,
            )
            .await
            .unwrap();

        assert_eq!(result.rate, Decimal::ONE);
        assert_eq!(result.converted_amount, dec!(100));
    }

    #[tokio::test]
    async fn test_inverse_rate_conversion() {
        let svc = make_service();

        svc.create_currency(
            CreateCurrency {
                code: "USD".to_string(),
                name: "US Dollar".to_string(),
                symbol: "$".to_string(),
                decimal_places: 2,
                is_active: true,
                is_base: false,
            },
            1,
        )
        .await
        .unwrap();

        svc.create_currency(
            CreateCurrency {
                code: "EUR".to_string(),
                name: "Euro".to_string(),
                symbol: "€".to_string(),
                decimal_places: 2,
                is_active: true,
                is_base: false,
            },
            1,
        )
        .await
        .unwrap();

        // Only create USD -> EUR rate, convert EUR -> USD
        svc.create_exchange_rate(
            CreateExchangeRate {
                from_currency: "USD".to_string(),
                to_currency: "EUR".to_string(),
                rate: dec!(0.85),
                effective_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            },
            1,
        )
        .await
        .unwrap();

        let result = svc
            .convert(
                dec!(85),
                "EUR",
                "USD",
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                1,
            )
            .await
            .unwrap();

        assert_eq!(result.rate, dec!(1) / dec!(0.85));
        assert_eq!(result.converted_amount, dec!(100));
    }

    #[tokio::test]
    async fn test_triangular_conversion_via_base() {
        let svc = make_service();

        svc.create_currency(
            CreateCurrency {
                code: "USD".to_string(),
                name: "US Dollar".to_string(),
                symbol: "$".to_string(),
                decimal_places: 2,
                is_active: true,
                is_base: true,
            },
            1,
        )
        .await
        .unwrap();

        svc.create_currency(
            CreateCurrency {
                code: "EUR".to_string(),
                name: "Euro".to_string(),
                symbol: "€".to_string(),
                decimal_places: 2,
                is_active: true,
                is_base: false,
            },
            1,
        )
        .await
        .unwrap();

        svc.create_currency(
            CreateCurrency {
                code: "GBP".to_string(),
                name: "British Pound".to_string(),
                symbol: "£".to_string(),
                decimal_places: 2,
                is_active: true,
                is_base: false,
            },
            1,
        )
        .await
        .unwrap();

        // Create USD -> EUR and USD -> GBP rates
        svc.create_exchange_rate(
            CreateExchangeRate {
                from_currency: "USD".to_string(),
                to_currency: "EUR".to_string(),
                rate: dec!(0.85),
                effective_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            },
            1,
        )
        .await
        .unwrap();

        svc.create_exchange_rate(
            CreateExchangeRate {
                from_currency: "USD".to_string(),
                to_currency: "GBP".to_string(),
                rate: dec!(0.75),
                effective_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            },
            1,
        )
        .await
        .unwrap();

        // Convert EUR -> GBP via USD base
        let result = svc
            .convert(
                dec!(85),
                "EUR",
                "GBP",
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                1,
            )
            .await
            .unwrap();

        // EUR -> USD = 1 / 0.85, USD -> GBP = 0.75
        // EUR -> GBP = 0.75 / 0.85 = 0.88235...
        assert_eq!(result.rate, dec!(0.75) / dec!(0.85));
        assert_eq!(result.converted_amount, dec!(75));
    }

    #[tokio::test]
    async fn test_list_currencies() {
        let svc = make_service();

        for code in ["USD", "EUR", "GBP"] {
            svc.create_currency(
                CreateCurrency {
                    code: code.to_string(),
                    name: format!("{}", code),
                    symbol: code.to_string(),
                    decimal_places: 2,
                    is_active: true,
                    is_base: false,
                },
                1,
            )
            .await
            .unwrap();
        }

        let params = PaginationParams::default();
        let all = svc.list_currencies(1, None, params).await.unwrap();
        assert_eq!(all.items.len(), 3);
    }
}
