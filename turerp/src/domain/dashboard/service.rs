//! Dashboard service for business logic and KPI orchestration

use crate::cache::CacheService;
use crate::cache::{cache_get, cache_key, cache_set};
use crate::domain::dashboard::model::{
    ChartData, ChartDataset, CreateWidgetConfig, DashboardFilter, DashboardWidgetConfig, KpiFormat,
    KpiName, KpiResponse, KpiWidget,
};
use crate::domain::dashboard::repository::BoxDashboardRepository;
use crate::error::ApiError;
use num_traits::ToPrimitive;
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;

/// TTL for dashboard cache entries (seconds)
const DASHBOARD_CACHE_TTL: u64 = 60;

/// Dashboard service
#[derive(Clone)]
pub struct DashboardService {
    repo: BoxDashboardRepository,
    cache: Arc<dyn CacheService>,
    widgets: Arc<Mutex<HashMap<i64, Vec<DashboardWidgetConfig>>>>,
    next_widget_id: Arc<Mutex<i64>>,
}

impl DashboardService {
    /// Create a new dashboard service
    pub fn new(repo: BoxDashboardRepository, cache: Arc<dyn CacheService>) -> Self {
        Self {
            repo,
            cache,
            widgets: Arc::new(Mutex::new(HashMap::new())),
            next_widget_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Get all KPIs for a tenant
    pub async fn get_all_kpis(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<KpiResponse, ApiError> {
        let cache_key = cache_key(tenant_id, "dashboard", "kpis");
        if let Some(cached) = cache_get::<KpiResponse>(&*self.cache, &cache_key).await? {
            return Ok(cached);
        }

        let revenue = self
            .get_kpi_widget(tenant_id, KpiName::Revenue, filter)
            .await?;
        let profit = self
            .get_kpi_widget(tenant_id, KpiName::Profit, filter)
            .await?;
        let cash_flow = self
            .get_kpi_widget(tenant_id, KpiName::CashFlow, filter)
            .await?;
        let stock_value = self
            .get_kpi_widget(tenant_id, KpiName::StockValue, filter)
            .await?;
        let customer_count = self
            .get_kpi_widget(tenant_id, KpiName::CustomerCount, filter)
            .await?;
        let ar_aging = self.repo.get_ar_aging(tenant_id, &[30, 60, 90]).await?;
        let ap_aging = self.repo.get_ap_aging(tenant_id, &[30, 60, 90]).await?;

        let response = KpiResponse {
            revenue,
            profit,
            cash_flow,
            ar_aging,
            ap_aging,
            stock_value,
            customer_count,
        };

        cache_set(
            &*self.cache,
            &cache_key,
            &response,
            Some(DASHBOARD_CACHE_TTL),
        )
        .await?;
        Ok(response)
    }

    /// Get a single KPI widget
    pub async fn get_kpi_widget(
        &self,
        tenant_id: i64,
        name: KpiName,
        filter: &DashboardFilter,
    ) -> Result<KpiWidget, ApiError> {
        let cache_key = cache_key(tenant_id, "dashboard", &format!("kpi:{}", name));
        if let Some(cached) = cache_get::<KpiWidget>(&*self.cache, &cache_key).await? {
            return Ok(cached);
        }

        let (value, previous_value, format) = match name {
            KpiName::Revenue => {
                let current = self.repo.get_revenue(tenant_id, filter).await?;
                let previous = self
                    .repo
                    .get_previous_period_revenue(tenant_id, filter)
                    .await?;
                (current, previous, KpiFormat::Currency)
            }
            KpiName::Profit => {
                let current = self.repo.get_profit(tenant_id, filter).await?;
                let previous = self
                    .repo
                    .get_previous_period_profit(tenant_id, filter)
                    .await?;
                (current, previous, KpiFormat::Currency)
            }
            KpiName::CashFlow => {
                let current = self.repo.get_cash_flow(tenant_id, filter).await?;
                let previous = self
                    .repo
                    .get_previous_period_cash_flow(tenant_id, filter)
                    .await?;
                (current, previous, KpiFormat::Currency)
            }
            KpiName::StockValue => {
                let current = self.repo.get_stock_value(tenant_id).await?;
                let previous = self.repo.get_previous_period_stock_value(tenant_id).await?;
                (current, previous, KpiFormat::Currency)
            }
            KpiName::CustomerCount => {
                let current = Decimal::from(self.repo.get_customer_count(tenant_id).await?);
                let previous = Decimal::from(
                    self.repo
                        .get_previous_period_customer_count(tenant_id)
                        .await?,
                );
                (current, previous, KpiFormat::Number)
            }
            KpiName::ArAging | KpiName::ApAging | KpiName::TopProducts => {
                return Err(ApiError::BadRequest(format!(
                    "KPI {} is not available as a single widget",
                    name
                )));
            }
        };

        let change_percent = calculate_change_percent(value, previous_value);

        let widget = KpiWidget {
            id: name.to_string(),
            name: format!("{:?}", name),
            value,
            previous_value,
            change_percent,
            format,
        };

        cache_set(&*self.cache, &cache_key, &widget, Some(DASHBOARD_CACHE_TTL)).await?;
        Ok(widget)
    }

    /// Get sales time-series chart data
    pub async fn get_sales_chart(
        &self,
        tenant_id: i64,
        period: &str,
    ) -> Result<ChartData, ApiError> {
        let cache_key = cache_key(tenant_id, "dashboard", &format!("sales:{}", period));
        if let Some(cached) = cache_get::<ChartData>(&*self.cache, &cache_key).await? {
            return Ok(cached);
        }

        let periods = self.repo.get_sales_by_period(tenant_id, period).await?;
        let labels: Vec<String> = periods.iter().map(|p| p.period.clone()).collect();
        let sales_data: Vec<Decimal> = periods.iter().map(|p| p.total_sales).collect();
        let profit_data: Vec<Decimal> = periods.iter().map(|p| p.profit).collect();

        let chart = ChartData {
            labels,
            datasets: vec![
                ChartDataset {
                    label: "Sales".to_string(),
                    data: sales_data,
                },
                ChartDataset {
                    label: "Profit".to_string(),
                    data: profit_data,
                },
            ],
        };

        cache_set(&*self.cache, &cache_key, &chart, Some(DASHBOARD_CACHE_TTL)).await?;
        Ok(chart)
    }

    /// Get revenue by category pie chart data
    pub async fn get_revenue_by_category_chart(
        &self,
        tenant_id: i64,
    ) -> Result<ChartData, ApiError> {
        let cache_key = cache_key(tenant_id, "dashboard", "revenue_by_category");
        if let Some(cached) = cache_get::<ChartData>(&*self.cache, &cache_key).await? {
            return Ok(cached);
        }

        let categories = self.repo.get_revenue_by_category(tenant_id).await?;
        let labels: Vec<String> = categories.iter().map(|c| c.category_name.clone()).collect();
        let data: Vec<Decimal> = categories.iter().map(|c| c.revenue).collect();

        let chart = ChartData {
            labels,
            datasets: vec![ChartDataset {
                label: "Revenue".to_string(),
                data,
            }],
        };

        cache_set(&*self.cache, &cache_key, &chart, Some(DASHBOARD_CACHE_TTL)).await?;
        Ok(chart)
    }

    /// Get top products bar chart data
    pub async fn get_top_products_chart(
        &self,
        tenant_id: i64,
        limit: i64,
    ) -> Result<ChartData, ApiError> {
        let cache_key = cache_key(tenant_id, "dashboard", &format!("top_products:{}", limit));
        if let Some(cached) = cache_get::<ChartData>(&*self.cache, &cache_key).await? {
            return Ok(cached);
        }

        let products = self.repo.get_top_products(tenant_id, limit).await?;
        let labels: Vec<String> = products.iter().map(|p| p.product_name.clone()).collect();
        let data: Vec<Decimal> = products.iter().map(|p| p.total_revenue).collect();

        let chart = ChartData {
            labels,
            datasets: vec![ChartDataset {
                label: "Revenue".to_string(),
                data,
            }],
        };

        cache_set(&*self.cache, &cache_key, &chart, Some(DASHBOARD_CACHE_TTL)).await?;
        Ok(chart)
    }

    /// Save a widget configuration
    pub async fn save_widget(
        &self,
        tenant_id: i64,
        create: CreateWidgetConfig,
    ) -> Result<DashboardWidgetConfig, ApiError> {
        let mut id_guard = self.next_widget_id.lock();
        let id = *id_guard;
        *id_guard += 1;
        drop(id_guard);

        let now = chrono::Utc::now();
        let widget = DashboardWidgetConfig {
            id,
            tenant_id,
            widget_type: create.widget_type,
            title: create.title,
            position: create.position,
            filter: create.filter,
            created_at: now,
            updated_at: now,
        };

        let mut widgets = self.widgets.lock();
        widgets.entry(tenant_id).or_default().push(widget.clone());
        drop(widgets);

        Ok(widget)
    }

    /// List saved widget configurations for a tenant
    pub async fn list_widgets(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<DashboardWidgetConfig>, ApiError> {
        let widgets = self.widgets.lock();
        Ok(widgets.get(&tenant_id).cloned().unwrap_or_default())
    }

    /// Delete a widget configuration
    pub async fn delete_widget(&self, tenant_id: i64, widget_id: i64) -> Result<(), ApiError> {
        let mut widgets = self.widgets.lock();
        let tenant_widgets = widgets.entry(tenant_id).or_default();
        let len_before = tenant_widgets.len();
        tenant_widgets.retain(|w| w.id != widget_id);
        if tenant_widgets.len() == len_before {
            return Err(ApiError::NotFound(format!(
                "Widget {} not found",
                widget_id
            )));
        }
        Ok(())
    }
}

/// Calculate the percentage change between a current and previous value.
fn calculate_change_percent(value: Decimal, previous_value: Decimal) -> f64 {
    if previous_value > Decimal::ZERO {
        ((value - previous_value) / previous_value)
            .to_f64()
            .unwrap_or(0.0)
            * 100.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::NoopCacheService;
    use crate::domain::dashboard::repository::InMemoryDashboardRepository;

    fn create_test_service() -> DashboardService {
        let repo = Arc::new(InMemoryDashboardRepository::new()) as BoxDashboardRepository;
        let cache = Arc::new(NoopCacheService);
        DashboardService::new(repo, cache)
    }

    #[test]
    fn test_calculate_change_percent_increase() {
        let current = Decimal::from(120);
        let previous = Decimal::from(100);
        let pct = calculate_change_percent(current, previous);
        assert!((pct - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_change_percent_decrease() {
        let current = Decimal::from(80);
        let previous = Decimal::from(100);
        let pct = calculate_change_percent(current, previous);
        assert!((pct - (-20.0)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_change_percent_zero_previous() {
        let current = Decimal::from(100);
        let previous = Decimal::ZERO;
        let pct = calculate_change_percent(current, previous);
        assert_eq!(pct, 0.0);
    }

    #[test]
    fn test_calculate_change_percent_same_value() {
        let current = Decimal::from(100);
        let previous = Decimal::from(100);
        let pct = calculate_change_percent(current, previous);
        assert!((pct - 0.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_save_and_list_widgets() {
        let service = create_test_service();
        let tenant_id = 1i64;

        let create = CreateWidgetConfig {
            widget_type: crate::domain::dashboard::model::WidgetType::Kpi,
            title: "Revenue KPI".to_string(),
            position: crate::domain::dashboard::model::WidgetPosition {
                x: 0,
                y: 0,
                w: 2,
                h: 1,
            },
            filter: Some(DashboardFilter {
                date_from: None,
                date_to: None,
                company_id: None,
                branch_id: None,
                product_category: None,
            }),
        };

        let widget = service
            .save_widget(tenant_id, create.clone())
            .await
            .unwrap();
        assert_eq!(widget.tenant_id, tenant_id);
        assert_eq!(widget.title, "Revenue KPI");
        assert_eq!(widget.id, 1);

        let widgets = service.list_widgets(tenant_id).await.unwrap();
        assert_eq!(widgets.len(), 1);
        assert_eq!(widgets[0].id, 1);
    }

    #[tokio::test]
    async fn test_delete_widget() {
        let service = create_test_service();
        let tenant_id = 1i64;

        let create = CreateWidgetConfig {
            widget_type: crate::domain::dashboard::model::WidgetType::Kpi,
            title: "Test".to_string(),
            position: crate::domain::dashboard::model::WidgetPosition {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            filter: Some(DashboardFilter {
                date_from: None,
                date_to: None,
                company_id: None,
                branch_id: None,
                product_category: None,
            }),
        };

        let widget = service.save_widget(tenant_id, create).await.unwrap();
        assert_eq!(widget.id, 1);

        service.delete_widget(tenant_id, widget.id).await.unwrap();
        let widgets = service.list_widgets(tenant_id).await.unwrap();
        assert!(widgets.is_empty());
    }

    #[tokio::test]
    async fn test_delete_widget_not_found() {
        let service = create_test_service();
        let tenant_id = 1i64;

        let result = service.delete_widget(tenant_id, 999).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_widgets_empty() {
        let service = create_test_service();
        let widgets = service.list_widgets(42).await.unwrap();
        assert!(widgets.is_empty());
    }

    #[tokio::test]
    async fn test_widget_ids_increment() {
        let service = create_test_service();
        let tenant_id = 1i64;

        let create = CreateWidgetConfig {
            widget_type: crate::domain::dashboard::model::WidgetType::BarChart,
            title: "First".to_string(),
            position: crate::domain::dashboard::model::WidgetPosition {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            },
            filter: Some(DashboardFilter {
                date_from: None,
                date_to: None,
                company_id: None,
                branch_id: None,
                product_category: None,
            }),
        };

        let w1 = service
            .save_widget(tenant_id, create.clone())
            .await
            .unwrap();
        let w2 = service
            .save_widget(tenant_id, create.clone())
            .await
            .unwrap();
        assert_eq!(w1.id, 1);
        assert_eq!(w2.id, 2);
    }
}
