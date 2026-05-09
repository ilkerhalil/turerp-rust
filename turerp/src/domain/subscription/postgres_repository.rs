//! PostgreSQL subscription repository implementation

use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::subscription::model::{
    BillingCycle, CreatePlan, CreateSubscription, Subscription, SubscriptionInvoice,
    SubscriptionInvoiceStatus, SubscriptionPlan, SubscriptionStatus, UpdatePlan,
    UpdateSubscription,
};
use crate::domain::subscription::repository::{BoxSubscriptionRepository, SubscriptionRepository};
use crate::error::ApiError;

// --- Row types ---

#[derive(Debug, FromRow)]
struct SubscriptionPlanRow {
    id: i64,
    tenant_id: i64,
    name: String,
    description: Option<String>,
    billing_cycle: String,
    base_amount: Decimal,
    currency: String,
    features: Option<serde_json::Value>,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<SubscriptionPlanRow> for SubscriptionPlan {
    fn from(row: SubscriptionPlanRow) -> Self {
        let billing_cycle = row.billing_cycle.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid billing_cycle '{}' in database: {}, defaulting to Monthly",
                row.billing_cycle,
                e
            );
            BillingCycle::default()
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            description: row.description,
            billing_cycle,
            base_amount: row.base_amount,
            currency: row.currency,
            features: row.features,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct SubscriptionRow {
    id: i64,
    tenant_id: i64,
    customer_id: i64,
    plan_id: i64,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    status: String,
    auto_renew: bool,
    last_billed_at: Option<chrono::DateTime<chrono::Utc>>,
    next_billing_date: Option<NaiveDate>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<SubscriptionRow> for Subscription {
    fn from(row: SubscriptionRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid subscription status '{}' in database: {}, defaulting to Trial",
                row.status,
                e
            );
            SubscriptionStatus::default()
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            customer_id: row.customer_id,
            plan_id: row.plan_id,
            start_date: row.start_date,
            end_date: row.end_date,
            status,
            auto_renew: row.auto_renew,
            last_billed_at: row.last_billed_at,
            next_billing_date: row.next_billing_date,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, FromRow)]
struct SubscriptionInvoiceRow {
    id: i64,
    tenant_id: i64,
    subscription_id: i64,
    invoice_id: Option<i64>,
    billing_period_start: NaiveDate,
    billing_period_end: NaiveDate,
    amount: Decimal,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<SubscriptionInvoiceRow> for SubscriptionInvoice {
    fn from(row: SubscriptionInvoiceRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid invoice status '{}' in database: {}, defaulting to Pending",
                row.status,
                e
            );
            SubscriptionInvoiceStatus::default()
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            subscription_id: row.subscription_id,
            invoice_id: row.invoice_id,
            billing_period_start: row.billing_period_start,
            billing_period_end: row.billing_period_end,
            amount: row.amount,
            status,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL subscription repository
pub struct PostgresSubscriptionRepository {
    pool: Arc<PgPool>,
}

impl PostgresSubscriptionRepository {
    /// Create a new PostgreSQL subscription repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxSubscriptionRepository {
        Arc::new(self) as BoxSubscriptionRepository
    }
}

#[async_trait]
impl SubscriptionRepository for PostgresSubscriptionRepository {
    // --- Plans ---

    async fn create_plan(&self, create: CreatePlan) -> Result<SubscriptionPlan, ApiError> {
        let billing_cycle = create.billing_cycle.to_string();

        let row: SubscriptionPlanRow = sqlx::query_as(
            r#"
            INSERT INTO subscription_plans (tenant_id, name, description, billing_cycle, base_amount, currency, features, is_active, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            RETURNING id, tenant_id, name, description, billing_cycle, base_amount, currency, features, is_active, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.name)
        .bind(&create.description)
        .bind(&billing_cycle)
        .bind(create.base_amount)
        .bind(&create.currency)
        .bind(&create.features)
        .bind(create.is_active)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SubscriptionPlan"))?;

        Ok(row.into())
    }

    async fn find_plan_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<SubscriptionPlan>, ApiError> {
        let result: Option<SubscriptionPlanRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, billing_cycle, base_amount, currency, features, is_active, created_at, updated_at
            FROM subscription_plans
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find subscription plan: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_plans_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionPlan>, ApiError> {
        let rows: Vec<SubscriptionPlanRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, billing_cycle, base_amount, currency, features, is_active, created_at, updated_at
            FROM subscription_plans
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to list subscription plans: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_plan(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdatePlan,
    ) -> Result<SubscriptionPlan, ApiError> {
        let billing_cycle_str = update.billing_cycle.map(|b| b.to_string());

        let row: SubscriptionPlanRow = sqlx::query_as(
            r#"
            UPDATE subscription_plans
            SET
                name = COALESCE($1, name),
                description = COALESCE($2, description),
                billing_cycle = COALESCE($3, billing_cycle),
                base_amount = COALESCE($4, base_amount),
                currency = COALESCE($5, currency),
                features = COALESCE($6, features),
                is_active = COALESCE($7, is_active),
                updated_at = NOW()
            WHERE id = $8 AND tenant_id = $9
            RETURNING id, tenant_id, name, description, billing_cycle, base_amount, currency, features, is_active, created_at, updated_at
            "#,
        )
        .bind(&update.name)
        .bind(&update.description)
        .bind(&billing_cycle_str)
        .bind(update.base_amount)
        .bind(&update.currency)
        .bind(&update.features)
        .bind(update.is_active)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SubscriptionPlan"))?;

        Ok(row.into())
    }

    async fn delete_plan(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM subscription_plans
            WHERE id = $1 AND tenant_id = $2
              AND NOT EXISTS (SELECT 1 FROM subscriptions WHERE plan_id = $1 AND tenant_id = $2)
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete subscription plan: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::BadRequest(
                "Plan not found or has active subscriptions".to_string(),
            ));
        }

        Ok(())
    }

    // --- Subscriptions ---

    async fn create_subscription(
        &self,
        create: CreateSubscription,
    ) -> Result<Subscription, ApiError> {
        let status = create.status.to_string();

        let row: SubscriptionRow = sqlx::query_as(
            r#"
            INSERT INTO subscriptions (tenant_id, customer_id, plan_id, start_date, end_date, status, auto_renew, last_billed_at, next_billing_date, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NULL, $8, NOW())
            RETURNING id, tenant_id, customer_id, plan_id, start_date, end_date, status, auto_renew, last_billed_at, next_billing_date, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(create.customer_id)
        .bind(create.plan_id)
        .bind(create.start_date)
        .bind(create.end_date)
        .bind(&status)
        .bind(create.auto_renew)
        .bind(create.next_billing_date)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Subscription"))?;

        Ok(row.into())
    }

    async fn find_subscription_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<Subscription>, ApiError> {
        let result: Option<SubscriptionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, customer_id, plan_id, start_date, end_date, status, auto_renew, last_billed_at, next_billing_date, created_at, updated_at
            FROM subscriptions
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find subscription: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_subscriptions_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Subscription>, ApiError> {
        let rows: Vec<SubscriptionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, customer_id, plan_id, start_date, end_date, status, auto_renew, last_billed_at, next_billing_date, created_at, updated_at
            FROM subscriptions
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to list subscriptions: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_active_by_customer(
        &self,
        customer_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Subscription>, ApiError> {
        let status = SubscriptionStatus::Active.to_string();

        let rows: Vec<SubscriptionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, customer_id, plan_id, start_date, end_date, status, auto_renew, last_billed_at, next_billing_date, created_at, updated_at
            FROM subscriptions
            WHERE customer_id = $1 AND tenant_id = $2 AND status = $3
            ORDER BY created_at DESC
            "#,
        )
        .bind(customer_id)
        .bind(tenant_id)
        .bind(&status)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find active subscriptions: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_subscription(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateSubscription,
    ) -> Result<Subscription, ApiError> {
        let status_str = update.status.map(|s| s.to_string());

        let row: SubscriptionRow = sqlx::query_as(
            r#"
            UPDATE subscriptions
            SET
                plan_id = COALESCE($1, plan_id),
                start_date = COALESCE($2, start_date),
                end_date = COALESCE($3, end_date),
                status = COALESCE($4, status),
                auto_renew = COALESCE($5, auto_renew),
                next_billing_date = COALESCE($6, next_billing_date),
                updated_at = NOW()
            WHERE id = $7 AND tenant_id = $8
            RETURNING id, tenant_id, customer_id, plan_id, start_date, end_date, status, auto_renew, last_billed_at, next_billing_date, created_at, updated_at
            "#,
        )
        .bind(update.plan_id)
        .bind(update.start_date)
        .bind(update.end_date)
        .bind(&status_str)
        .bind(update.auto_renew)
        .bind(update.next_billing_date)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Subscription"))?;

        Ok(row.into())
    }

    async fn delete_subscription(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM subscriptions
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete subscription: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Subscription not found".to_string()));
        }

        Ok(())
    }

    // --- Billing ---

    async fn find_due_for_billing(
        &self,
        tenant_id: i64,
        date: NaiveDate,
    ) -> Result<Vec<Subscription>, ApiError> {
        let status = SubscriptionStatus::Active.to_string();

        let rows: Vec<SubscriptionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, customer_id, plan_id, start_date, end_date, status, auto_renew, last_billed_at, next_billing_date, created_at, updated_at
            FROM subscriptions
            WHERE tenant_id = $1 AND status = $2 AND auto_renew = true
              AND next_billing_date IS NOT NULL AND next_billing_date <= $3
            ORDER BY next_billing_date ASC
            "#,
        )
        .bind(tenant_id)
        .bind(&status)
        .bind(date)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find subscriptions due for billing: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn renew_subscription(&self, id: i64, tenant_id: i64) -> Result<Subscription, ApiError> {
        // Get the subscription and plan billing cycle
        let sub: SubscriptionRow = sqlx::query_as(
            r#"
            SELECT s.id, s.tenant_id, s.customer_id, s.plan_id, s.start_date, s.end_date, s.status, s.auto_renew, s.last_billed_at, s.next_billing_date, s.created_at, s.updated_at
            FROM subscriptions s
            WHERE s.id = $1 AND s.tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Subscription"))?;

        let plan_row: (String,) = sqlx::query_as(
            r#"
            SELECT billing_cycle FROM subscription_plans WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(sub.plan_id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SubscriptionPlan"))?;

        let billing_cycle: BillingCycle = plan_row.0.parse().unwrap_or_default();
        let now = Utc::now();
        let current_next = sub.next_billing_date.unwrap_or_else(|| now.date_naive());

        let new_next = match billing_cycle {
            BillingCycle::Monthly => current_next
                .checked_add_months(chrono::Months::new(1))
                .unwrap_or(current_next),
            BillingCycle::Quarterly => current_next
                .checked_add_months(chrono::Months::new(3))
                .unwrap_or(current_next),
            BillingCycle::Yearly => current_next
                .checked_add_months(chrono::Months::new(12))
                .unwrap_or(current_next),
        };

        let new_end = sub.end_date.map(|end| match billing_cycle {
            BillingCycle::Monthly => end
                .checked_add_months(chrono::Months::new(1))
                .unwrap_or(end),
            BillingCycle::Quarterly => end
                .checked_add_months(chrono::Months::new(3))
                .unwrap_or(end),
            BillingCycle::Yearly => end
                .checked_add_months(chrono::Months::new(12))
                .unwrap_or(end),
        });

        let row: SubscriptionRow = sqlx::query_as(
            r#"
            UPDATE subscriptions
            SET
                last_billed_at = NOW(),
                next_billing_date = $1,
                end_date = $2,
                updated_at = NOW()
            WHERE id = $3 AND tenant_id = $4
            RETURNING id, tenant_id, customer_id, plan_id, start_date, end_date, status, auto_renew, last_billed_at, next_billing_date, created_at, updated_at
            "#,
        )
        .bind(new_next)
        .bind(new_end)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Subscription"))?;

        Ok(row.into())
    }

    // --- Invoices ---

    async fn create_subscription_invoice(
        &self,
        subscription_id: i64,
        tenant_id: i64,
        invoice_id: Option<i64>,
        billing_period_start: NaiveDate,
        billing_period_end: NaiveDate,
        amount: Decimal,
        status: SubscriptionInvoiceStatus,
    ) -> Result<SubscriptionInvoice, ApiError> {
        let status_str = status.to_string();

        let row: SubscriptionInvoiceRow = sqlx::query_as(
            r#"
            INSERT INTO subscription_invoices (tenant_id, subscription_id, invoice_id, billing_period_start, billing_period_end, amount, status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING id, tenant_id, subscription_id, invoice_id, billing_period_start, billing_period_end, amount, status, created_at
            "#,
        )
        .bind(tenant_id)
        .bind(subscription_id)
        .bind(invoice_id)
        .bind(billing_period_start)
        .bind(billing_period_end)
        .bind(amount)
        .bind(&status_str)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "SubscriptionInvoice"))?;

        Ok(row.into())
    }

    async fn find_invoices_by_subscription(
        &self,
        subscription_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionInvoice>, ApiError> {
        let rows: Vec<SubscriptionInvoiceRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, subscription_id, invoice_id, billing_period_start, billing_period_end, amount, status, created_at
            FROM subscription_invoices
            WHERE subscription_id = $1 AND tenant_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(subscription_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to list subscription invoices: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}
