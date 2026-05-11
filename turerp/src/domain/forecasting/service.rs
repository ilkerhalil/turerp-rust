//! Forecasting service with simple statistical algorithms

use chrono::{Datelike, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;

use crate::domain::forecasting::model::{
    DemandDataPoint, DemandForecast, ForecastPeriod, ForecastReport, ForecastRequest,
    ReorderRequest, ReorderSuggestion, ReorderUrgency, StockAlert, StockAlertRequest,
    StockAlertType,
};
use crate::domain::forecasting::repository::{BoxForecastingRepository, HistoricalSale};
use crate::error::ApiError;

/// Default lead time in days if not specified
const DEFAULT_LEAD_TIME_DAYS: i32 = 7;
/// Default safety stock factor (0.5 = 50% buffer)
const DEFAULT_SAFETY_FACTOR: Decimal = dec!(0.5);
/// Days of stock coverage considered "excess"
const EXCESS_STOCK_DAYS: Decimal = dec!(90.0);

/// Forecasting service
#[derive(Clone)]
pub struct ForecastingService {
    repo: BoxForecastingRepository,
}

impl ForecastingService {
    pub fn new(repo: BoxForecastingRepository) -> Self {
        Self { repo }
    }

    /// Generate a demand forecast using simple moving average
    pub async fn forecast_demand(
        &self,
        tenant_id: i64,
        request: ForecastRequest,
    ) -> Result<DemandForecast, ApiError> {
        request
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let sales = self
            .repo
            .get_historical_sales(
                tenant_id,
                Some(request.product_id),
                request.warehouse_id,
                request.history_days,
            )
            .await?;

        let product = self
            .repo
            .get_products(tenant_id)
            .await?
            .into_iter()
            .find(|p| p.id == request.product_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!("Product {} not found", request.product_id))
            })?;

        let data_points = aggregate_by_period(sales, request.period_type);
        let historical_average = calculate_moving_average(&data_points, request.period_type);
        let forecasted_quantity = historical_average * Decimal::from(request.periods);

        Ok(DemandForecast {
            product_id: request.product_id,
            product_name: product.name,
            forecast_period: request.period_type,
            periods_ahead: request.periods,
            forecasted_quantity,
            historical_average,
            historical_data_points: data_points,
            generated_at: Utc::now(),
        })
    }

    /// Generate reorder suggestions for all products in a warehouse
    pub async fn get_reorder_suggestions(
        &self,
        tenant_id: i64,
        request: ReorderRequest,
    ) -> Result<Vec<ReorderSuggestion>, ApiError> {
        let lead_time_days = request.lead_time_days.unwrap_or(DEFAULT_LEAD_TIME_DAYS);
        let safety_factor = request.safety_factor.unwrap_or(DEFAULT_SAFETY_FACTOR);

        let products = self.repo.get_products(tenant_id).await?;
        let stock_levels = self
            .repo
            .get_stock_levels(tenant_id, request.warehouse_id)
            .await?;

        let mut suggestions = Vec::new();

        for level in stock_levels {
            let product = products.iter().find(|p| p.id == level.product_id);
            let product_name = product
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Product {}", level.product_id));

            let sales = self
                .repo
                .get_historical_sales(
                    tenant_id,
                    Some(level.product_id),
                    Some(level.warehouse_id),
                    30,
                )
                .await?;

            let avg_daily_demand = calculate_avg_daily_demand(&sales, 30);
            let safety_stock = avg_daily_demand * Decimal::from(lead_time_days) * safety_factor;
            let reorder_point = avg_daily_demand * Decimal::from(lead_time_days) + safety_stock;
            let available_stock = level.quantity - level.reserved_quantity;

            let suggested_quantity = if available_stock < reorder_point {
                reorder_point + (avg_daily_demand * Decimal::from(lead_time_days)) - available_stock
            } else {
                Decimal::ZERO
            };

            let urgency = if available_stock <= safety_stock {
                ReorderUrgency::Critical
            } else if available_stock <= reorder_point * dec!(1.1) {
                ReorderUrgency::High
            } else if available_stock <= reorder_point * dec!(1.3) {
                ReorderUrgency::Medium
            } else {
                ReorderUrgency::Low
            };

            suggestions.push(ReorderSuggestion {
                product_id: level.product_id,
                product_name,
                warehouse_id: level.warehouse_id,
                current_stock: level.quantity,
                reserved_stock: level.reserved_quantity,
                available_stock,
                avg_daily_demand,
                lead_time_days,
                safety_stock,
                reorder_point,
                suggested_quantity: suggested_quantity.max(Decimal::ZERO),
                urgency,
            });
        }

        suggestions.sort_by(|a, b| {
            let urgency_order = |u: &ReorderUrgency| match u {
                ReorderUrgency::Critical => 0,
                ReorderUrgency::High => 1,
                ReorderUrgency::Medium => 2,
                ReorderUrgency::Low => 3,
            };
            urgency_order(&a.urgency).cmp(&urgency_order(&b.urgency))
        });

        Ok(suggestions)
    }

    /// Generate stock level alerts
    pub async fn get_stock_alerts(
        &self,
        tenant_id: i64,
        request: StockAlertRequest,
    ) -> Result<Vec<StockAlert>, ApiError> {
        let products = self.repo.get_products(tenant_id).await?;
        let stock_levels = self
            .repo
            .get_stock_levels(tenant_id, request.warehouse_id)
            .await?;

        let alert_types = request.alert_types.unwrap_or_else(|| {
            vec![
                StockAlertType::BelowSafetyStock,
                StockAlertType::NearReorderPoint,
                StockAlertType::ExcessStock,
            ]
        });

        let mut alerts = Vec::new();

        for level in stock_levels {
            let product = products.iter().find(|p| p.id == level.product_id);
            let product_name = product
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Product {}", level.product_id));

            let sales = self
                .repo
                .get_historical_sales(
                    tenant_id,
                    Some(level.product_id),
                    Some(level.warehouse_id),
                    30,
                )
                .await?;

            let avg_daily_demand = calculate_avg_daily_demand(&sales, 30);
            let safety_stock =
                avg_daily_demand * Decimal::from(DEFAULT_LEAD_TIME_DAYS) * DEFAULT_SAFETY_FACTOR;
            let reorder_point =
                avg_daily_demand * Decimal::from(DEFAULT_LEAD_TIME_DAYS) + safety_stock;
            let available_stock = level.quantity - level.reserved_quantity;

            if alert_types.contains(&StockAlertType::BelowSafetyStock)
                && available_stock <= safety_stock
            {
                alerts.push(StockAlert {
                    product_id: level.product_id,
                    product_name: product_name.clone(),
                    warehouse_id: level.warehouse_id,
                    current_stock: level.quantity,
                    safety_stock,
                    alert_type: StockAlertType::BelowSafetyStock,
                    message: format!(
                        "Stock ({}) is below safety stock ({})",
                        available_stock, safety_stock
                    ),
                });
            }

            if alert_types.contains(&StockAlertType::NearReorderPoint)
                && available_stock > safety_stock
                && available_stock <= reorder_point
            {
                alerts.push(StockAlert {
                    product_id: level.product_id,
                    product_name: product_name.clone(),
                    warehouse_id: level.warehouse_id,
                    current_stock: level.quantity,
                    safety_stock,
                    alert_type: StockAlertType::NearReorderPoint,
                    message: format!(
                        "Stock ({}) is near reorder point ({})",
                        available_stock, reorder_point
                    ),
                });
            }

            if alert_types.contains(&StockAlertType::ExcessStock)
                && avg_daily_demand > Decimal::ZERO
                && available_stock / avg_daily_demand >= EXCESS_STOCK_DAYS
            {
                alerts.push(StockAlert {
                    product_id: level.product_id,
                    product_name: product_name.clone(),
                    warehouse_id: level.warehouse_id,
                    current_stock: level.quantity,
                    safety_stock,
                    alert_type: StockAlertType::ExcessStock,
                    message: format!(
                        "Excess stock: {} days of coverage",
                        available_stock / avg_daily_demand
                    ),
                });
            }
        }

        Ok(alerts)
    }

    /// Generate a comprehensive forecast report for all products
    pub async fn get_forecast_report(
        &self,
        tenant_id: i64,
        warehouse_id: Option<i64>,
    ) -> Result<Vec<ForecastReport>, ApiError> {
        let products = self.repo.get_products(tenant_id).await?;
        let stock_levels = self.repo.get_stock_levels(tenant_id, warehouse_id).await?;

        let mut reports = Vec::new();

        for level in stock_levels {
            let product = products.iter().find(|p| p.id == level.product_id);
            let product_name = product
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Product {}", level.product_id));

            let sales = self
                .repo
                .get_historical_sales(
                    tenant_id,
                    Some(level.product_id),
                    Some(level.warehouse_id),
                    30,
                )
                .await?;

            let avg_daily_demand = calculate_avg_daily_demand(&sales, 30);
            let forecasted_demand = avg_daily_demand * Decimal::from(30);
            let available_stock = level.quantity - level.reserved_quantity;

            let stock_coverage_days = if avg_daily_demand > Decimal::ZERO {
                available_stock / avg_daily_demand
            } else {
                Decimal::ZERO
            };

            let safety_stock =
                avg_daily_demand * Decimal::from(DEFAULT_LEAD_TIME_DAYS) * DEFAULT_SAFETY_FACTOR;
            let reorder_point =
                avg_daily_demand * Decimal::from(DEFAULT_LEAD_TIME_DAYS) + safety_stock;

            let reorder_suggestion = if available_stock < reorder_point {
                let suggested_quantity = reorder_point + forecasted_demand - available_stock;
                Some(ReorderSuggestion {
                    product_id: level.product_id,
                    product_name: product_name.clone(),
                    warehouse_id: level.warehouse_id,
                    current_stock: level.quantity,
                    reserved_stock: level.reserved_quantity,
                    available_stock,
                    avg_daily_demand,
                    lead_time_days: DEFAULT_LEAD_TIME_DAYS,
                    safety_stock,
                    reorder_point,
                    suggested_quantity: suggested_quantity.max(Decimal::ZERO),
                    urgency: if available_stock <= safety_stock {
                        ReorderUrgency::Critical
                    } else {
                        ReorderUrgency::High
                    },
                })
            } else {
                None
            };

            let mut alerts = Vec::new();
            if available_stock <= safety_stock {
                alerts.push(StockAlert {
                    product_id: level.product_id,
                    product_name: product_name.clone(),
                    warehouse_id: level.warehouse_id,
                    current_stock: level.quantity,
                    safety_stock,
                    alert_type: StockAlertType::BelowSafetyStock,
                    message: format!(
                        "Stock ({}) is below safety stock ({})",
                        available_stock, safety_stock
                    ),
                });
            }

            reports.push(ForecastReport {
                product_id: level.product_id,
                product_name,
                warehouse_id: level.warehouse_id,
                current_stock: level.quantity,
                reserved_stock: level.reserved_quantity,
                available_stock,
                forecasted_demand,
                forecast_period: ForecastPeriod::Daily,
                periods_ahead: 30,
                stock_coverage_days,
                reorder_suggestion,
                alerts,
                generated_at: Utc::now(),
            });
        }

        Ok(reports)
    }
}

