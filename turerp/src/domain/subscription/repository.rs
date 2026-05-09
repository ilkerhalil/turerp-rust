//! Subscription repository

use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::subscription::model::{
    BillingCycle, CreatePlan, CreateSubscription, Subscription, SubscriptionInvoice,
    SubscriptionInvoiceStatus, SubscriptionPlan, SubscriptionStatus, UpdatePlan,
    UpdateSubscription,
};
use crate::error::ApiError;

/// Repository trait for subscription operations
#[async_trait]
pub trait SubscriptionRepository: Send + Sync {
    // --- Plans ---
    async fn create_plan(&self, create: CreatePlan) -> Result<SubscriptionPlan, ApiError>;
    async fn find_plan_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<SubscriptionPlan>, ApiError>;
    async fn find_plans_by_tenant(&self, tenant_id: i64)
        -> Result<Vec<SubscriptionPlan>, ApiError>;
    async fn update_plan(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdatePlan,
    ) -> Result<SubscriptionPlan, ApiError>;
    async fn delete_plan(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    // --- Subscriptions ---
    async fn create_subscription(
        &self,
        create: CreateSubscription,
    ) -> Result<Subscription, ApiError>;
    async fn find_subscription_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<Subscription>, ApiError>;
    async fn find_subscriptions_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Subscription>, ApiError>;
    async fn find_active_by_customer(
        &self,
        customer_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Subscription>, ApiError>;
    async fn update_subscription(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateSubscription,
    ) -> Result<Subscription, ApiError>;
    async fn delete_subscription(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    // --- Billing ---
    async fn find_due_for_billing(
        &self,
        tenant_id: i64,
        date: NaiveDate,
    ) -> Result<Vec<Subscription>, ApiError>;
    async fn renew_subscription(&self, id: i64, tenant_id: i64) -> Result<Subscription, ApiError>;

    // --- Invoices ---
    #[allow(clippy::too_many_arguments)]
    async fn create_subscription_invoice(
        &self,
        subscription_id: i64,
        tenant_id: i64,
        invoice_id: Option<i64>,
        billing_period_start: NaiveDate,
        billing_period_end: NaiveDate,
        amount: Decimal,
        status: SubscriptionInvoiceStatus,
    ) -> Result<SubscriptionInvoice, ApiError>;
    async fn find_invoices_by_subscription(
        &self,
        subscription_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionInvoice>, ApiError>;
}

/// Type alias for boxed repository
pub type BoxSubscriptionRepository = Arc<dyn SubscriptionRepository>;

struct InMemorySubscriptionInner {
    plans: HashMap<i64, SubscriptionPlan>,
    subscriptions: HashMap<i64, Subscription>,
    invoices: HashMap<i64, SubscriptionInvoice>,
    next_plan_id: i64,
    next_subscription_id: i64,
    next_invoice_id: i64,
}

/// In-memory subscription repository for testing
pub struct InMemorySubscriptionRepository {
    inner: Mutex<InMemorySubscriptionInner>,
}

impl InMemorySubscriptionRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemorySubscriptionInner {
                plans: HashMap::new(),
                subscriptions: HashMap::new(),
                invoices: HashMap::new(),
                next_plan_id: 1,
                next_subscription_id: 1,
                next_invoice_id: 1,
            }),
        }
    }
}

impl Default for InMemorySubscriptionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SubscriptionRepository for InMemorySubscriptionRepository {
    async fn create_plan(&self, create: CreatePlan) -> Result<SubscriptionPlan, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_plan_id;
        inner.next_plan_id += 1;

        let plan = SubscriptionPlan {
            id,
            tenant_id: create.tenant_id,
            name: create.name,
            description: create.description,
            billing_cycle: create.billing_cycle,
            base_amount: create.base_amount,
            currency: create.currency,
            features: create.features,
            is_active: create.is_active,
            created_at: Utc::now(),
            updated_at: None,
        };

