//! Tax domain models
//!
//! Provides types for Turkish tax management including KDV (VAT),
//! OIV, BSMV, stopaj, and corporate/income tax period tracking.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Turkish tax types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub enum TaxType {
    /// Katma Değer Vergisi (Value Added Tax)
    KDV,
    /// Özel İletişim Vergisi (Special Communication Tax)
    OIV,
    /// Banka ve Sigorta Muameleleri Vergisi (Banking and Insurance Transaction Tax)
    BSMV,
    /// Damga Vergisi (Stamp Tax)
    Damga,
    /// Gelir Vergisi Stopajı (Income Tax Withholding)
    Stopaj,
    /// Kurumlar Vergisi (Corporate Tax)
    KV,
    /// Gelir Vergisi (Income Tax)
    GV,
}

impl std::fmt::Display for TaxType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaxType::KDV => write!(f, "KDV"),
            TaxType::OIV => write!(f, "OIV"),
            TaxType::BSMV => write!(f, "BSMV"),
            TaxType::Damga => write!(f, "Damga"),
            TaxType::Stopaj => write!(f, "Stopaj"),
            TaxType::KV => write!(f, "KV"),
            TaxType::GV => write!(f, "GV"),
        }
    }
}

impl std::str::FromStr for TaxType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "KDV" => Ok(TaxType::KDV),
            "OIV" => Ok(TaxType::OIV),
            "BSMV" => Ok(TaxType::BSMV),
            "Damga" => Ok(TaxType::Damga),
            "Stopaj" => Ok(TaxType::Stopaj),
            "KV" => Ok(TaxType::KV),
            "GV" => Ok(TaxType::GV),
            _ => Err(format!("Invalid tax type: {}", s)),
        }
    }
}

/// Tax rate configuration per tenant and tax type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxRate {
    pub id: i64,
    pub tenant_id: i64,
    pub tax_type: TaxType,
    pub rate: Decimal,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub category: Option<String>,
    pub description: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

/// Result of a tax calculation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxCalculationResult {
    pub base_amount: Decimal,
    pub tax_type: TaxType,
    pub rate: Decimal,
    pub tax_amount: Decimal,
    pub inclusive: bool,
}

/// Status of a tax filing period
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema)]
pub enum TaxPeriodStatus {
    #[default]
    Open,
    Calculated,
    Filed,
    Closed,
}

impl std::fmt::Display for TaxPeriodStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaxPeriodStatus::Open => write!(f, "Open"),
            TaxPeriodStatus::Calculated => write!(f, "Calculated"),
            TaxPeriodStatus::Filed => write!(f, "Filed"),
            TaxPeriodStatus::Closed => write!(f, "Closed"),
        }
    }
}

impl std::str::FromStr for TaxPeriodStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Open" => Ok(TaxPeriodStatus::Open),
            "Calculated" => Ok(TaxPeriodStatus::Calculated),
            "Filed" => Ok(TaxPeriodStatus::Filed),
            "Closed" => Ok(TaxPeriodStatus::Closed),
            _ => Err(format!("Invalid tax period status: {}", s)),
        }
    }
}

/// A tax filing period (monthly)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxPeriod {
    pub id: i64,
    pub tenant_id: i64,
    pub tax_type: TaxType,
    pub period_year: i32,
    pub period_month: u32,
    pub total_base: Decimal,
    pub total_tax: Decimal,
    pub total_deduction: Decimal,
    pub net_tax: Decimal,
    pub status: TaxPeriodStatus,
    pub filed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Individual transaction detail within a tax period
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxPeriodDetail {
    pub id: i64,
    pub period_id: i64,
    pub transaction_date: NaiveDate,
    pub transaction_type: String,
    pub base_amount: Decimal,
    pub tax_rate: Decimal,
    pub tax_amount: Decimal,
    pub deduction_amount: Decimal,
    pub reference_id: Option<i64>,
}

// ---- DTOs ----

/// Create a new tax rate
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTaxRate {
    pub tax_type: TaxType,
    pub rate: Decimal,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub category: Option<String>,
    pub description: String,
    #[serde(default)]
    pub is_default: bool,
}

/// Update an existing tax rate
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateTaxRate {
    #[serde(default)]
    pub rate: Option<Decimal>,
    #[serde(default)]
    pub effective_to: Option<NaiveDate>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_default: Option<bool>,
}

/// Create a new tax period
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTaxPeriod {
    pub tax_type: TaxType,
    pub period_year: i32,
    pub period_month: u32,
}

/// Tax rate response (same fields as TaxRate)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxRateResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub tax_type: TaxType,
    pub rate: Decimal,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub category: Option<String>,
    pub description: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

impl From<TaxRate> for TaxRateResponse {
    fn from(rate: TaxRate) -> Self {
        Self {
            id: rate.id,
            tenant_id: rate.tenant_id,
            tax_type: rate.tax_type,
            rate: rate.rate,
            effective_from: rate.effective_from,
            effective_to: rate.effective_to,
            category: rate.category,
            description: rate.description,
            is_default: rate.is_default,
            created_at: rate.created_at,
        }
    }
}

