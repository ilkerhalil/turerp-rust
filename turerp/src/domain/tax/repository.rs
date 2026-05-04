//! Tax repository traits and in-memory implementations

use async_trait::async_trait;
use chrono::NaiveDate;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::tax::model::{
    CreateTaxRate, TaxPeriod, TaxPeriodDetail, TaxPeriodStatus, TaxRate, TaxType, UpdateTaxRate,
};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// TaxRateRepository
// ---------------------------------------------------------------------------

/// Repository trait for tax rate operations
#[async_trait]
pub trait TaxRateRepository: Send + Sync {
    /// Create a new tax rate
    async fn create(&self, rate: CreateTaxRate, tenant_id: i64) -> Result<TaxRate, ApiError>;

    /// Find a tax rate by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<TaxRate>, ApiError>;

    /// Find all tax rates with optional type filter and pagination
    async fn find_all(
        &self,
        tenant_id: i64,
        tax_type: Option<TaxType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<TaxRate>, ApiError>;

    /// Find the effective tax rate for a given type and date
    async fn find_effective(
        &self,
        tax_type: TaxType,
        date: NaiveDate,
        tenant_id: i64,
    ) -> Result<Option<TaxRate>, ApiError>;

    /// Update a tax rate
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateTaxRate,
    ) -> Result<TaxRate, ApiError>;