/// Aggregate historical sales into demand data points by period
fn aggregate_by_period(sales: Vec<HistoricalSale>, period: ForecastPeriod) -> Vec<DemandDataPoint> {
    let mut buckets: HashMap<String, Decimal> = HashMap::new();

    for sale in sales {
        let key = match period {
            ForecastPeriod::Daily => sale.sale_date.format("%Y-%m-%d").to_string(),
            ForecastPeriod::Weekly => {
                let iso_week = sale.sale_date.iso_week();
                format!("{}-W{}", iso_week.year(), iso_week.week())
            }
            ForecastPeriod::Monthly => sale.sale_date.format("%Y-%m").to_string(),
        };
        *buckets.entry(key).or_insert(Decimal::ZERO) += sale.quantity;
    }

    let mut points: Vec<DemandDataPoint> = buckets
        .into_values()
        .map(|quantity| DemandDataPoint {
            period_start: Utc::now(),
            quantity,
        })
        .collect();

    points.sort_by_key(|b| std::cmp::Reverse(b.period_start));
    points
}

/// Calculate simple moving average from data points
fn calculate_moving_average(data_points: &[DemandDataPoint], _period: ForecastPeriod) -> Decimal {
    if data_points.is_empty() {
        return Decimal::ZERO;
    }
    let total: Decimal = data_points.iter().map(|dp| dp.quantity).sum();
    total / Decimal::from(data_points.len() as i32)
}