/// Tax period response (same fields as TaxPeriod)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxPeriodResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub tax_type: TaxType,
    pub period_year: i32,
    pub period_month: u32,
    pub total_base: Decimal,
    pub total_tax: Decimal,
    pub total_deduction: Decimal,
    pub net_tax: Decimal,
    pub status: TaxPeriodStatus,
    pub filed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<TaxPeriod> for TaxPeriodResponse {
    fn from(period: TaxPeriod) -> Self {
        Self {
            id: period.id,
            tenant_id: period.tenant_id,
            tax_type: period.tax_type,
            period_year: period.period_year,
            period_month: period.period_month,
            total_base: period.total_base,
            total_tax: period.total_tax,
            total_deduction: period.total_deduction,
            net_tax: period.net_tax,
            status: period.status,
            filed_at: period.filed_at,
            created_at: period.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tax_type_display() {
        assert_eq!(TaxType::KDV.to_string(), "KDV");
        assert_eq!(TaxType::OIV.to_string(), "OIV");
        assert_eq!(TaxType::BSMV.to_string(), "BSMV");
        assert_eq!(TaxType::Damga.to_string(), "Damga");
        assert_eq!(TaxType::Stopaj.to_string(), "Stopaj");
        assert_eq!(TaxType::KV.to_string(), "KV");
        assert_eq!(TaxType::GV.to_string(), "GV");
    }

    #[test]
    fn test_tax_type_from_str() {
        assert_eq!("KDV".parse::<TaxType>().unwrap(), TaxType::KDV);
        assert_eq!("OIV".parse::<TaxType>().unwrap(), TaxType::OIV);
        assert_eq!("BSMV".parse::<TaxType>().unwrap(), TaxType::BSMV);
        assert_eq!("Damga".parse::<TaxType>().unwrap(), TaxType::Damga);
        assert_eq!("Stopaj".parse::<TaxType>().unwrap(), TaxType::Stopaj);
        assert_eq!("KV".parse::<TaxType>().unwrap(), TaxType::KV);
        assert_eq!("GV".parse::<TaxType>().unwrap(), TaxType::GV);
        assert!("INVALID".parse::<TaxType>().is_err());
    }

    #[test]
    fn test_tax_period_status_display() {
        assert_eq!(TaxPeriodStatus::Open.to_string(), "Open");
        assert_eq!(TaxPeriodStatus::Calculated.to_string(), "Calculated");
        assert_eq!(TaxPeriodStatus::Filed.to_string(), "Filed");
        assert_eq!(TaxPeriodStatus::Closed.to_string(), "Closed");
    }

    #[test]
    fn test_tax_period_status_from_str() {
        assert_eq!(
            "Open".parse::<TaxPeriodStatus>().unwrap(),
            TaxPeriodStatus::Open
        );
        assert_eq!(
            "Calculated".parse::<TaxPeriodStatus>().unwrap(),
            TaxPeriodStatus::Calculated
        );
        assert_eq!(
            "Filed".parse::<TaxPeriodStatus>().unwrap(),
            TaxPeriodStatus::Filed
        );
        assert_eq!(
            "Closed".parse::<TaxPeriodStatus>().unwrap(),
            TaxPeriodStatus::Closed
        );
        assert!("INVALID".parse::<TaxPeriodStatus>().is_err());
    }

    #[test]
    fn test_tax_rate_response_from_tax_rate() {
        let rate = TaxRate {
            id: 1,
            tenant_id: 100,
            tax_type: TaxType::KDV,
            rate: Decimal::new(20, 2), // 0.20 = 20%
            effective_from: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            effective_to: None,
            category: Some("KDV_ISTISNA".to_string()),
            description: "Standard KDV rate".to_string(),
            is_default: true,
            created_at: Utc::now(),
        };

        let resp = TaxRateResponse::from(rate);
        assert_eq!(resp.id, 1);
        assert_eq!(resp.tenant_id, 100);
        assert_eq!(resp.tax_type, TaxType::KDV);
    }

    #[test]
    fn test_tax_period_response_from_tax_period() {
        let period = TaxPeriod {
            id: 1,
            tenant_id: 100,
            tax_type: TaxType::KDV,
            period_year: 2024,
            period_month: 1,
            total_base: Decimal::new(100000, 2),
            total_tax: Decimal::new(20000, 2),
            total_deduction: Decimal::ZERO,
            net_tax: Decimal::new(20000, 2),
            status: TaxPeriodStatus::Open,
            filed_at: None,
            created_at: Utc::now(),
        };

        let resp = TaxPeriodResponse::from(period);
        assert_eq!(resp.id, 1);
        assert_eq!(resp.period_year, 2024);
        assert_eq!(resp.status, TaxPeriodStatus::Open);
    }

    #[test]
    fn test_tax_calculation_result() {
        let result = TaxCalculationResult {
            base_amount: Decimal::new(1000, 0),
            tax_type: TaxType::KDV,
            rate: Decimal::new(20, 2),
            tax_amount: Decimal::new(200, 0),
            inclusive: false,
        };
        assert_eq!(result.base_amount, Decimal::new(1000, 0));
        assert_eq!(result.tax_amount, Decimal::new(200, 0));
        assert!(!result.inclusive);
    }
}
