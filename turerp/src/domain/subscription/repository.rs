//! Subscription repository

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;

use crate::common::SoftDeletable;
use crate::domain::subscription::model::{
    BillingCycle, CreatePlan, CreateSubscription, DunningEntry, DunningStatus, Subscription,
    SubscriptionInvoice, SubscriptionInvoiceStatus, SubscriptionPlan, SubscriptionStatus,
    UpdatePlan, UpdateSubscription, UsageRecord,
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

    // --- Dunning ---
    async fn create_dunning_entry(
        &self,
        tenant_id: i64,
        subscription_id: i64,
        invoice_id: i64,
        attempt_number: i32,
    ) -> Result<DunningEntry, ApiError>;
    async fn find_dunning_by_subscription(
        &self,
        subscription_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DunningEntry>, ApiError>;
    async fn update_dunning_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: DunningStatus,
        attempt_number: i32,
        retry_at: Option<DateTime<Utc>>,
    ) -> Result<DunningEntry, ApiError>;
    async fn find_subscriptions_for_dunning(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Subscription>, ApiError>;

    // --- Usage ---
    async fn create_usage_record(
        &self,
        tenant_id: i64,
        subscription_id: i64,
        quantity: i64,
        unit: String,
        billing_period_start: NaiveDate,
        billing_period_end: NaiveDate,
    ) -> Result<UsageRecord, ApiError>;
    async fn find_usage_by_subscription(
        &self,
        subscription_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<UsageRecord>, ApiError>;
    async fn find_usage_by_period(
        &self,
        subscription_id: i64,
        tenant_id: i64,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<Vec<UsageRecord>, ApiError>;

    // --- Trial ---
    async fn find_trial_subscriptions(&self, tenant_id: i64)
        -> Result<Vec<Subscription>, ApiError>;
}

/// Type alias for boxed repository
pub type BoxSubscriptionRepository = Arc<dyn SubscriptionRepository>;

struct InMemorySubscriptionInner {
    plans: HashMap<i64, SubscriptionPlan>,
    subscriptions: HashMap<i64, Subscription>,
    invoices: HashMap<i64, SubscriptionInvoice>,
    dunning_entries: HashMap<i64, DunningEntry>,
    usage_records: HashMap<i64, UsageRecord>,
    next_plan_id: i64,
    next_subscription_id: i64,
    next_invoice_id: i64,
    next_dunning_id: i64,
    next_usage_id: i64,
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
                dunning_entries: HashMap::new(),
                usage_records: HashMap::new(),
                next_plan_id: 1,
                next_subscription_id: 1,
                next_invoice_id: 1,
                next_dunning_id: 1,
                next_usage_id: 1,
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
            included_quantity: create.included_quantity,
            overage_rate: create.overage_rate,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
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
            .filter(|p| p.tenant_id == tenant_id && !p.is_deleted())
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
            .filter(|p| p.tenant_id == tenant_id && !p.is_deleted())
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
            .filter(|p| p.tenant_id == tenant_id && !p.is_deleted())
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
        if let Some(included_quantity) = update.included_quantity {
            plan.included_quantity = Some(included_quantity);
        }
        if let Some(overage_rate) = update.overage_rate {
            plan.overage_rate = Some(overage_rate);
        }

        plan.updated_at = Some(Utc::now());
        Ok(plan.clone())
    }

    async fn delete_plan(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let _plan = inner
            .plans
            .get(&id)
            .filter(|p| p.tenant_id == tenant_id && !p.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Plan {} not found", id)))?;

        // Prevent deleting a plan that has active subscriptions
        let has_subscriptions = inner
            .subscriptions
            .values()
            .any(|s| s.plan_id == id && s.tenant_id == tenant_id && !s.is_deleted());

        if has_subscriptions {
            return Err(ApiError::BadRequest(
                "Cannot delete plan with active subscriptions".to_string(),
            ));
        }

        inner.plans.get_mut(&id).unwrap().mark_deleted(0);
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
            trial_start_date: create.trial_start_date,
            trial_end_date: create.trial_end_date,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
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
            .filter(|s| s.tenant_id == tenant_id && !s.is_deleted())
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
            .filter(|s| s.tenant_id == tenant_id && !s.is_deleted())
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
                    && !s.is_deleted()
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
            .filter(|s| s.tenant_id == tenant_id && !s.is_deleted())
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
        if let Some(trial_start_date) = update.trial_start_date {
            sub.trial_start_date = Some(trial_start_date);
        }
        if let Some(trial_end_date) = update.trial_end_date {
            sub.trial_end_date = Some(trial_end_date);
        }

        sub.updated_at = Some(Utc::now());
        Ok(sub.clone())
    }

    async fn delete_subscription(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let sub = inner
            .subscriptions
            .get_mut(&id)
            .filter(|s| s.tenant_id == tenant_id && !s.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Subscription {} not found", id)))?;

        sub.mark_deleted(0);
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

    async fn create_dunning_entry(
        &self,
        tenant_id: i64,
        subscription_id: i64,
        invoice_id: i64,
        attempt_number: i32,
    ) -> Result<DunningEntry, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_dunning_id;
        inner.next_dunning_id += 1;

        let entry = DunningEntry {
            id,
            tenant_id,
            subscription_id,
            invoice_id,
            attempt_number,
            status: DunningStatus::Active,
            retry_at: None,
            created_at: Utc::now(),
            resolved_at: None,
        };

        inner.dunning_entries.insert(id, entry.clone());
        Ok(entry)
    }

    async fn find_dunning_by_subscription(
        &self,
        subscription_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DunningEntry>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .dunning_entries
            .values()
            .filter(|d| d.subscription_id == subscription_id && d.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn update_dunning_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: DunningStatus,
        attempt_number: i32,
        retry_at: Option<DateTime<Utc>>,
    ) -> Result<DunningEntry, ApiError> {
        let mut inner = self.inner.lock();

        let entry = inner
            .dunning_entries
            .get_mut(&id)
            .filter(|d| d.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Dunning entry {} not found", id)))?;

        entry.status = status;
        entry.attempt_number = attempt_number;
        if let Some(retry) = retry_at {
            entry.retry_at = Some(retry);
        }
        if status == DunningStatus::Resolved || status == DunningStatus::Failed {
            entry.resolved_at = Some(Utc::now());
        }

        Ok(entry.clone())
    }

    async fn find_subscriptions_for_dunning(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Subscription>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .subscriptions
            .values()
            .filter(|s| {
                s.tenant_id == tenant_id
                    && (s.status == SubscriptionStatus::PastDue
                        || s.status == SubscriptionStatus::Active)
                    && !s.is_deleted()
            })
            .cloned()
            .collect())
    }

    async fn create_usage_record(
        &self,
        tenant_id: i64,
        subscription_id: i64,
        quantity: i64,
        unit: String,
        billing_period_start: NaiveDate,
        billing_period_end: NaiveDate,
    ) -> Result<UsageRecord, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_usage_id;
        inner.next_usage_id += 1;

        let record = UsageRecord {
            id,
            tenant_id,
            subscription_id,
            record_type: crate::domain::subscription::model::UsageRecordType::Metered,
            quantity,
            unit,
            recorded_at: Utc::now(),
            billing_period_start,
            billing_period_end,
        };

        inner.usage_records.insert(id, record.clone());
        Ok(record)
    }

    async fn find_usage_by_subscription(
        &self,
        subscription_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<UsageRecord>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .usage_records
            .values()
            .filter(|u| u.subscription_id == subscription_id && u.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_usage_by_period(
        &self,
        subscription_id: i64,
        tenant_id: i64,
        period_start: NaiveDate,
        period_end: NaiveDate,
    ) -> Result<Vec<UsageRecord>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .usage_records
            .values()
            .filter(|u| {
                u.subscription_id == subscription_id
                    && u.tenant_id == tenant_id
                    && u.billing_period_start == period_start
                    && u.billing_period_end == period_end
            })
            .cloned()
            .collect())
    }

    async fn find_trial_subscriptions(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Subscription>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .subscriptions
            .values()
            .filter(|s| {
                s.tenant_id == tenant_id && s.status == SubscriptionStatus::Trial && !s.is_deleted()
            })
            .cloned()
            .collect())
    }
}
