//! Dashboard domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Widget type enumeration for dashboard configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum WidgetType {
    Kpi,
    LineChart,
    BarChart,
    PieChart,
    Table,
    Gauge,
}

impl std::fmt::Display for WidgetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WidgetType::Kpi => write!(f, "kpi"),
            WidgetType::LineChart => write!(f, "line_chart"),
            WidgetType::BarChart => write!(f, "bar_chart"),
            WidgetType::PieChart => write!(f, "pie_chart"),
            WidgetType::Table => write!(f, "table"),
            WidgetType::Gauge => write!(f, "gauge"),
        }
    }
}

impl std::str::FromStr for WidgetType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "kpi" => Ok(WidgetType::Kpi),
            "line_chart" => Ok(WidgetType::LineChart),
            "bar_chart" => Ok(WidgetType::BarChart),
            "pie_chart" => Ok(WidgetType::PieChart),
            "table" => Ok(WidgetType::Table),
            "gauge" => Ok(WidgetType::Gauge),
            _ => Err(format!("Invalid widget type: {}", s)),
        }
    }
}

/// KPI widget data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KpiWidget {
    pub id: String,
    pub name: String,
    pub value: Decimal,
    pub previous_value: Decimal,
    pub change_percent: f64,
    pub format: KpiFormat,
}

/// KPI value formatting type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum KpiFormat {
    Currency,
    Percentage,
    Number,
    Decimal,
}

impl std::fmt::Display for KpiFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KpiFormat::Currency => write!(f, "currency"),
            KpiFormat::Percentage => write!(f, "percentage"),
            KpiFormat::Number => write!(f, "number"),
            KpiFormat::Decimal => write!(f, "decimal"),
        }
    }
}

impl std::str::FromStr for KpiFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "currency" => Ok(KpiFormat::Currency),
            "percentage" => Ok(KpiFormat::Percentage),
            "number" => Ok(KpiFormat::Number),
            "decimal" => Ok(KpiFormat::Decimal),
            _ => Err(format!("Invalid KPI format: {}", s)),
        }
    }
}

/// Chart dataset for time-series or categorical charts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChartDataset {
    pub label: String,
    pub data: Vec<Decimal>,
}

/// Chart data response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChartData {
    pub labels: Vec<String>,
    pub datasets: Vec<ChartDataset>,
}

/// Dashboard filter parameters
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardFilter {
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub company_id: Option<i64>,
    pub branch_id: Option<i64>,
    pub product_category: Option<i64>,
}

/// Dashboard widget configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardWidgetConfig {
    pub id: i64,
    pub tenant_id: i64,
    pub widget_type: WidgetType,
    pub title: String,
    pub position: WidgetPosition,
    pub filter: Option<DashboardFilter>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Widget position in dashboard grid
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WidgetPosition {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Request to create or update a widget configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWidgetConfig {
    pub widget_type: WidgetType,
    pub title: String,
    pub position: WidgetPosition,
    pub filter: Option<DashboardFilter>,
}

/// Aging bucket for AR/AP reports
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgingBucket {
    pub bucket: String,
    pub amount: Decimal,
    pub count: i64,
}

/// Top selling product entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopProduct {
    pub product_id: i64,
    pub product_name: String,
    pub total_quantity: Decimal,
    pub total_revenue: Decimal,
}

/// Sales period entry for time-series charts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SalesPeriod {
    pub period: String,
    pub total_sales: Decimal,
    pub total_cost: Decimal,
    pub profit: Decimal,
}

/// Revenue by category entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevenueByCategory {
    pub category_id: i64,
    pub category_name: String,
    pub revenue: Decimal,
}

/// Expense summary entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExpenseSummary {
    pub category: String,
    pub amount: Decimal,
}

/// KPI names for single KPI endpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum KpiName {
    Revenue,
    Profit,
    CashFlow,
    ArAging,
    ApAging,
    StockValue,
    CustomerCount,
    TopProducts,
}

impl std::fmt::Display for KpiName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KpiName::Revenue => write!(f, "revenue"),
            KpiName::Profit => write!(f, "profit"),
            KpiName::CashFlow => write!(f, "cash_flow"),
            KpiName::ArAging => write!(f, "ar_aging"),
            KpiName::ApAging => write!(f, "ap_aging"),
            KpiName::StockValue => write!(f, "stock_value"),
            KpiName::CustomerCount => write!(f, "customer_count"),
            KpiName::TopProducts => write!(f, "top_products"),
        }
    }
}

impl std::str::FromStr for KpiName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "revenue" => Ok(KpiName::Revenue),
            "profit" => Ok(KpiName::Profit),
            "cash_flow" => Ok(KpiName::CashFlow),
            "ar_aging" => Ok(KpiName::ArAging),
            "ap_aging" => Ok(KpiName::ApAging),
            "stock_value" => Ok(KpiName::StockValue),
            "customer_count" => Ok(KpiName::CustomerCount),
            "top_products" => Ok(KpiName::TopProducts),
            _ => Err(format!("Invalid KPI name: {}", s)),
        }
    }
}

/// All KPIs response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KpiResponse {
    pub revenue: KpiWidget,
    pub profit: KpiWidget,
    pub cash_flow: KpiWidget,
    pub ar_aging: Vec<AgingBucket>,
    pub ap_aging: Vec<AgingBucket>,
    pub stock_value: KpiWidget,
    pub customer_count: KpiWidget,
}
