//! PostgreSQL dashboard repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::dashboard::model::{
    AgingBucket, DashboardFilter, ExpenseSummary, RevenueByCategory, SalesPeriod, TopProduct,
};
use crate::domain::dashboard::repository::DashboardRepository;
use crate::error::ApiError;

/// PostgreSQL dashboard repository
pub struct PostgresDashboardRepository {
    pool: Arc<PgPool>,
}

impl PostgresDashboardRepository {
    /// Create a new PostgreSQL dashboard repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> Arc<dyn DashboardRepository> {
        Arc::new(self) as Arc<dyn DashboardRepository>
    }
}

/// Helper to compute previous period bounds from a filter
fn previous_period_bounds(
    filter: &DashboardFilter,
) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
    let (from, to) = match (filter.date_from, filter.date_to) {
        (Some(f), Some(t)) => {
            let duration = t - f;
            let prev_to = f;
            let prev_from = f - duration;
            (Some(prev_from), Some(prev_to))
        }
        (Some(f), None) => {
            let duration = Utc::now() - f;
            let prev_to = f;
            let prev_from = f - duration;
            (Some(prev_from), Some(prev_to))
        }
        (None, Some(t)) => {
            let duration = t - DateTime::UNIX_EPOCH;
            let prev_to = DateTime::UNIX_EPOCH;
            let prev_from = DateTime::UNIX_EPOCH - duration;
            (Some(prev_from), Some(prev_to))
        }
        (None, None) => {
            let now = Utc::now();
            let start_of_month = now
                .date_naive()
                .with_day(1)
                .unwrap_or_default()
                .and_hms_opt(0, 0, 0)
                .unwrap_or_default()
                .and_local_timezone(Utc)
                .single()
                .unwrap_or(now);
            let start_of_prev_month = (start_of_month - chrono::Duration::days(1))
                .date_naive()
                .with_day(1)
                .unwrap_or_default()
                .and_hms_opt(0, 0, 0)
                .unwrap_or_default()
                .and_local_timezone(Utc)
                .single()
                .unwrap_or(now);
            (Some(start_of_prev_month), Some(start_of_month))
        }
    };
    (from, to)
}

