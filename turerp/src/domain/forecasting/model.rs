//! Forecasting domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Time period for forecasting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ForecastPeriod {
    Daily,
    Weekly,
    Monthly,
}

impl std::fmt::Display for ForecastPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForecastPeriod::Daily => write!(f, "Daily"),
            ForecastPeriod::Weekly => write!(f, "Weekly"),
            ForecastPeriod::Monthly => write!(f, "Monthly"),
        }
    }
}

/// Historical demand data point
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DemandDataPoint {
    pub period_start: DateTime<Utc>,
    pub quantity: Decimal,
}

/// Demand forecast for a product
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DemandForecast {
    pub product_id: i64,
    pub product_name: String,
    pub forecast_period: ForecastPeriod,
    pub periods_ahead: i32,
    pub forecasted_quantity: Decimal,
    pub historical_average: Decimal,
    pub historical_data_points: Vec<DemandDataPoint>,
    pub generated_at: DateTime<Utc>,
}

/// Reorder suggestion for a product
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReorderSuggestion {
    pub product_id: i64,
    pub product_name: String,
    pub warehouse_id: i64,
    pub current_stock: Decimal,
    pub reserved_stock: Decimal,
    pub available_stock: Decimal,
    pub avg_daily_demand: Decimal,
    pub lead_time_days: i32,
    pub safety_stock: Decimal,
    pub reorder_point: Decimal,
    pub suggested_quantity: Decimal,
    pub urgency: ReorderUrgency,
}

/// Urgency level for reorder
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ReorderUrgency {
    Critical, // Stock below safety stock
    High,     // Stock near reorder point
    Medium,   // Stock approaching reorder point
    Low,      // Stock well above reorder point
}

impl std::fmt::Display for ReorderUrgency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReorderUrgency::Critical => write!(f, "Critical"),
            ReorderUrgency::High => write!(f, "High"),
            ReorderUrgency::Medium => write!(f, "Medium"),
            ReorderUrgency::Low => write!(f, "Low"),
        }
    }
}

/// Stock level alert
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockAlert {
    pub product_id: i64,
    pub product_name: String,
    pub warehouse_id: i64,
    pub current_stock: Decimal,
    pub safety_stock: Decimal,
    pub alert_type: StockAlertType,
    pub message: String,
}

/// Type of stock alert
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum StockAlertType {
    BelowSafetyStock,
    NearReorderPoint,
    ExcessStock,
}

impl std::fmt::Display for StockAlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StockAlertType::BelowSafetyStock => write!(f, "BelowSafetyStock"),
            StockAlertType::NearReorderPoint => write!(f, "NearReorderPoint"),
            StockAlertType::ExcessStock => write!(f, "ExcessStock"),
        }
    }
}

/// Forecast report combining demand and stock status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ForecastReport {
    pub product_id: i64,
    pub product_name: String,
    pub warehouse_id: i64,
    pub current_stock: Decimal,
    pub reserved_stock: Decimal,
    pub available_stock: Decimal,
    pub forecasted_demand: Decimal,
    pub forecast_period: ForecastPeriod,
    pub periods_ahead: i32,
    pub stock_coverage_days: Decimal,
    pub reorder_suggestion: Option<ReorderSuggestion>,
    pub alerts: Vec<StockAlert>,
    pub generated_at: DateTime<Utc>,
}

/// Request parameters for demand forecast
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ForecastRequest {
    pub product_id: i64,
    pub warehouse_id: Option<i64>,
    pub periods: i32,
    pub period_type: ForecastPeriod,
    pub history_days: i32,
}

impl ForecastRequest {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.periods <= 0 {
            errors.push("Periods must be positive".to_string());
        }
        if self.history_days <= 0 {
            errors.push("History days must be positive".to_string());
        }
        if self.history_days > 730 {
            errors.push("History days cannot exceed 730".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Request parameters for reorder suggestions
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReorderRequest {
    pub warehouse_id: Option<i64>,
    pub lead_time_days: Option<i32>,
    pub safety_factor: Option<Decimal>,
}

/// Request parameters for stock alerts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockAlertRequest {
    pub warehouse_id: Option<i64>,
    pub alert_types: Option<Vec<StockAlertType>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_forecast_request_validation() {
        let valid = ForecastRequest {
            product_id: 1,
            warehouse_id: Some(1),
            periods: 4,
            period_type: ForecastPeriod::Weekly,
            history_days: 90,
        };
        assert!(valid.validate().is_ok());

        let invalid = ForecastRequest {
            product_id: 1,
            warehouse_id: None,
            periods: 0,
            period_type: ForecastPeriod::Daily,
            history_days: 800,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_reorder_urgency_display() {
        assert_eq!(ReorderUrgency::Critical.to_string(), "Critical");
        assert_eq!(ReorderUrgency::Low.to_string(), "Low");
    }

    #[test]
    fn test_stock_alert_type_display() {
        assert_eq!(
            StockAlertType::BelowSafetyStock.to_string(),
            "BelowSafetyStock"
        );
        assert_eq!(StockAlertType::ExcessStock.to_string(), "ExcessStock");
    }
}
