//! Dashboard repository trait

use async_trait::async_trait;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::domain::dashboard::model::{
    AgingBucket, DashboardFilter, ExpenseSummary, RevenueByCategory, SalesPeriod, TopProduct,
};
use crate::error::ApiError;

/// Repository trait for BI dashboard aggregation queries
#[async_trait]
pub trait DashboardRepository: Send + Sync {
    /// Total revenue for the given tenant and date range
    async fn get_revenue(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError>;

    /// Gross profit for the given tenant and date range
    async fn get_profit(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError>;

    /// Net cash flow (payments received - payments made) for the given tenant and date range
    async fn get_cash_flow(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError>;

    /// Accounts receivable aging buckets
    async fn get_ar_aging(
        &self,
        tenant_id: i64,
        days_buckets: &[i32],
    ) -> Result<Vec<AgingBucket>, ApiError>;

    /// Accounts payable aging buckets
    async fn get_ap_aging(
        &self,
        tenant_id: i64,
        days_buckets: &[i32],
    ) -> Result<Vec<AgingBucket>, ApiError>;

    /// Total stock value (quantity * purchase_price)
    async fn get_stock_value(&self, tenant_id: i64) -> Result<Decimal, ApiError>;

    /// Top selling products by revenue
    async fn get_top_products(
        &self,
        tenant_id: i64,
        limit: i64,
    ) -> Result<Vec<TopProduct>, ApiError>;

    /// Sales time-series data grouped by period
    async fn get_sales_by_period(
        &self,
        tenant_id: i64,
        period: &str,
    ) -> Result<Vec<SalesPeriod>, ApiError>;

    /// Revenue breakdown by product category
    async fn get_revenue_by_category(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<RevenueByCategory>, ApiError>;

    /// Active customer count
    async fn get_customer_count(&self, tenant_id: i64) -> Result<i64, ApiError>;

    /// Expense summary from purchase invoices
    async fn get_expense_summary(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Vec<ExpenseSummary>, ApiError>;

    /// Previous period revenue for change percentage calculation
    async fn get_previous_period_revenue(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError>;

    /// Previous period profit for change percentage calculation
    async fn get_previous_period_profit(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError>;

    /// Previous period cash flow for change percentage calculation
    async fn get_previous_period_cash_flow(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError>;

    /// Previous period stock value for change percentage calculation
    async fn get_previous_period_stock_value(&self, tenant_id: i64) -> Result<Decimal, ApiError>;

    /// Previous period customer count for change percentage calculation
    async fn get_previous_period_customer_count(&self, tenant_id: i64) -> Result<i64, ApiError>;
}

/// Type alias for boxed dashboard repository
pub type BoxDashboardRepository = Arc<dyn DashboardRepository>;

/// In-memory dashboard repository (returns zeros/empty for testing)
pub struct InMemoryDashboardRepository;

impl InMemoryDashboardRepository {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InMemoryDashboardRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DashboardRepository for InMemoryDashboardRepository {
    async fn get_revenue(
        &self,
        _tenant_id: i64,
        _filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        Ok(Decimal::ZERO)
    }

    async fn get_profit(
        &self,
        _tenant_id: i64,
        _filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        Ok(Decimal::ZERO)
    }

    async fn get_cash_flow(
        &self,
        _tenant_id: i64,
        _filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        Ok(Decimal::ZERO)
    }

    async fn get_ar_aging(
        &self,
        _tenant_id: i64,
        _days_buckets: &[i32],
    ) -> Result<Vec<AgingBucket>, ApiError> {
        Ok(vec![])
    }

    async fn get_ap_aging(
        &self,
        _tenant_id: i64,
        _days_buckets: &[i32],
    ) -> Result<Vec<AgingBucket>, ApiError> {
        Ok(vec![])
    }

    async fn get_stock_value(&self, _tenant_id: i64) -> Result<Decimal, ApiError> {
        Ok(Decimal::ZERO)
    }

    async fn get_top_products(
        &self,
        _tenant_id: i64,
        _limit: i64,
    ) -> Result<Vec<TopProduct>, ApiError> {
        Ok(vec![])
    }

    async fn get_sales_by_period(
        &self,
        _tenant_id: i64,
        _period: &str,
    ) -> Result<Vec<SalesPeriod>, ApiError> {
        Ok(vec![])
    }

    async fn get_revenue_by_category(
        &self,
        _tenant_id: i64,
    ) -> Result<Vec<RevenueByCategory>, ApiError> {
        Ok(vec![])
    }

    async fn get_customer_count(&self, _tenant_id: i64) -> Result<i64, ApiError> {
        Ok(0)
    }

    async fn get_expense_summary(
        &self,
        _tenant_id: i64,
        _filter: &DashboardFilter,
    ) -> Result<Vec<ExpenseSummary>, ApiError> {
        Ok(vec![])
    }

    async fn get_previous_period_revenue(
        &self,
        _tenant_id: i64,
        _filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        Ok(Decimal::ZERO)
    }

    async fn get_previous_period_profit(
        &self,
        _tenant_id: i64,
        _filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        Ok(Decimal::ZERO)
    }

    async fn get_previous_period_cash_flow(
        &self,
        _tenant_id: i64,
        _filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        Ok(Decimal::ZERO)
    }

    async fn get_previous_period_stock_value(&self, _tenant_id: i64) -> Result<Decimal, ApiError> {
        Ok(Decimal::ZERO)
    }

    async fn get_previous_period_customer_count(&self, _tenant_id: i64) -> Result<i64, ApiError> {
        Ok(0)
    }
}
