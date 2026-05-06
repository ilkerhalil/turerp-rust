//! Currency repository traits and in-memory implementations

use async_trait::async_trait;
use chrono::NaiveDate;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::common::soft_delete::SoftDeletable;
use crate::domain::currency::model::{
    CreateCurrency, CreateExchangeRate, Currency, ExchangeRate, UpdateCurrency, UpdateExchangeRate,
};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// CurrencyRepository
// ---------------------------------------------------------------------------

/// Repository trait for currency operations
#[async_trait]
pub trait CurrencyRepository: Send + Sync {
    /// Create a new currency
    async fn create(&self, create: CreateCurrency, tenant_id: i64) -> Result<Currency, ApiError>;

    /// Find a currency by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Currency>, ApiError>;

    /// Find a currency by code
    async fn find_by_code(&self, code: &str, tenant_id: i64) -> Result<Option<Currency>, ApiError>;

    /// Find the base currency for a tenant
    async fn find_base(&self, tenant_id: i64) -> Result<Option<Currency>, ApiError>;

    /// Find all currencies with optional active filter and pagination
    async fn find_all(
        &self,
        tenant_id: i64,
        active_only: Option<bool>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<Currency>, ApiError>;

    /// Update a currency
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCurrency,
    ) -> Result<Currency, ApiError>;

    /// Delete a currency
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Soft delete a currency
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    /// Restore a soft-deleted currency
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// List deleted currencies for a tenant
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Currency>, ApiError>;

    /// Permanently destroy a soft-deleted currency
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type alias for boxed CurrencyRepository
pub type BoxCurrencyRepository = Arc<dyn CurrencyRepository>;

// ---------------------------------------------------------------------------
// InMemoryCurrencyRepository
// ---------------------------------------------------------------------------

struct CurrencyInner {
    currencies: HashMap<i64, Currency>,
    next_id: AtomicI64,
}

/// In-memory currency repository for testing and development
pub struct InMemoryCurrencyRepository {
    inner: Mutex<CurrencyInner>,
}

impl InMemoryCurrencyRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(CurrencyInner {
                currencies: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryCurrencyRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CurrencyRepository for InMemoryCurrencyRepository {
    async fn create(&self, create: CreateCurrency, tenant_id: i64) -> Result<Currency, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now();

        let currency = Currency {
            id,
            tenant_id,
            code: create.code.trim().to_uppercase(),
            name: create.name,
            symbol: create.symbol,
            decimal_places: create.decimal_places,
            is_active: create.is_active,
            is_base: create.is_base,
            created_at: now,
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };

        inner.currencies.insert(id, currency.clone());
        Ok(currency)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Currency>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .currencies
            .get(&id)
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .cloned())
    }

    async fn find_by_code(&self, code: &str, tenant_id: i64) -> Result<Option<Currency>, ApiError> {
        let inner = self.inner.lock();
        let code_upper = code.trim().to_uppercase();
        Ok(inner
            .currencies
            .values()
            .find(|c| c.tenant_id == tenant_id && c.code == code_upper && !c.is_deleted())
            .cloned())
    }

    async fn find_base(&self, tenant_id: i64) -> Result<Option<Currency>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .currencies
            .values()
            .find(|c| c.tenant_id == tenant_id && c.is_base && !c.is_deleted())
            .cloned())
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        active_only: Option<bool>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<Currency>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<Currency> = inner
            .currencies
            .values()
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .filter(|c| match active_only {
                Some(true) => c.is_active,
                Some(false) => !c.is_active,
                None => true,
            })
            .cloned()
            .collect();

