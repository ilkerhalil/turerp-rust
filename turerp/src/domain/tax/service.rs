//! Tax service — business logic for tax rate management and period calculations

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::tax::model::{
    CreateTaxPeriod, CreateTaxRate, TaxCalculationResult, TaxPeriod, TaxPeriodDetail,
    TaxPeriodStatus, TaxRate, TaxType, UpdateTaxRate,
};
use crate::domain::tax::repository::{BoxTaxPeriodRepository, BoxTaxRateRepository};
use crate::error::ApiError;

/// Service for managing tax rates and tax periods
#[derive(Clone)]
pub struct TaxService {
    rate_repo: BoxTaxRateRepository,
    period_repo: BoxTaxPeriodRepository,
}

impl TaxService {
    pub fn new(rate_repo: BoxTaxRateRepository, period_repo: BoxTaxPeriodRepository) -> Self {
        Self {
            rate_repo,
            period_repo,
        }
    }

    // ---- Tax Rate Operations ----

    /// Create a new tax rate
    pub async fn create_tax_rate(
        &self,
        create: CreateTaxRate,
        tenant_id: i64,
    ) -> Result<TaxRate, ApiError> {
        if create.rate < Decimal::ZERO {
            return Err(ApiError::Validation(
                "Tax rate cannot be negative".to_string(),
            ));
        }
        if create.rate > Decimal::new(100, 0) {
            return Err(ApiError::Validation(
                "Tax rate cannot exceed 100%".to_string(),
            ));
        }
        self.rate_repo.create(create, tenant_id).await
    }

    /// Get a tax rate by ID
    pub async fn get_tax_rate(&self, id: i64, tenant_id: i64) -> Result<TaxRate, ApiError> {
        self.rate_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Tax rate {} not found", id)))
    }