#[async_trait]
impl DashboardRepository for PostgresDashboardRepository {
    async fn get_revenue(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        let row: (Decimal,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(i.total_amount), 0.0)
            FROM invoices i
            WHERE i.tenant_id = $1
              AND i.invoice_type = 'SalesInvoice'
              AND i.status IN ('Approved', 'Sent', 'PartiallyPaid', 'Paid')
              AND i.deleted_at IS NULL
              AND ($2::timestamptz IS NULL OR i.issue_date >= $2)
              AND ($3::timestamptz IS NULL OR i.issue_date <= $3)
            "#,
        )
        .bind(tenant_id)
        .bind(filter.date_from)
        .bind(filter.date_to)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Revenue"))?;
        Ok(row.0)
    }

    async fn get_profit(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        let row: (Decimal,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(
                il.line_total - (il.quantity * COALESCE(p.purchase_price, 0.0))
            ), 0.0)
            FROM invoice_lines il
            JOIN invoices i ON i.id = il.invoice_id
            LEFT JOIN products p ON p.id = il.product_id AND p.deleted_at IS NULL
            WHERE i.tenant_id = $1
              AND i.invoice_type = 'SalesInvoice'
              AND i.status IN ('Approved', 'Sent', 'PartiallyPaid', 'Paid')
              AND i.deleted_at IS NULL
              AND il.deleted_at IS NULL
              AND ($2::timestamptz IS NULL OR i.issue_date >= $2)
              AND ($3::timestamptz IS NULL OR i.issue_date <= $3)
            "#,
        )
        .bind(tenant_id)
        .bind(filter.date_from)
        .bind(filter.date_to)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Profit"))?;
        Ok(row.0)
    }

    async fn get_cash_flow(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        let row: (Decimal,) = sqlx::query_as(
            r#"
            WITH cash_in AS (
                SELECT COALESCE(SUM(p.amount), 0.0) as inflow
                FROM payments p
                JOIN invoices i ON i.id = p.invoice_id
                WHERE i.tenant_id = $1
                  AND i.invoice_type = 'SalesInvoice'
                  AND p.deleted_at IS NULL
                  AND i.deleted_at IS NULL
                  AND ($2::timestamptz IS NULL OR p.payment_date >= $2)
                  AND ($3::timestamptz IS NULL OR p.payment_date <= $3)
            ),
            cash_out AS (
                SELECT COALESCE(SUM(p.amount), 0.0) as outflow
                FROM payments p
                JOIN invoices i ON i.id = p.invoice_id
                WHERE i.tenant_id = $1
                  AND i.invoice_type = 'PurchaseInvoice'
                  AND p.deleted_at IS NULL
                  AND i.deleted_at IS NULL
                  AND ($4::timestamptz IS NULL OR p.payment_date >= $4)
                  AND ($5::timestamptz IS NULL OR p.payment_date <= $5)
            )
            SELECT ci.inflow - co.outflow
            FROM cash_in ci, cash_out co
            "#,
        )
        .bind(tenant_id)
        .bind(filter.date_from)
        .bind(filter.date_to)
        .bind(filter.date_from)
        .bind(filter.date_to)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "CashFlow"))?;
        Ok(row.0)
    }

    async fn get_ar_aging(
        &self,
        tenant_id: i64,
        _days_buckets: &[i32],
    ) -> Result<Vec<AgingBucket>, ApiError> {
        let rows: Vec<AgingBucketRow> = sqlx::query_as(
            r#"
            SELECT
                CASE
                    WHEN CURRENT_DATE - i.due_date::date <= 30 THEN '0-30'
                    WHEN CURRENT_DATE - i.due_date::date <= 60 THEN '31-60'
                    WHEN CURRENT_DATE - i.due_date::date <= 90 THEN '61-90'
                    ELSE '90+'
                END as bucket,
                COALESCE(SUM(i.total_amount - i.paid_amount), 0.0) as amount,
                COUNT(*) as count
            FROM invoices i
            WHERE i.tenant_id = $1
              AND i.invoice_type = 'SalesInvoice'
              AND i.status IN ('Approved', 'Sent', 'PartiallyPaid', 'Overdue')
              AND i.deleted_at IS NULL
              AND i.total_amount > i.paid_amount
            GROUP BY bucket
            ORDER BY MIN(i.due_date)
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "AR Aging"))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_ap_aging(
        &self,
        tenant_id: i64,
        _days_buckets: &[i32],
    ) -> Result<Vec<AgingBucket>, ApiError> {
        let rows: Vec<AgingBucketRow> = sqlx::query_as(
            r#"
            SELECT
                CASE
                    WHEN CURRENT_DATE - i.due_date::date <= 30 THEN '0-30'
                    WHEN CURRENT_DATE - i.due_date::date <= 60 THEN '31-60'
                    WHEN CURRENT_DATE - i.due_date::date <= 90 THEN '61-90'
                    ELSE '90+'
                END as bucket,
                COALESCE(SUM(i.total_amount - i.paid_amount), 0.0) as amount,
                COUNT(*) as count
            FROM invoices i
            WHERE i.tenant_id = $1
              AND i.invoice_type = 'PurchaseInvoice'
              AND i.status IN ('Approved', 'Sent', 'PartiallyPaid', 'Overdue')
              AND i.deleted_at IS NULL
              AND i.total_amount > i.paid_amount
            GROUP BY bucket
            ORDER BY MIN(i.due_date)
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "AP Aging"))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_stock_value(&self, tenant_id: i64) -> Result<Decimal, ApiError> {
        let row: (Decimal,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(sl.quantity * COALESCE(p.purchase_price, 0.0)), 0.0)
            FROM stock_levels sl
            JOIN products p ON p.id = sl.product_id AND p.deleted_at IS NULL
            JOIN warehouses w ON w.id = sl.warehouse_id AND w.deleted_at IS NULL
            WHERE w.tenant_id = $1
              AND sl.deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Stock Value"))?;
        Ok(row.0)
    }

    async fn get_top_products(
        &self,
        tenant_id: i64,
        limit: i64,
    ) -> Result<Vec<TopProduct>, ApiError> {
        let rows: Vec<TopProductRow> = sqlx::query_as(
            r#"
            SELECT
                p.id as product_id,
                p.name as product_name,
                COALESCE(SUM(il.quantity), 0.0) as total_quantity,
                COALESCE(SUM(il.line_total), 0.0) as total_revenue
            FROM invoice_lines il
            JOIN invoices i ON i.id = il.invoice_id
            JOIN products p ON p.id = il.product_id
            WHERE i.tenant_id = $1
              AND i.invoice_type = 'SalesInvoice'
              AND i.status IN ('Approved', 'Sent', 'PartiallyPaid', 'Paid')
              AND i.deleted_at IS NULL
              AND il.deleted_at IS NULL
              AND p.deleted_at IS NULL
            GROUP BY p.id, p.name
            ORDER BY total_revenue DESC
            LIMIT $2
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Top Products"))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_sales_by_period(
        &self,
        tenant_id: i64,
        period: &str,
    ) -> Result<Vec<SalesPeriod>, ApiError> {
        let trunc_expr = match period {
            "week" => "DATE_TRUNC('week', i.issue_date)",
            "day" => "DATE_TRUNC('day', i.issue_date)",
            _ => "DATE_TRUNC('month', i.issue_date)",
        };
        let fmt_expr = match period {
            "week" => "TO_CHAR(DATE_TRUNC('week', i.issue_date), 'YYYY-IW')",
            "day" => "TO_CHAR(DATE_TRUNC('day', i.issue_date), 'YYYY-MM-DD')",
            _ => "TO_CHAR(DATE_TRUNC('month', i.issue_date), 'YYYY-MM')",
        };
        let sql = format!(
            r#"
            SELECT
                {} as period,
                COALESCE(SUM(i.total_amount), 0.0) as total_sales,
                COALESCE(SUM(il.line_total - (il.quantity * COALESCE(p.purchase_price, 0.0))), 0.0) as total_cost,
                COALESCE(SUM(i.total_amount) - SUM(il.line_total - (il.quantity * COALESCE(p.purchase_price, 0.0))), 0.0) as profit
            FROM invoices i
            LEFT JOIN invoice_lines il ON il.invoice_id = i.id AND il.deleted_at IS NULL
            LEFT JOIN products p ON p.id = il.product_id AND p.deleted_at IS NULL
            WHERE i.tenant_id = $1
              AND i.invoice_type = 'SalesInvoice'
              AND i.status IN ('Approved', 'Sent', 'PartiallyPaid', 'Paid')
              AND i.deleted_at IS NULL
            GROUP BY {}
            ORDER BY {}
            "#,
            fmt_expr, trunc_expr, trunc_expr
        );
        let rows: Vec<SalesPeriodRow> = sqlx::query_as(&sql)
            .bind(tenant_id)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Sales by Period"))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_revenue_by_category(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<RevenueByCategory>, ApiError> {
        let rows: Vec<RevenueByCategoryRow> = sqlx::query_as(
            r#"
            SELECT
                COALESCE(c.id, 0) as category_id,
                COALESCE(c.name, 'Uncategorized') as category_name,
                COALESCE(SUM(il.line_total), 0.0) as revenue
            FROM invoice_lines il
            JOIN invoices i ON i.id = il.invoice_id
            LEFT JOIN products p ON p.id = il.product_id AND p.deleted_at IS NULL
            LEFT JOIN categories c ON c.id = p.category_id AND c.deleted_at IS NULL
            WHERE i.tenant_id = $1
              AND i.invoice_type = 'SalesInvoice'
              AND i.status IN ('Approved', 'Sent', 'PartiallyPaid', 'Paid')
              AND i.deleted_at IS NULL
              AND il.deleted_at IS NULL
            GROUP BY c.id, c.name
            ORDER BY revenue DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Revenue by Category"))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_customer_count(&self, tenant_id: i64) -> Result<i64, ApiError> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) as count
            FROM cari
            WHERE tenant_id = $1
              AND deleted_at IS NULL
              AND (cari_type = 'customer' OR cari_type = 'both')
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Customer Count"))?;
        Ok(row.0)
    }

    async fn get_expense_summary(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Vec<ExpenseSummary>, ApiError> {
        let rows: Vec<ExpenseSummaryRow> = sqlx::query_as(
            r#"
            SELECT
                'Purchases' as category,
                COALESCE(SUM(total_amount), 0.0) as amount
            FROM invoices
            WHERE tenant_id = $1
              AND invoice_type = 'PurchaseInvoice'
              AND status IN ('Approved', 'Sent', 'PartiallyPaid', 'Paid')
              AND deleted_at IS NULL
              AND ($2::timestamptz IS NULL OR issue_date >= $2)
              AND ($3::timestamptz IS NULL OR issue_date <= $3)
            "#,
        )
        .bind(tenant_id)
        .bind(filter.date_from)
        .bind(filter.date_to)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Expense Summary"))?;
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_previous_period_revenue(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        let (from, to) = previous_period_bounds(filter);
        let pf = DashboardFilter {
            date_from: from,
            date_to: to,
            company_id: filter.company_id,
            branch_id: filter.branch_id,
            product_category: filter.product_category,
        };
        self.get_revenue(tenant_id, &pf).await
    }

    async fn get_previous_period_profit(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        let (from, to) = previous_period_bounds(filter);
        let pf = DashboardFilter {
            date_from: from,
            date_to: to,
            company_id: filter.company_id,
            branch_id: filter.branch_id,
            product_category: filter.product_category,
        };
        self.get_profit(tenant_id, &pf).await
    }

    async fn get_previous_period_cash_flow(
        &self,
        tenant_id: i64,
        filter: &DashboardFilter,
    ) -> Result<Decimal, ApiError> {
        let (from, to) = previous_period_bounds(filter);
        let pf = DashboardFilter {
            date_from: from,
            date_to: to,
            company_id: filter.company_id,
            branch_id: filter.branch_id,
            product_category: filter.product_category,
        };
        self.get_cash_flow(tenant_id, &pf).await
    }

    async fn get_previous_period_stock_value(&self, _tenant_id: i64) -> Result<Decimal, ApiError> {
        // Stock value doesn't have a time dimension in the current schema;
        // return zero to indicate no change
        Ok(Decimal::ZERO)
    }

    async fn get_previous_period_customer_count(&self, _tenant_id: i64) -> Result<i64, ApiError> {
        // Customer count is a point-in-time metric; return zero to indicate no change
        Ok(0)
    }
}

// ============================================================================
// Database rows
// ============================================================================

#[derive(Debug, FromRow)]
struct AgingBucketRow {
    bucket: String,
    amount: Decimal,
    count: i64,
}

impl From<AgingBucketRow> for AgingBucket {
    fn from(row: AgingBucketRow) -> Self {
        Self {
            bucket: row.bucket,
            amount: row.amount,
            count: row.count,
        }
    }
}

#[derive(Debug, FromRow)]
struct TopProductRow {
    product_id: i64,
    product_name: String,
    total_quantity: Decimal,
    total_revenue: Decimal,
}

impl From<TopProductRow> for TopProduct {
    fn from(row: TopProductRow) -> Self {
        Self {
            product_id: row.product_id,
            product_name: row.product_name,
            total_quantity: row.total_quantity,
            total_revenue: row.total_revenue,
        }
    }
}

#[derive(Debug, FromRow)]
struct SalesPeriodRow {
    period: String,
    total_sales: Decimal,
    total_cost: Decimal,
    profit: Decimal,
}

impl From<SalesPeriodRow> for SalesPeriod {
    fn from(row: SalesPeriodRow) -> Self {
        Self {
            period: row.period,
            total_sales: row.total_sales,
            total_cost: row.total_cost,
            profit: row.profit,
        }
    }
}

#[derive(Debug, FromRow)]
struct RevenueByCategoryRow {
    category_id: i64,
    category_name: String,
    revenue: Decimal,
}

impl From<RevenueByCategoryRow> for RevenueByCategory {
    fn from(row: RevenueByCategoryRow) -> Self {
        Self {
            category_id: row.category_id,
            category_name: row.category_name,
            revenue: row.revenue,
        }
    }
}

#[derive(Debug, FromRow)]
struct ExpenseSummaryRow {
    category: String,
    amount: Decimal,
}

impl From<ExpenseSummaryRow> for ExpenseSummary {
    fn from(row: ExpenseSummaryRow) -> Self {
        Self {
            category: row.category,
            amount: row.amount,
        }
    }
}