        items.sort_by(|a, b| a.code.cmp(&b.code));
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<Currency> = items
            .into_iter()
            .skip(start as usize)
            .take(params.per_page as usize)
            .collect();
        Ok(PaginatedResult::new(
            paginated,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCurrency,
    ) -> Result<Currency, ApiError> {
        let mut inner = self.inner.lock();

        let currency = inner
            .currencies
            .get_mut(&id)
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Currency {} not found", id)))?;

        if let Some(name) = update.name {
            currency.name = name;
        }
        if let Some(symbol) = update.symbol {
            currency.symbol = symbol;
        }
        if let Some(decimal_places) = update.decimal_places {
            currency.decimal_places = decimal_places;
        }
        if let Some(is_active) = update.is_active {
            currency.is_active = is_active;
        }
        if let Some(is_base) = update.is_base {
            currency.is_base = is_base;
        }
        currency.updated_at = Some(chrono::Utc::now());

        Ok(currency.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let currency = inner
            .currencies
            .get(&id)
            .filter(|c| c.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Currency {} not found", id)))?;

        let key = currency.id;
        inner.currencies.remove(&key);
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let currency = inner
            .currencies
            .get_mut(&id)
            .filter(|c| c.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Currency {} not found", id)))?;

        if currency.is_deleted() {
            return Err(ApiError::Conflict(format!(
                "Currency {} is already deleted",
                id
            )));
        }

        currency.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let currency = inner
            .currencies
            .get_mut(&id)
            .filter(|c| c.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Currency {} not found", id)))?;

        if !currency.is_deleted() {
            return Err(ApiError::BadRequest(format!(
                "Currency {} is not deleted",
                id
            )));
        }

        currency.restore();
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Currency>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .currencies
            .values()
            .filter(|c| c.tenant_id == tenant_id && c.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let len_before = inner.currencies.len();
        inner
            .currencies
            .retain(|_, c| !(c.id == id && c.tenant_id == tenant_id && c.is_deleted()));

        if inner.currencies.len() == len_before {
            return Err(ApiError::NotFound(format!(
                "Deleted currency {} not found",
                id
            )));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ExchangeRateRepository
// ---------------------------------------------------------------------------

/// Repository trait for exchange rate operations
#[async_trait]
pub trait ExchangeRateRepository: Send + Sync {
    /// Create a new exchange rate
    async fn create(
        &self,
        create: CreateExchangeRate,
        tenant_id: i64,
    ) -> Result<ExchangeRate, ApiError>;

    /// Find an exchange rate by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ExchangeRate>, ApiError>;

    /// Find the effective exchange rate for a pair on a given date
    async fn find_effective_rate(
        &self,
        from: &str,
        to: &str,
        date: NaiveDate,
        tenant_id: i64,
    ) -> Result<Option<ExchangeRate>, ApiError>;

    /// List exchange rates by currency with optional date filter and pagination
    async fn find_all(
        &self,
        tenant_id: i64,
        currency: Option<String>,
        date: Option<NaiveDate>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ExchangeRate>, ApiError>;

    /// List all rates effective on a specific date
    async fn list_effective_on(
        &self,
        tenant_id: i64,
        date: NaiveDate,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ExchangeRate>, ApiError>;

    /// Update an exchange rate
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateExchangeRate,
    ) -> Result<ExchangeRate, ApiError>;

    /// Delete an exchange rate
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Soft delete an exchange rate
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    /// Restore a soft-deleted exchange rate
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// List deleted exchange rates for a tenant
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<ExchangeRate>, ApiError>;

    /// Permanently destroy a soft-deleted exchange rate
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type alias for boxed ExchangeRateRepository
pub type BoxExchangeRateRepository = Arc<dyn ExchangeRateRepository>;

// ---------------------------------------------------------------------------
// InMemoryExchangeRateRepository
// ---------------------------------------------------------------------------

struct RateInner {
    rates: HashMap<i64, ExchangeRate>,
    next_id: AtomicI64,
}

/// In-memory exchange rate repository for testing and development
pub struct InMemoryExchangeRateRepository {
    inner: Mutex<RateInner>,
}

impl InMemoryExchangeRateRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RateInner {
                rates: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryExchangeRateRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExchangeRateRepository for InMemoryExchangeRateRepository {
    async fn create(
        &self,
        create: CreateExchangeRate,
        tenant_id: i64,
    ) -> Result<ExchangeRate, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now();

        let rate = ExchangeRate {
            id,
            tenant_id,
            from_currency: create.from_currency.trim().to_uppercase(),
            to_currency: create.to_currency.trim().to_uppercase(),
            rate: create.rate,
            effective_date: create.effective_date,
            created_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        inner.rates.insert(id, rate.clone());
        Ok(rate)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ExchangeRate>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .rates
            .get(&id)
            .filter(|r| r.tenant_id == tenant_id && !r.is_deleted())
            .cloned())
    }

    async fn find_effective_rate(
        &self,
        from: &str,
        to: &str,
        date: NaiveDate,
        tenant_id: i64,
    ) -> Result<Option<ExchangeRate>, ApiError> {
        let inner = self.inner.lock();
        let from_upper = from.trim().to_uppercase();
        let to_upper = to.trim().to_uppercase();

        let mut best: Option<&ExchangeRate> = None;

        for rate in inner.rates.values() {
            if rate.tenant_id != tenant_id || rate.is_deleted() {
                continue;
            }
            if rate.from_currency != from_upper || rate.to_currency != to_upper {
                continue;
            }
            if rate.effective_date > date {
                continue;
            }
            // Pick the most recent effective rate
            if best.is_none() || rate.effective_date > best.unwrap().effective_date {
                best = Some(rate);
            }
        }

        Ok(best.cloned())
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        currency: Option<String>,
        date: Option<NaiveDate>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ExchangeRate>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<ExchangeRate> = inner
            .rates
            .values()
            .filter(|r| r.tenant_id == tenant_id && !r.is_deleted())
            .filter(|r| match &currency {
                Some(c) => {
                    let c_upper = c.trim().to_uppercase();
                    r.from_currency == c_upper || r.to_currency == c_upper
                }
                None => true,
            })
            .filter(|r| match date {
                Some(d) => r.effective_date == d,
                None => true,
            })
            .cloned()
            .collect();

        items.sort_by(|a, b| {
            a.from_currency
                .cmp(&b.from_currency)
                .then_with(|| a.to_currency.cmp(&b.to_currency))
                .then_with(|| b.effective_date.cmp(&a.effective_date))
        });
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<ExchangeRate> = items
            .into_iter()
            .skip(start as usize)
            .take(params.per_page as usize)
            .collect();
        Ok(PaginatedResult::new(
            paginated,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn list_effective_on(
        &self,
        tenant_id: i64,
        date: NaiveDate,
        params: PaginationParams,
    ) -> Result<PaginatedResult<ExchangeRate>, ApiError> {
        let inner = self.inner.lock();

        // For each unique from/to pair, find the most recent rate on or before date
        let mut best_rates: HashMap<(String, String), &ExchangeRate> = HashMap::new();

        for rate in inner.rates.values() {
            if rate.tenant_id != tenant_id || rate.effective_date > date || rate.is_deleted() {
                continue;
            }
            let key = (rate.from_currency.clone(), rate.to_currency.clone());
            match best_rates.get(&key) {
                Some(current) if current.effective_date >= rate.effective_date => {}
                _ => {
                    best_rates.insert(key, rate);
                }
            }
        }

        let mut items: Vec<ExchangeRate> = best_rates.into_values().cloned().collect();
        items.sort_by(|a, b| {
            a.from_currency
                .cmp(&b.from_currency)
                .then_with(|| a.to_currency.cmp(&b.to_currency))
        });
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<ExchangeRate> = items
            .into_iter()
            .skip(start as usize)
            .take(params.per_page as usize)
            .collect();
        Ok(PaginatedResult::new(
            paginated,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateExchangeRate,
    ) -> Result<ExchangeRate, ApiError> {
        let mut inner = self.inner.lock();

        let rate = inner
            .rates
            .get_mut(&id)
            .filter(|r| r.tenant_id == tenant_id && !r.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Exchange rate {} not found", id)))?;

        if let Some(r) = update.rate {
            rate.rate = r;
        }
        if let Some(effective_date) = update.effective_date {
            rate.effective_date = effective_date;
        }

        Ok(rate.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let rate = inner
            .rates
            .get(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Exchange rate {} not found", id)))?;

        let key = rate.id;
        inner.rates.remove(&key);
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let rate = inner
            .rates
            .get_mut(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Exchange rate {} not found", id)))?;

        if rate.is_deleted() {
            return Err(ApiError::Conflict(format!(
                "Exchange rate {} is already deleted",
                id
            )));
        }

        rate.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let rate = inner
            .rates
            .get_mut(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Exchange rate {} not found", id)))?;

        if !rate.is_deleted() {
            return Err(ApiError::BadRequest(format!(
                "Exchange rate {} is not deleted",
                id
            )));
        }

        rate.restore();
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<ExchangeRate>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .rates
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let len_before = inner.rates.len();
        inner
            .rates
            .retain(|_, r| !(r.id == id && r.tenant_id == tenant_id && r.is_deleted()));

        if inner.rates.len() == len_before {
            return Err(ApiError::NotFound(format!(
                "Deleted exchange rate {} not found",
                id
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[tokio::test]
    async fn test_currency_crud() {
        let repo = InMemoryCurrencyRepository::new();

        let create = CreateCurrency {
            code: "USD".to_string(),
            name: "US Dollar".to_string(),
            symbol: "$".to_string(),
            decimal_places: 2,
            is_active: true,
            is_base: false,
        };
        let currency = repo.create(create, 1).await.unwrap();
        assert_eq!(currency.id, 1);
        assert_eq!(currency.code, "USD");

        let found = repo.find_by_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.id, currency.id);

        let by_code = repo.find_by_code("usd", 1).await.unwrap().unwrap();
        assert_eq!(by_code.code, "USD");

        let not_found = repo.find_by_id(1, 999).await.unwrap();
        assert!(not_found.is_none());

        let update = UpdateCurrency {
            name: Some("US Dollar Updated".to_string()),
            symbol: None,
            decimal_places: None,
            is_active: None,
            is_base: Some(true),
        };
        let updated = repo.update(1, 1, update).await.unwrap();
        assert_eq!(updated.name, "US Dollar Updated");
        assert!(updated.is_base);

        repo.delete(1, 1).await.unwrap();
        let gone = repo.find_by_id(1, 1).await.unwrap();
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn test_find_base_currency() {
        let repo = InMemoryCurrencyRepository::new();

        let base = CreateCurrency {
            code: "TRY".to_string(),
            name: "Turkish Lira".to_string(),
            symbol: "₺".to_string(),
            decimal_places: 2,
            is_active: true,
            is_base: true,
        };
        repo.create(base, 1).await.unwrap();

        let found = repo.find_base(1).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().code, "TRY");

        let not_found = repo.find_base(2).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_exchange_rate_crud() {
        let repo = InMemoryExchangeRateRepository::new();

        let create = CreateExchangeRate {
            from_currency: "USD".to_string(),
            to_currency: "EUR".to_string(),
            rate: dec!(0.85),
            effective_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        let rate = repo.create(create, 1).await.unwrap();
        assert_eq!(rate.id, 1);
        assert_eq!(rate.from_currency, "USD");

        let found = repo.find_by_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.id, rate.id);

        let update = UpdateExchangeRate {
            rate: Some(dec!(0.86)),
            effective_date: None,
        };
        let updated = repo.update(1, 1, update).await.unwrap();
        assert_eq!(updated.rate, dec!(0.86));

        repo.delete(1, 1).await.unwrap();
        let gone = repo.find_by_id(1, 1).await.unwrap();
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn test_find_effective_rate() {
        let repo = InMemoryExchangeRateRepository::new();

        let create1 = CreateExchangeRate {
            from_currency: "USD".to_string(),
            to_currency: "EUR".to_string(),
            rate: dec!(0.85),
            effective_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        repo.create(create1, 1).await.unwrap();

        let create2 = CreateExchangeRate {
            from_currency: "USD".to_string(),
            to_currency: "EUR".to_string(),
            rate: dec!(0.88),
            effective_date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        };
        repo.create(create2, 1).await.unwrap();

        // Should find the older rate for Jan date
        let rate_jan = repo
            .find_effective_rate(
                "USD",
                "EUR",
                NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
                1,
            )
            .await
            .unwrap();
        assert!(rate_jan.is_some());
        assert_eq!(rate_jan.unwrap().rate, dec!(0.85));

        // Should find the newer rate for July date
        let rate_jul = repo
            .find_effective_rate(
                "USD",
                "EUR",
                NaiveDate::from_ymd_opt(2024, 7, 15).unwrap(),
                1,
            )
            .await
            .unwrap();
        assert!(rate_jul.is_some());
        assert_eq!(rate_jul.unwrap().rate, dec!(0.88));

        // Should not find for date before any rate
        let rate_early = repo
            .find_effective_rate(
                "USD",
                "EUR",
                NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                1,
            )
            .await
            .unwrap();
        assert!(rate_early.is_none());
    }

    #[tokio::test]
    async fn test_list_effective_on() {
        let repo = InMemoryExchangeRateRepository::new();

        let create1 = CreateExchangeRate {
            from_currency: "USD".to_string(),
            to_currency: "EUR".to_string(),
            rate: dec!(0.85),
            effective_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        };
        repo.create(create1, 1).await.unwrap();

        let create2 = CreateExchangeRate {
            from_currency: "USD".to_string(),
            to_currency: "EUR".to_string(),
            rate: dec!(0.88),
            effective_date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        };
        repo.create(create2, 1).await.unwrap();

        let create3 = CreateExchangeRate {
            from_currency: "TRY".to_string(),
            to_currency: "USD".to_string(),
            rate: dec!(0.032),
            effective_date: NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
        };
        repo.create(create3, 1).await.unwrap();

        let params = PaginationParams::default();
        let effective = repo
            .list_effective_on(1, NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(), params)
            .await
            .unwrap();
        // Should return one rate per unique pair (the most recent)
        assert_eq!(effective.items.len(), 2);

        // USD->EUR should be 0.88 (the newer one)
        let usd_eur = effective
            .items
            .iter()
            .find(|r| r.from_currency == "USD" && r.to_currency == "EUR")
            .unwrap();
        assert_eq!(usd_eur.rate, dec!(0.88));
    }
}