        inner.plans.insert(id, plan.clone());
        Ok(plan)
    }

    async fn find_plan_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<SubscriptionPlan>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .plans
            .get(&id)
            .filter(|p| p.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_plans_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionPlan>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .plans
            .values()
            .filter(|p| p.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn update_plan(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdatePlan,
    ) -> Result<SubscriptionPlan, ApiError> {
        let mut inner = self.inner.lock();

        let plan = inner
            .plans
            .get_mut(&id)
            .filter(|p| p.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Plan {} not found", id)))?;

        if let Some(name) = update.name {
            plan.name = name;
        }
        if let Some(description) = update.description {
            plan.description = Some(description);
        }
        if let Some(billing_cycle) = update.billing_cycle {
            plan.billing_cycle = billing_cycle;
        }
        if let Some(base_amount) = update.base_amount {
            plan.base_amount = base_amount;
        }
        if let Some(currency) = update.currency {
            plan.currency = currency;
        }
        if let Some(features) = update.features {
            plan.features = Some(features);
        }
        if let Some(is_active) = update.is_active {
            plan.is_active = is_active;
        }

        plan.updated_at = Some(Utc::now());
        Ok(plan.clone())
    }

    async fn delete_plan(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        if !inner
            .plans
            .get(&id)
            .map(|p| p.tenant_id == tenant_id)
            .unwrap_or(false)
        {
            return Err(ApiError::NotFound(format!("Plan {} not found", id)));
        }

        // Prevent deleting a plan that has active subscriptions
        let has_subscriptions = inner
            .subscriptions
            .values()
            .any(|s| s.plan_id == id && s.tenant_id == tenant_id);

        if has_subscriptions {
            return Err(ApiError::BadRequest(
                "Cannot delete plan with active subscriptions".to_string(),
            ));
        }

        inner.plans.remove(&id);
        Ok(())
    }

    async fn create_subscription(
        &self,
        create: CreateSubscription,
    ) -> Result<Subscription, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_subscription_id;
        inner.next_subscription_id += 1;

        let sub = Subscription {
            id,
            tenant_id: create.tenant_id,
            customer_id: create.customer_id,
            plan_id: create.plan_id,
            start_date: create.start_date,
            end_date: create.end_date,
            status: create.status,
            auto_renew: create.auto_renew,
            last_billed_at: None,
            next_billing_date: create.next_billing_date,
            created_at: Utc::now(),
            updated_at: None,
        };

        inner.subscriptions.insert(id, sub.clone());
        Ok(sub)
    }

    async fn find_subscription_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<Subscription>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .subscriptions
            .get(&id)
            .filter(|s| s.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_subscriptions_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Subscription>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .subscriptions
            .values()
            .filter(|s| s.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_active_by_customer(
        &self,
        customer_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Subscription>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .subscriptions
            .values()
            .filter(|s| {
                s.customer_id == customer_id
                    && s.tenant_id == tenant_id
                    && s.status == SubscriptionStatus::Active
            })
            .cloned()
            .collect())
    }

    async fn update_subscription(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateSubscription,
    ) -> Result<Subscription, ApiError> {
        let mut inner = self.inner.lock();

        let sub = inner
            .subscriptions
            .get_mut(&id)
            .filter(|s| s.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Subscription {} not found", id)))?;

        if let Some(plan_id) = update.plan_id {
            sub.plan_id = plan_id;
        }
        if let Some(start_date) = update.start_date {
            sub.start_date = start_date;
        }
        if let Some(end_date) = update.end_date {
            sub.end_date = Some(end_date);
        }
        if let Some(status) = update.status {
            sub.status = status;
        }
        if let Some(auto_renew) = update.auto_renew {
            sub.auto_renew = auto_renew;
        }
        if let Some(next_billing_date) = update.next_billing_date {
            sub.next_billing_date = Some(next_billing_date);
        }

        sub.updated_at = Some(Utc::now());
        Ok(sub.clone())
    }

    async fn delete_subscription(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        if !inner
            .subscriptions
            .get(&id)
            .map(|s| s.tenant_id == tenant_id)
            .unwrap_or(false)
        {
            return Err(ApiError::NotFound(format!("Subscription {} not found", id)));
        }

        inner.subscriptions.remove(&id);
        // Also remove associated invoices
        inner
            .invoices
            .retain(|_, inv| inv.subscription_id != id || inv.tenant_id != tenant_id);
        Ok(())
    }

    async fn find_due_for_billing(
        &self,
        tenant_id: i64,
        date: NaiveDate,
    ) -> Result<Vec<Subscription>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .subscriptions
            .values()
            .filter(|s| {
                s.tenant_id == tenant_id
                    && s.auto_renew
                    && s.status == SubscriptionStatus::Active
                    && s.next_billing_date.map(|d| d <= date).unwrap_or(false)
            })
            .cloned()
            .collect())
    }

    async fn renew_subscription(&self, id: i64, tenant_id: i64) -> Result<Subscription, ApiError> {
        let mut inner = self.inner.lock();

        let plan_id = inner
            .subscriptions
            .get(&id)
            .filter(|s| s.tenant_id == tenant_id)
            .map(|s| s.plan_id)
            .ok_or_else(|| ApiError::NotFound(format!("Subscription {} not found", id)))?;

        let billing_cycle = inner
            .plans
            .get(&plan_id)
            .map(|p| p.billing_cycle)
            .ok_or_else(|| ApiError::NotFound(format!("Plan {} not found", plan_id)))?;

        let sub = inner
            .subscriptions
            .get_mut(&id)
            .filter(|s| s.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Subscription {} not found", id)))?;

        let now = Utc::now();
        sub.last_billed_at = Some(now);

        // Calculate next billing date based on plan cycle
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
        sub.next_billing_date = Some(new_next);

        // Extend end_date if present
        if let Some(end_date) = sub.end_date {
            let new_end = match billing_cycle {
                BillingCycle::Monthly => end_date
                    .checked_add_months(chrono::Months::new(1))
                    .unwrap_or(end_date),
                BillingCycle::Quarterly => end_date
                    .checked_add_months(chrono::Months::new(3))
                    .unwrap_or(end_date),
                BillingCycle::Yearly => end_date
                    .checked_add_months(chrono::Months::new(12))
                    .unwrap_or(end_date),
            };
            sub.end_date = Some(new_end);
        }

        sub.updated_at = Some(now);
        Ok(sub.clone())
    }

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
        let mut inner = self.inner.lock();
        let id = inner.next_invoice_id;
        inner.next_invoice_id += 1;

        let inv = SubscriptionInvoice {
            id,
            tenant_id,
            subscription_id,
            invoice_id,
            billing_period_start,
            billing_period_end,
            amount,
            status,
            created_at: Utc::now(),
        };

        inner.invoices.insert(id, inv.clone());
        Ok(inv)
    }

    async fn find_invoices_by_subscription(
        &self,
        subscription_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionInvoice>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .invoices
            .values()
            .filter(|inv| inv.subscription_id == subscription_id && inv.tenant_id == tenant_id)
            .cloned()
            .collect())
    }
}