    /// List tax rates with optional type filter and pagination
    pub async fn list_tax_rates(
        &self,
        tenant_id: i64,
        tax_type: Option<TaxType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<TaxRate>, ApiError> {
        self.rate_repo.find_all(tenant_id, tax_type, params).await
    }

    /// Update a tax rate
    pub async fn update_tax_rate(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateTaxRate,
    ) -> Result<TaxRate, ApiError> {
        if let Some(rate) = update.rate {
            if rate < Decimal::ZERO {
                return Err(ApiError::Validation(
                    "Tax rate cannot be negative".to_string(),
                ));
            }
            if rate > Decimal::new(100, 0) {
                return Err(ApiError::Validation(
                    "Tax rate cannot exceed 100%".to_string(),
                ));
            }
        }
        self.rate_repo.update(id, tenant_id, update).await
    }

    /// Delete a tax rate
    pub async fn delete_tax_rate(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        // Verify existence first
        self.rate_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Tax rate {} not found", id)))?;
        self.rate_repo.delete(id, tenant_id).await
    }

    /// Find the effective tax rate for a given type and date
    pub async fn get_effective_rate(
        &self,
        tax_type: TaxType,
        date: NaiveDate,
        tenant_id: i64,
    ) -> Result<TaxRate, ApiError> {
        let tt = tax_type.clone();
        self.rate_repo
            .find_effective(tax_type, date, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("No effective {} rate found for date {}", tt, date))
            })
    }

    /// Calculate tax on a base amount using the effective rate for a given date
    pub async fn calculate_tax(
        &self,
        tax_type: TaxType,
        base_amount: Decimal,
        date: NaiveDate,
        tenant_id: i64,
        inclusive: bool,
    ) -> Result<TaxCalculationResult, ApiError> {
        let tax_rate = self.get_effective_rate(tax_type, date, tenant_id).await?;
        let rate_value = tax_rate.rate;

        let tax_amount: Decimal = if inclusive {
            // Tax is already included in the base_amount
            // tax_amount = base_amount - (base_amount / (1 + rate))
            base_amount - (base_amount / (Decimal::ONE + rate_value))
        } else {
            // Tax is added on top of the base_amount
            base_amount * rate_value
        };

        Ok(TaxCalculationResult {
            base_amount,
            tax_type: tax_rate.tax_type.clone(),
            rate: rate_value,
            tax_amount: tax_amount.round_dp(2),
            inclusive,
        })
    }

    // ---- Tax Period Operations ----

    /// Create a new tax period
    pub async fn create_tax_period(
        &self,
        create: CreateTaxPeriod,
        tenant_id: i64,
    ) -> Result<TaxPeriod, ApiError> {
        if create.period_month < 1 || create.period_month > 12 {
            return Err(ApiError::Validation(
                "Period month must be between 1 and 12".to_string(),
            ));
        }
        self.period_repo
            .create(
                create.tax_type,
                create.period_year,
                create.period_month,
                tenant_id,
            )
            .await
    }

    /// List tax periods with optional type filter and pagination
    pub async fn list_tax_periods(
        &self,
        tenant_id: i64,
        tax_type: Option<TaxType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<TaxPeriod>, ApiError> {
        self.period_repo.find_all(tenant_id, tax_type, params).await
    }

    /// Get a tax period by ID
    pub async fn get_tax_period(&self, id: i64, tenant_id: i64) -> Result<TaxPeriod, ApiError> {
        self.period_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Tax period {} not found", id)))
    }

    /// Get all detail lines for a tax period
    pub async fn get_period_details(
        &self,
        period_id: i64,
    ) -> Result<Vec<TaxPeriodDetail>, ApiError> {
        self.period_repo.get_details(period_id).await
    }

    /// Calculate (recalculate) a tax period by summing all details
    pub async fn calculate_period(&self, id: i64, tenant_id: i64) -> Result<TaxPeriod, ApiError> {
        let mut period = self.get_tax_period(id, tenant_id).await?;

        // Only allow calculation on Open or Calculated periods
        if period.status == TaxPeriodStatus::Filed || period.status == TaxPeriodStatus::Closed {
            return Err(ApiError::BadRequest(format!(
                "Cannot recalculate period in {} status",
                period.status
            )));
        }

        let details = self.period_repo.get_details(period.id).await?;

        let total_base: Decimal = details.iter().map(|d| d.base_amount).sum();
        let total_tax: Decimal = details.iter().map(|d| d.tax_amount).sum();
        let total_deduction: Decimal = details.iter().map(|d| d.deduction_amount).sum();
        let net_tax = (total_tax - total_deduction).round_dp(2);

        period.total_base = total_base;
        period.total_tax = total_tax;
        period.total_deduction = total_deduction;
        period.net_tax = net_tax;
        period.status = TaxPeriodStatus::Calculated;

        self.period_repo.update(id, tenant_id, period).await
    }

    /// File a tax period (change status to Filed)
    pub async fn file_period(&self, id: i64, tenant_id: i64) -> Result<TaxPeriod, ApiError> {
        let mut period = self.get_tax_period(id, tenant_id).await?;

        if period.status != TaxPeriodStatus::Calculated {
            return Err(ApiError::BadRequest(format!(
                "Cannot file period in {} status; must be Calculated first",
                period.status
            )));
        }

        period.status = TaxPeriodStatus::Filed;
        period.filed_at = Some(chrono::Utc::now());

        self.period_repo.update(id, tenant_id, period).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::tax::repository::{InMemoryTaxPeriodRepository, InMemoryTaxRateRepository};
    use std::sync::Arc;

    fn make_service() -> TaxService {
        let rate_repo = Arc::new(InMemoryTaxRateRepository::new());
        let period_repo = Arc::new(InMemoryTaxPeriodRepository::new());
        TaxService::new(rate_repo, period_repo)
    }

    #[tokio::test]
    async fn test_create_and_get_tax_rate() {
        let svc = make_service();

        let create = CreateTaxRate {
            tax_type: TaxType::KDV,
            rate: Decimal::new(20, 2),
            effective_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            effective_to: None,
            category: None,
            description: "Standard KDV".to_string(),
            is_default: true,
        };

        let rate = svc.create_tax_rate(create, 1).await.unwrap();
        assert_eq!(rate.tax_type, TaxType::KDV);
        assert_eq!(rate.rate, Decimal::new(20, 2));

        let found = svc.get_tax_rate(rate.id, 1).await.unwrap();
        assert_eq!(found.id, rate.id);
    }

    #[tokio::test]
    async fn test_create_tax_rate_validation() {
        let svc = make_service();

        let negative = CreateTaxRate {
            tax_type: TaxType::KDV,
            rate: Decimal::new(-1, 0),
            effective_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            effective_to: None,
            category: None,
            description: "Bad".to_string(),
            is_default: false,
        };
        assert!(svc.create_tax_rate(negative, 1).await.is_err());

        let over_100 = CreateTaxRate {
            tax_type: TaxType::KDV,
            rate: Decimal::new(101, 0),
            effective_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            effective_to: None,
            category: None,
            description: "Bad".to_string(),
            is_default: false,
        };
        assert!(svc.create_tax_rate(over_100, 1).await.is_err());
    }

    #[tokio::test]
    async fn test_get_effective_rate() {
        let svc = make_service();

        let create = CreateTaxRate {
            tax_type: TaxType::KDV,
            rate: Decimal::new(20, 2),
            effective_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            effective_to: None,
            category: None,
            description: "Standard KDV".to_string(),
            is_default: true,
        };
        svc.create_tax_rate(create, 1).await.unwrap();

        let rate = svc
            .get_effective_rate(
                TaxType::KDV,
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                1,
            )
            .await
            .unwrap();
        assert_eq!(rate.rate, Decimal::new(20, 2));
    }

    #[tokio::test]
    async fn test_calculate_tax_exclusive() {
        let svc = make_service();

        let create = CreateTaxRate {
            tax_type: TaxType::KDV,
            rate: Decimal::new(20, 2),
            effective_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            effective_to: None,
            category: None,
            description: "Standard KDV".to_string(),
            is_default: true,
        };
        svc.create_tax_rate(create, 1).await.unwrap();

        let result = svc
            .calculate_tax(
                TaxType::KDV,
                Decimal::new(1000, 0),
                NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
                1,
                false,
            )
            .await
            .unwrap();

        assert_eq!(result.base_amount, Decimal::new(1000, 0));
        assert_eq!(result.rate, Decimal::new(20, 2));
        assert!(!result.inclusive);
        // 1000 * 0.20 = 200
        assert_eq!(result.tax_amount, Decimal::new(200, 0));
    }

    #[tokio::test]
    async fn test_tax_period_lifecycle() {
        let svc = make_service();

        // Create period
        let create = CreateTaxPeriod {
            tax_type: TaxType::KDV,
            period_year: 2024,
            period_month: 1,
        };
        let period = svc.create_tax_period(create, 1).await.unwrap();
        assert_eq!(period.status, TaxPeriodStatus::Open);

        // Add a detail
        let detail = TaxPeriodDetail {
            id: 0,
            period_id: period.id,
            transaction_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            transaction_type: "sales".to_string(),
            base_amount: Decimal::new(10000, 2),
            tax_rate: Decimal::new(20, 2),
            tax_amount: Decimal::new(2000, 2),
            deduction_amount: Decimal::ZERO,
            reference_id: None,
        };
        svc.period_repo.add_detail(detail).await.unwrap();

        // Calculate
        let calculated = svc.calculate_period(period.id, 1).await.unwrap();
        assert_eq!(calculated.status, TaxPeriodStatus::Calculated);
        assert_eq!(calculated.total_base, Decimal::new(10000, 2));
        assert_eq!(calculated.total_tax, Decimal::new(2000, 2));

        // File
        let filed = svc.file_period(period.id, 1).await.unwrap();
        assert_eq!(filed.status, TaxPeriodStatus::Filed);
        assert!(filed.filed_at.is_some());
    }

    #[tokio::test]
    async fn test_file_period_requires_calculated() {
        let svc = make_service();

        let create = CreateTaxPeriod {
            tax_type: TaxType::KDV,
            period_year: 2024,
            period_month: 1,
        };
        let period = svc.create_tax_period(create, 1).await.unwrap();

        // Cannot file an Open period
        let result = svc.file_period(period.id, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_tax_period_validation() {
        let svc = make_service();

        let bad_month = CreateTaxPeriod {
            tax_type: TaxType::KDV,
            period_year: 2024,
            period_month: 13,
        };
        assert!(svc.create_tax_period(bad_month, 1).await.is_err());

        let zero_month = CreateTaxPeriod {
            tax_type: TaxType::KDV,
            period_year: 2024,
            period_month: 0,
        };
        assert!(svc.create_tax_period(zero_month, 1).await.is_err());
    }

    #[tokio::test]
    async fn test_list_tax_rates() {
        let svc = make_service();

        let tax_types = [TaxType::KDV, TaxType::OIV, TaxType::BSMV];
        let descriptions = ["KDV", "OIV", "BSMV"];
        for (tt, desc) in tax_types.iter().zip(descriptions.iter()) {
            let create = CreateTaxRate {
                tax_type: tt.clone(),
                rate: Decimal::new(20, 2),
                effective_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                effective_to: None,
                category: None,
                description: format!("{} rate", desc),
                is_default: true,
            };
            svc.create_tax_rate(create, 1).await.unwrap();
        }

        let params = PaginationParams::default();
        let all = svc.list_tax_rates(1, None, params).await.unwrap();
        assert_eq!(all.items.len(), 3);

        let params = PaginationParams::default();
        let kdv_only = svc
            .list_tax_rates(1, Some(TaxType::KDV), params)
            .await
            .unwrap();
        assert_eq!(kdv_only.items.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_tax_rate() {
        let svc = make_service();

        let create = CreateTaxRate {
            tax_type: TaxType::KDV,
            rate: Decimal::new(20, 2),
            effective_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            effective_to: None,
            category: None,
            description: "Standard KDV".to_string(),
            is_default: true,
        };
        let rate = svc.create_tax_rate(create, 1).await.unwrap();

        svc.delete_tax_rate(rate.id, 1).await.unwrap();
        let result = svc.get_tax_rate(rate.id, 1).await;
        assert!(result.is_err());
    }
}