    /// Delete a tax rate
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type alias for boxed TaxRateRepository
pub type BoxTaxRateRepository = Arc<dyn TaxRateRepository>;

// ---------------------------------------------------------------------------
// InMemoryTaxRateRepository
// ---------------------------------------------------------------------------

struct RateInner {
    rates: HashMap<i64, TaxRate>,
    next_id: AtomicI64,
}

/// In-memory tax rate repository for testing and development
pub struct InMemoryTaxRateRepository {
    inner: Mutex<RateInner>,
}

impl InMemoryTaxRateRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RateInner {
                rates: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryTaxRateRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaxRateRepository for InMemoryTaxRateRepository {
    async fn create(&self, create: CreateTaxRate, tenant_id: i64) -> Result<TaxRate, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now();

        let rate = TaxRate {
            id,
            tenant_id,
            tax_type: create.tax_type,
            rate: create.rate,
            effective_from: create.effective_from,
            effective_to: create.effective_to,
            category: create.category,
            description: create.description,
            is_default: create.is_default,
            created_at: now,
        };

        inner.rates.insert(id, rate.clone());
        Ok(rate)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<TaxRate>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .rates
            .get(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        tax_type: Option<TaxType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<TaxRate>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<TaxRate> = inner
            .rates
            .values()
            .filter(|r| r.tenant_id == tenant_id)
            .filter(|r| match &tax_type {
                Some(tt) => r.tax_type == *tt,
                None => true,
            })
            .cloned()
            .collect();

        items.sort_by(|a, b| a.id.cmp(&b.id));
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<TaxRate> = items
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

    async fn find_effective(
        &self,
        tax_type: TaxType,
        date: NaiveDate,
        tenant_id: i64,
    ) -> Result<Option<TaxRate>, ApiError> {
        let inner = self.inner.lock();
        let mut best: Option<&TaxRate> = None;

        for rate in inner.rates.values() {
            if rate.tenant_id != tenant_id || rate.tax_type != tax_type {
                continue;
            }
            if rate.effective_from > date {
                continue;
            }
            if let Some(effective_to) = rate.effective_to {
                if effective_to < date {
                    continue;
                }
            }
            // Pick the most specific (latest effective_from) that is still effective
            if best.is_none() || rate.effective_from > best.unwrap().effective_from {
                best = Some(rate);
            }
        }

        Ok(best.cloned())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateTaxRate,
    ) -> Result<TaxRate, ApiError> {
        let mut inner = self.inner.lock();

        let rate = inner
            .rates
            .get_mut(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Tax rate {} not found", id)))?;

        if let Some(r) = update.rate {
            rate.rate = r;
        }
        if let Some(effective_to) = update.effective_to {
            rate.effective_to = Some(effective_to);
        }
        if let Some(category) = update.category {
            rate.category = Some(category);
        }
        if let Some(description) = update.description {
            rate.description = description;
        }
        if let Some(is_default) = update.is_default {
            rate.is_default = is_default;
        }

        Ok(rate.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let rate = inner
            .rates
            .get(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Tax rate {} not found", id)))?;

        let key = rate.id;
        inner.rates.remove(&key);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// TaxPeriodRepository
// ---------------------------------------------------------------------------

/// Repository trait for tax period operations
#[async_trait]
pub trait TaxPeriodRepository: Send + Sync {
    /// Create a new tax period
    async fn create(
        &self,
        tax_type: TaxType,
        year: i32,
        month: u32,
        tenant_id: i64,
    ) -> Result<TaxPeriod, ApiError>;

    /// Find a tax period by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<TaxPeriod>, ApiError>;

    /// Find all tax periods with optional type filter and pagination
    async fn find_all(
        &self,
        tenant_id: i64,
        tax_type: Option<TaxType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<TaxPeriod>, ApiError>;

    /// Update a tax period
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        period: TaxPeriod,
    ) -> Result<TaxPeriod, ApiError>;

    /// Add a detail line to a tax period
    async fn add_detail(&self, detail: TaxPeriodDetail) -> Result<TaxPeriodDetail, ApiError>;

    /// Get all detail lines for a tax period
    async fn get_details(&self, period_id: i64) -> Result<Vec<TaxPeriodDetail>, ApiError>;
}

/// Type alias for boxed TaxPeriodRepository
pub type BoxTaxPeriodRepository = Arc<dyn TaxPeriodRepository>;

// ---------------------------------------------------------------------------
// InMemoryTaxPeriodRepository
// ---------------------------------------------------------------------------

struct PeriodInner {
    periods: HashMap<i64, TaxPeriod>,
    details: HashMap<i64, Vec<TaxPeriodDetail>>,
    next_period_id: AtomicI64,
    next_detail_id: AtomicI64,
}

/// In-memory tax period repository for testing and development
pub struct InMemoryTaxPeriodRepository {
    inner: Mutex<PeriodInner>,
}

impl InMemoryTaxPeriodRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PeriodInner {
                periods: HashMap::new(),
                details: HashMap::new(),
                next_period_id: AtomicI64::new(1),
                next_detail_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryTaxPeriodRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TaxPeriodRepository for InMemoryTaxPeriodRepository {
    async fn create(
        &self,
        tax_type: TaxType,
        year: i32,
        month: u32,
        tenant_id: i64,
    ) -> Result<TaxPeriod, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_period_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now();

        let period = TaxPeriod {
            id,
            tenant_id,
            tax_type,
            period_year: year,
            period_month: month,
            total_base: rust_decimal::Decimal::ZERO,
            total_tax: rust_decimal::Decimal::ZERO,
            total_deduction: rust_decimal::Decimal::ZERO,
            net_tax: rust_decimal::Decimal::ZERO,
            status: TaxPeriodStatus::Open,
            filed_at: None,
            created_at: now,
        };

        inner.periods.insert(id, period.clone());
        inner.details.insert(id, Vec::new());
        Ok(period)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<TaxPeriod>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .periods
            .get(&id)
            .filter(|p| p.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        tax_type: Option<TaxType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<TaxPeriod>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<TaxPeriod> = inner
            .periods
            .values()
            .filter(|p| p.tenant_id == tenant_id)
            .filter(|p| match &tax_type {
                Some(tt) => p.tax_type == *tt,
                None => true,
            })
            .cloned()
            .collect();

        items.sort_by(|a, b| a.id.cmp(&b.id));
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<TaxPeriod> = items
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
        period: TaxPeriod,
    ) -> Result<TaxPeriod, ApiError> {
        let mut inner = self.inner.lock();

        let existing = inner
            .periods
            .get_mut(&id)
            .filter(|p| p.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Tax period {} not found", id)))?;

        *existing = period.clone();
        Ok(period)
    }

    async fn add_detail(&self, detail: TaxPeriodDetail) -> Result<TaxPeriodDetail, ApiError> {
        let mut inner = self.inner.lock();

        // Verify the period exists
        if !inner.periods.contains_key(&detail.period_id) {
            return Err(ApiError::NotFound(format!(
                "Tax period {} not found",
                detail.period_id
            )));
        }

        let id = inner.next_detail_id.fetch_add(1, Ordering::SeqCst);
        let stored = TaxPeriodDetail {
            id,
            ..detail.clone()
        };

        inner
            .details
            .entry(detail.period_id)
            .or_default()
            .push(stored.clone());

        Ok(stored)
    }

    async fn get_details(&self, period_id: i64) -> Result<Vec<TaxPeriodDetail>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.details.get(&period_id).cloned().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tax::model::{CreateTaxRate, TaxType, UpdateTaxRate};
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_tax_rate_crud() {
        let repo = InMemoryTaxRateRepository::new();

        // Create
        let create = CreateTaxRate {
            tax_type: TaxType::KDV,
            rate: Decimal::new(20, 2),
            effective_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            effective_to: None,
            category: None,
            description: "Standard KDV".to_string(),
            is_default: true,
        };
        let rate = repo.create(create, 1).await.unwrap();
        assert_eq!(rate.id, 1);
        assert_eq!(rate.tenant_id, 1);
        assert_eq!(rate.tax_type, TaxType::KDV);

        // Find by ID
        let found = repo.find_by_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.id, rate.id);

        // Not found for different tenant
        let not_found = repo.find_by_id(1, 999).await.unwrap();
        assert!(not_found.is_none());

        // Update
        let update = UpdateTaxRate {
            rate: Some(Decimal::new(20, 2)),
            effective_to: Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
            category: None,
            description: None,
            is_default: None,
        };
        let updated = repo.update(1, 1, update).await.unwrap();
        assert_eq!(updated.rate, Decimal::new(20, 2));
        assert!(updated.effective_to.is_some());

        // Delete
        repo.delete(1, 1).await.unwrap();
        let gone = repo.find_by_id(1, 1).await.unwrap();
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn test_find_effective_rate() {
        let repo = InMemoryTaxRateRepository::new();

        let create = CreateTaxRate {
            tax_type: TaxType::KDV,
            rate: Decimal::new(20, 2),
            effective_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            effective_to: None,
            category: None,
            description: "Standard KDV".to_string(),
            is_default: true,
        };
        repo.create(create, 1).await.unwrap();

        // Should find effective rate
        let effective = repo
            .find_effective(
                TaxType::KDV,
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                1,
            )
            .await
            .unwrap();
        assert!(effective.is_some());
        assert_eq!(effective.unwrap().tax_type, TaxType::KDV);

        // Should not find for date before effective_from
        let not_effective = repo
            .find_effective(
                TaxType::KDV,
                NaiveDate::from_ymd_opt(2023, 12, 31).unwrap(),
                1,
            )
            .await
            .unwrap();
        assert!(not_effective.is_none());

        // Should not find for different tax type
        let wrong_type = repo
            .find_effective(
                TaxType::OIV,
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                1,
            )
            .await
            .unwrap();
        assert!(wrong_type.is_none());
    }

    #[tokio::test]
    async fn test_tax_period_crud() {
        let repo = InMemoryTaxPeriodRepository::new();

        // Create
        let period = repo.create(TaxType::KDV, 2024, 1, 1).await.unwrap();
        assert_eq!(period.id, 1);
        assert_eq!(period.period_year, 2024);
        assert_eq!(period.period_month, 1);
        assert_eq!(period.status, TaxPeriodStatus::Open);

        // Find by ID
        let found = repo.find_by_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.id, period.id);

        // Add detail
        let detail = TaxPeriodDetail {
            id: 0,
            period_id: 1,
            transaction_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            transaction_type: "sales".to_string(),
            base_amount: Decimal::new(10000, 2),
            tax_rate: Decimal::new(20, 2),
            tax_amount: Decimal::new(2000, 2),
            deduction_amount: Decimal::ZERO,
            reference_id: None,
        };
        let stored = repo.add_detail(detail).await.unwrap();
        assert_eq!(stored.id, 1);

        // Get details
        let details = repo.get_details(1).await.unwrap();
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].base_amount, Decimal::new(10000, 2));
    }
}