/// Calculate average daily demand from historical sales
fn calculate_avg_daily_demand(sales: &[HistoricalSale], days: i32) -> Decimal {
    if sales.is_empty() || days <= 0 {
        return Decimal::ZERO;
    }
    let total: Decimal = sales.iter().map(|s| s.quantity).sum();
    total / Decimal::from(days)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::forecasting::repository::InMemoryForecastingRepository;
    use chrono::Duration;
    use std::sync::Arc;

    fn create_service() -> ForecastingService {
        let repo = Arc::new(InMemoryForecastingRepository::new()) as BoxForecastingRepository;
        ForecastingService::new(repo)
    }

    fn seed_test_data(repo: &InMemoryForecastingRepository) {
        repo.seed_product(1, 1, "Widget A");
        repo.seed_product(2, 1, "Widget B");

        let today = Utc::now();
        for i in 0..10 {
            repo.seed_sale(1, 1, dec!(5.0), today - Duration::days(i));
        }
        for i in 0..5 {
            repo.seed_sale(2, 1, dec!(10.0), today - Duration::days(i));
        }

        repo.seed_stock_level(1, 1, dec!(100.0), dec!(10.0));
        repo.seed_stock_level(1, 2, dec!(5.0), dec!(1.0));
    }

    #[tokio::test]
    async fn test_forecast_demand() {
        let repo = InMemoryForecastingRepository::new();
        seed_test_data(&repo);
        let service = ForecastingService::new(Arc::new(repo) as BoxForecastingRepository);

        let request = ForecastRequest {
            product_id: 1,
            warehouse_id: Some(1),
            periods: 4,
            period_type: ForecastPeriod::Daily,
            history_days: 30,
        };

        let forecast = service.forecast_demand(1, request).await.unwrap();
        assert_eq!(forecast.product_id, 1);
        assert_eq!(forecast.product_name, "Widget A");
        assert!(forecast.forecasted_quantity > Decimal::ZERO);
        assert_eq!(forecast.historical_data_points.len(), 10);
    }

    #[tokio::test]
    async fn test_reorder_suggestions() {
        let repo = InMemoryForecastingRepository::new();
        seed_test_data(&repo);
        let service = ForecastingService::new(Arc::new(repo) as BoxForecastingRepository);

        let request = ReorderRequest {
            warehouse_id: Some(1),
            lead_time_days: Some(7),
            safety_factor: Some(dec!(0.5)),
        };

        let suggestions = service.get_reorder_suggestions(1, request).await.unwrap();
        assert!(!suggestions.is_empty());

        let critical = suggestions.iter().find(|s| s.product_id == 2);
        assert!(critical.is_some());
        let critical = critical.unwrap();
        assert_eq!(critical.urgency, ReorderUrgency::Critical);
        assert!(critical.suggested_quantity > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_stock_alerts() {
        let repo = InMemoryForecastingRepository::new();
        seed_test_data(&repo);
        let service = ForecastingService::new(Arc::new(repo) as BoxForecastingRepository);

        let request = StockAlertRequest {
            warehouse_id: Some(1),
            alert_types: Some(vec![StockAlertType::BelowSafetyStock]),
        };

        let alerts = service.get_stock_alerts(1, request).await.unwrap();
        assert!(!alerts.is_empty());

        let alert = alerts.iter().find(|a| a.product_id == 2);
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().alert_type, StockAlertType::BelowSafetyStock);
    }

    #[tokio::test]
    async fn test_forecast_report() {
        let repo = InMemoryForecastingRepository::new();
        seed_test_data(&repo);
        let service = ForecastingService::new(Arc::new(repo) as BoxForecastingRepository);

        let reports = service.get_forecast_report(1, Some(1)).await.unwrap();
        assert!(!reports.is_empty());

        let report = reports.iter().find(|r| r.product_id == 2).unwrap();
        assert!(report.reorder_suggestion.is_some());
        assert!(!report.alerts.is_empty());
    }

    #[tokio::test]
    async fn test_empty_sales_returns_zero_forecast() {
        let repo = InMemoryForecastingRepository::new();
        repo.seed_product(1, 1, "Widget A");
        repo.seed_stock_level(1, 1, dec!(100.0), Decimal::ZERO);
        let service = ForecastingService::new(Arc::new(repo) as BoxForecastingRepository);

        let request = ForecastRequest {
            product_id: 1,
            warehouse_id: Some(1),
            periods: 4,
            period_type: ForecastPeriod::Weekly,
            history_days: 30,
        };

        let forecast = service.forecast_demand(1, request).await.unwrap();
        assert_eq!(forecast.forecasted_quantity, Decimal::ZERO);
        assert_eq!(forecast.historical_average, Decimal::ZERO);
    }
}
