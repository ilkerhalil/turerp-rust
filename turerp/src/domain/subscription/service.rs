//! Subscription service for business logic

use chrono::{NaiveDate, Utc};
use validator::Validate;

use crate::domain::cari::repository::BoxCariRepository;
use crate::domain::subscription::model::{
    CancelSubscriptionRequest, CancellationResult, CreatePlan, CreateSubscription,
    DunningEntryResponse, ProrationDirection, ProrationResult, RecordUsageRequest,
    SubscriptionInvoiceResponse, SubscriptionPlanResponse, SubscriptionResponse,
    SubscriptionStatus, TrialConversionResult, UpdatePlan, UpdateSubscription, UsageRecordResponse,
};
use crate::domain::subscription::repository::BoxSubscriptionRepository;
use crate::error::ApiError;

/// Subscription service
#[derive(Clone)]
pub struct SubscriptionService {
    repo: BoxSubscriptionRepository,
    cari_repo: BoxCariRepository,
}

impl SubscriptionService {
    /// Create a new subscription service
    pub fn new(repo: BoxSubscriptionRepository, cari_repo: BoxCariRepository) -> Self {
        Self { repo, cari_repo }
    }

    // --- Plans ---

    /// Create a subscription plan
    #[tracing::instrument(skip(self))]
    pub async fn create_plan(
        &self,
        create: CreatePlan,
    ) -> Result<SubscriptionPlanResponse, ApiError> {
        create.validate().map_err(|e| {
            tracing::warn!(tenant_id = create.tenant_id, error = %e, "Plan validation failed");
            ApiError::Validation(e.to_string())
        })?;
        let tenant_id = create.tenant_id;
        let plan = self.repo.create_plan(create).await?;
        tracing::info!(tenant_id, "Created subscription plan");
        Ok(plan.into())
    }

    /// Get a plan by ID
    #[tracing::instrument(skip(self))]
    pub async fn get_plan(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<SubscriptionPlanResponse, ApiError> {
        let plan = self
            .repo
            .find_plan_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| {
                tracing::warn!(tenant_id, plan_id = id, "Subscription plan not found");
                ApiError::NotFound(format!("Plan {} not found", id))
            })?;
        Ok(plan.into())
    }

    /// List all plans for a tenant
    #[tracing::instrument(skip(self))]
    pub async fn list_plans(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionPlanResponse>, ApiError> {
        let plans = self.repo.find_plans_by_tenant(tenant_id).await?;
        Ok(plans.into_iter().map(|p| p.into()).collect())
    }

    /// Update a plan
    #[tracing::instrument(skip(self))]
    pub async fn update_plan(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdatePlan,
    ) -> Result<SubscriptionPlanResponse, ApiError> {
        update
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;
        let plan = self.repo.update_plan(id, tenant_id, update).await?;
        tracing::info!(tenant_id, plan_id = id, "Updated subscription plan");
        Ok(plan.into())
    }

    /// Delete a plan
    #[tracing::instrument(skip(self))]
    pub async fn delete_plan(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.delete_plan(id, tenant_id).await?;
        tracing::info!(tenant_id, plan_id = id, "Deleted subscription plan");
        Ok(())
    }

    // --- Subscriptions ---

    /// Create a subscription
    #[tracing::instrument(skip(self))]
    pub async fn create_subscription(
        &self,
        create: CreateSubscription,
    ) -> Result<SubscriptionResponse, ApiError> {
        create.validate().map_err(|e| {
            tracing::warn!(tenant_id = create.tenant_id, error = %e, "Plan validation failed");
            ApiError::Validation(e.to_string())
        })?;
        // Parent-ownership precheck: the referenced plan must belong to the
        // caller's tenant (plan_id is a required FK).
        self.repo
            .find_plan_by_id(create.plan_id, create.tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Plan {} not found", create.plan_id)))?;
        // Parent-ownership precheck: the referenced customer (cari FK,
        // 026:22 NOT NULL REFERENCES cari(id)) must belong to the caller's
        // tenant. customer_id is a required FK, so the precheck is
        // unconditional (no None path).
        self.cari_repo
            .find_by_id(create.customer_id, create.tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Customer {} not found", create.customer_id))
            })?;
        let tenant_id = create.tenant_id;
        let sub = self.repo.create_subscription(create).await?;
        tracing::info!(tenant_id, "Created subscription");
        Ok(sub.into())
    }

    /// Get a subscription by ID
    #[tracing::instrument(skip(self))]
    pub async fn get_subscription(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<SubscriptionResponse, ApiError> {
        let sub = self
            .repo
            .find_subscription_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| {
                tracing::warn!(tenant_id, subscription_id = id, "Subscription not found");
                ApiError::NotFound(format!("Subscription {} not found", id))
            })?;
        Ok(sub.into())
    }

    /// List all subscriptions for a tenant
    #[tracing::instrument(skip(self))]
    pub async fn list_subscriptions(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionResponse>, ApiError> {
        let subs = self.repo.find_subscriptions_by_tenant(tenant_id).await?;
        Ok(subs.into_iter().map(|s| s.into()).collect())
    }

    /// List active subscriptions for a customer
    #[tracing::instrument(skip(self))]
    pub async fn list_active_by_customer(
        &self,
        customer_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionResponse>, ApiError> {
        let subs = self
            .repo
            .find_active_by_customer(customer_id, tenant_id)
            .await?;
        Ok(subs.into_iter().map(|s| s.into()).collect())
    }

    /// Update a subscription
    #[tracing::instrument(skip(self))]
    pub async fn update_subscription(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateSubscription,
    ) -> Result<SubscriptionResponse, ApiError> {
        update
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;
        let sub = self.repo.update_subscription(id, tenant_id, update).await?;
        tracing::info!(tenant_id, subscription_id = id, "Updated subscription");
        Ok(sub.into())
    }

    /// Delete a subscription
    #[tracing::instrument(skip(self))]
    pub async fn delete_subscription(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.delete_subscription(id, tenant_id).await?;
        tracing::info!(tenant_id, subscription_id = id, "Deleted subscription");
        Ok(())
    }

    // --- Billing ---

    /// Renew a subscription (extend by billing cycle)
    #[tracing::instrument(skip(self))]
    pub async fn renew_subscription(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<SubscriptionResponse, ApiError> {
        let sub = self.repo.renew_subscription(id, tenant_id).await?;

        // Create an invoice record for the renewal
        let plan = self
            .repo
            .find_plan_by_id(sub.plan_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Plan {} not found", sub.plan_id)))?;

        let billing_period_start = sub
            .last_billed_at
            .map(|dt| dt.date_naive())
            .unwrap_or(sub.start_date);
        let billing_period_end = sub.next_billing_date.ok_or_else(|| {
            ApiError::BadRequest(format!("Subscription {} has no next billing date", id))
        })?;

        self.repo
            .create_subscription_invoice(
                sub.id,
                tenant_id,
                None,
                billing_period_start,
                billing_period_end,
                plan.base_amount,
                crate::domain::subscription::model::SubscriptionInvoiceStatus::Pending,
            )
            .await?;
        tracing::info!(tenant_id, subscription_id = id, "Renewed subscription");

        Ok(sub.into())
    }

    /// Find subscriptions due for billing on or before a date
    #[tracing::instrument(skip(self))]
    pub async fn due_for_billing(
        &self,
        tenant_id: i64,
        date: NaiveDate,
    ) -> Result<Vec<SubscriptionResponse>, ApiError> {
        let subs = self.repo.find_due_for_billing(tenant_id, date).await?;
        Ok(subs.into_iter().map(|s| s.into()).collect())
    }

    /// Get invoices for a subscription
    #[tracing::instrument(skip(self))]
    pub async fn get_invoices(
        &self,
        subscription_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionInvoiceResponse>, ApiError> {
        let invoices = self
            .repo
            .find_invoices_by_subscription(subscription_id, tenant_id)
            .await?;
        Ok(invoices.into_iter().map(|inv| inv.into()).collect())
    }

    // --- Proration ---

    /// Calculate proration for a subscription plan change
    #[tracing::instrument(skip(self))]
    pub async fn calculate_proration(
        &self,
        subscription_id: i64,
        tenant_id: i64,
        new_plan_id: i64,
        effective_date: NaiveDate,
    ) -> Result<ProrationResult, ApiError> {
        let sub = self
            .repo
            .find_subscription_by_id(subscription_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Subscription {} not found", subscription_id))
            })?;

        let current_plan = self
            .repo
            .find_plan_by_id(sub.plan_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Plan {} not found", sub.plan_id)))?;

        let new_plan = self
            .repo
            .find_plan_by_id(new_plan_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Plan {} not found", new_plan_id)))?;

        let billing_period_start = sub
            .last_billed_at
            .map(|dt| dt.date_naive())
            .unwrap_or(sub.start_date);
        let billing_period_end = sub.next_billing_date.ok_or_else(|| {
            ApiError::BadRequest(format!(
                "Subscription {} has no next billing date",
                subscription_id
            ))
        })?;

        if billing_period_end <= billing_period_start {
            return Err(ApiError::BadRequest(
                "Invalid billing period for proration".to_string(),
            ));
        }

        let total_days = billing_period_end
            .signed_duration_since(billing_period_start)
            .num_days();
        let unused_days = billing_period_end
            .signed_duration_since(effective_date)
            .num_days();

        if unused_days < 0 || total_days <= 0 {
            return Err(ApiError::BadRequest(
                "Effective date must be within the current billing period".to_string(),
            ));
        }

        let original_amount = current_plan.base_amount;
        let prorated_refund = (original_amount * rust_decimal::Decimal::from(unused_days))
            / rust_decimal::Decimal::from(total_days);
        let prorated_charge = (new_plan.base_amount * rust_decimal::Decimal::from(unused_days))
            / rust_decimal::Decimal::from(total_days);

        let (direction, refund_or_charge) = if new_plan.base_amount > original_amount {
            (
                ProrationDirection::Charge,
                prorated_charge - prorated_refund,
            )
        } else {
            (
                ProrationDirection::Refund,
                prorated_refund - prorated_charge,
            )
        };

        Ok(ProrationResult {
            original_amount,
            prorated_amount: prorated_charge,
            unused_days,
            total_days,
            refund_or_charge: refund_or_charge.abs(),
            direction,
        })
    }

    // --- Dunning ---

    /// Get dunning entries for a subscription
    #[tracing::instrument(skip(self))]
    pub async fn get_dunning_status(
        &self,
        subscription_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DunningEntryResponse>, ApiError> {
        let entries = self
            .repo
            .find_dunning_by_subscription(subscription_id, tenant_id)
            .await?;
        Ok(entries.into_iter().map(|e| e.into()).collect())
    }

    /// Handle dunning retry for a subscription with failed payment
    /// Returns the updated dunning entry after retry attempt
    #[tracing::instrument(skip(self))]
    pub async fn handle_dunning(
        &self,
        subscription_id: i64,
        tenant_id: i64,
        invoice_id: i64,
    ) -> Result<DunningEntryResponse, ApiError> {
        let existing = self
            .repo
            .find_dunning_by_subscription(subscription_id, tenant_id)
            .await?;

        let active_entry = existing
            .into_iter()
            .filter(|d| d.invoice_id == invoice_id)
            .max_by_key(|d| d.attempt_number);

        let attempt_number = active_entry
            .as_ref()
            .map(|d| d.attempt_number + 1)
            .unwrap_or(1);

        if attempt_number > 3 {
            // Max retries reached - mark as failed and update subscription status
            if let Some(entry) = active_entry {
                let updated = self
                    .repo
                    .update_dunning_status(
                        entry.id,
                        tenant_id,
                        crate::domain::subscription::model::DunningStatus::Failed,
                        attempt_number,
                        None,
                    )
                    .await?;

                // Update subscription to past_due
                let update = UpdateSubscription {
                    status: Some(SubscriptionStatus::PastDue),
                    ..Default::default()
                };
                self.repo
                    .update_subscription(subscription_id, tenant_id, update)
                    .await?;

                return Ok(updated.into());
            }

            return Err(ApiError::BadRequest(
                "Maximum dunning retries exceeded".to_string(),
            ));
        }

        let retry_at = if attempt_number == 1 {
            Some(Utc::now() + chrono::Duration::days(1))
        } else if attempt_number == 2 {
            Some(Utc::now() + chrono::Duration::days(3))
        } else {
            Some(Utc::now() + chrono::Duration::days(7))
        };

        let entry = if let Some(prev) = active_entry {
            self.repo
                .update_dunning_status(
                    prev.id,
                    tenant_id,
                    crate::domain::subscription::model::DunningStatus::Active,
                    attempt_number,
                    retry_at,
                )
                .await?
        } else {
            let created = self
                .repo
                .create_dunning_entry(tenant_id, subscription_id, invoice_id, attempt_number)
                .await?;
            self.repo
                .update_dunning_status(
                    created.id,
                    tenant_id,
                    crate::domain::subscription::model::DunningStatus::Active,
                    attempt_number,
                    retry_at,
                )
                .await?
        };

        Ok(entry.into())
    }

    // --- Trial ---

    /// Convert a trial subscription to paid
    #[tracing::instrument(skip(self))]
    pub async fn process_trial_conversion(
        &self,
        subscription_id: i64,
        tenant_id: i64,
    ) -> Result<TrialConversionResult, ApiError> {
        let sub = self
            .repo
            .find_subscription_by_id(subscription_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Subscription {} not found", subscription_id))
            })?;

        if sub.status != SubscriptionStatus::Trial {
            return Err(ApiError::BadRequest(
                "Subscription is not in trial status".to_string(),
            ));
        }

        let plan = self
            .repo
            .find_plan_by_id(sub.plan_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Plan {} not found", sub.plan_id)))?;

        let today = Utc::now().date_naive();
        let next_billing = match plan.billing_cycle {
            crate::domain::subscription::model::BillingCycle::Monthly => today
                .checked_add_months(chrono::Months::new(1))
                .unwrap_or(today),
            crate::domain::subscription::model::BillingCycle::Quarterly => today
                .checked_add_months(chrono::Months::new(3))
                .unwrap_or(today),
            crate::domain::subscription::model::BillingCycle::Yearly => today
                .checked_add_months(chrono::Months::new(12))
                .unwrap_or(today),
        };

        let update = UpdateSubscription {
            status: Some(SubscriptionStatus::Active),
            next_billing_date: Some(next_billing),
            trial_end_date: Some(today),
            ..Default::default()
        };

        let updated = self
            .repo
            .update_subscription(subscription_id, tenant_id, update)
            .await?;

        Ok(TrialConversionResult {
            subscription_id: updated.id,
            previous_status: SubscriptionStatus::Trial,
            new_status: SubscriptionStatus::Active,
            billing_start_date: today,
            next_billing_date: next_billing,
        })
    }

    /// List trial subscriptions for a tenant
    #[tracing::instrument(skip(self))]
    pub async fn list_trial_subscriptions(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionResponse>, ApiError> {
        let subs = self.repo.find_trial_subscriptions(tenant_id).await?;
        Ok(subs.into_iter().map(|s| s.into()).collect())
    }

    // --- Usage ---

    /// Record usage for metered billing
    #[tracing::instrument(skip(self))]
    pub async fn record_usage(
        &self,
        subscription_id: i64,
        tenant_id: i64,
        request: RecordUsageRequest,
    ) -> Result<UsageRecordResponse, ApiError> {
        let sub = self
            .repo
            .find_subscription_by_id(subscription_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Subscription {} not found", subscription_id))
            })?;

        let plan = self
            .repo
            .find_plan_by_id(sub.plan_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Plan {} not found", sub.plan_id)))?;

        // Validate period
        if request.billing_period_end <= request.billing_period_start {
            return Err(ApiError::BadRequest(
                "Billing period end must be after start".to_string(),
            ));
        }

        let record = self
            .repo
            .create_usage_record(
                tenant_id,
                subscription_id,
                request.quantity,
                request.unit,
                request.billing_period_start,
                request.billing_period_end,
            )
            .await?;

        // If plan has metered billing, calculate and record overage if applicable
        if let (Some(included), Some(overage_rate)) = (plan.included_quantity, plan.overage_rate) {
            let existing = self
                .repo
                .find_usage_by_period(
                    subscription_id,
                    tenant_id,
                    request.billing_period_start,
                    request.billing_period_end,
                )
                .await?;

            let total_usage: i64 = existing.iter().map(|u| u.quantity).sum();

            if total_usage > included {
                let overage = total_usage - included;
                tracing::info!(
                    "Subscription {} exceeded included quantity by {} units at rate {}",
                    subscription_id,
                    overage,
                    overage_rate
                );
            }
        }

        Ok(record.into())
    }

    /// Get usage records for a subscription
    #[tracing::instrument(skip(self))]
    pub async fn get_usage_records(
        &self,
        subscription_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<UsageRecordResponse>, ApiError> {
        let records = self
            .repo
            .find_usage_by_subscription(subscription_id, tenant_id)
            .await?;
        Ok(records.into_iter().map(|r| r.into()).collect())
    }

    // --- Cancellation ---

    /// Cancel a subscription with optional refund calculation
    #[tracing::instrument(skip(self))]
    pub async fn cancel_subscription(
        &self,
        subscription_id: i64,
        tenant_id: i64,
        request: CancelSubscriptionRequest,
    ) -> Result<CancellationResult, ApiError> {
        let sub = self
            .repo
            .find_subscription_by_id(subscription_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Subscription {} not found", subscription_id))
            })?;

        if sub.status == SubscriptionStatus::Cancelled || sub.status == SubscriptionStatus::Expired
        {
            return Err(ApiError::BadRequest(
                "Subscription is already cancelled or expired".to_string(),
            ));
        }

        let plan = self
            .repo
            .find_plan_by_id(sub.plan_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Plan {} not found", sub.plan_id)))?;

        let today = Utc::now().date_naive();
        let mut refund_amount = rust_decimal::Decimal::ZERO;
        let mut unused_days = 0i64;

        if !request.cancel_immediately {
            // Cancel at end of period - no refund
            let update = UpdateSubscription {
                status: Some(SubscriptionStatus::Cancelled),
                end_date: sub.next_billing_date,
                auto_renew: Some(false),
                ..Default::default()
            };
            self.repo
                .update_subscription(subscription_id, tenant_id, update)
                .await?;
        } else {
            // Immediate cancellation with prorated refund
            let billing_period_start = sub
                .last_billed_at
                .map(|dt| dt.date_naive())
                .unwrap_or(sub.start_date);
            let billing_period_end = sub.next_billing_date.unwrap_or(billing_period_start);

            if billing_period_end > today {
                let total_days = billing_period_end
                    .signed_duration_since(billing_period_start)
                    .num_days();
                unused_days = billing_period_end.signed_duration_since(today).num_days();

                if total_days > 0 {
                    refund_amount = (plan.base_amount * rust_decimal::Decimal::from(unused_days))
                        / rust_decimal::Decimal::from(total_days);
                }
            }

            let update = UpdateSubscription {
                status: Some(SubscriptionStatus::Cancelled),
                end_date: Some(today),
                auto_renew: Some(false),
                ..Default::default()
            };
            self.repo
                .update_subscription(subscription_id, tenant_id, update)
                .await?;
        }

        Ok(CancellationResult {
            subscription_id,
            status: SubscriptionStatus::Cancelled,
            refund_amount,
            unused_days,
            cancelled_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cari::model::CreateCari;
    use crate::domain::cari::repository::InMemoryCariRepository;
    use crate::domain::subscription::model::{BillingCycle, SubscriptionStatus};
    use crate::domain::subscription::repository::InMemorySubscriptionRepository;
    use rust_decimal::Decimal;
    use std::sync::Arc;

    async fn create_service() -> SubscriptionService {
        let repo = Arc::new(InMemorySubscriptionRepository::new()) as BoxSubscriptionRepository;
        // Seed the parent cari (customer) entities the create_subscription
        // customer_id precheck validates against. InMemory auto-id starts at 1,
        // matching the `customer_id: 1` happy-path tests below; a tenant-2 cari
        // (auto-id 2) is the foreign referent for the cross-tenant IDOR
        // rejection test.
        let cari_repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
        cari_repo
            .create(CreateCari {
                code: "C1".to_string(),
                name: "Test Cari T1".to_string(),
                tenant_id: 1,
                created_by: 1,
                ..Default::default()
            })
            .await
            .expect("seed cari t1");
        cari_repo
            .create(CreateCari {
                code: "C2".to_string(),
                name: "Test Cari T2".to_string(),
                tenant_id: 2,
                created_by: 1,
                ..Default::default()
            })
            .await
            .expect("seed cari t2");
        SubscriptionService::new(repo, cari_repo)
    }

    #[tokio::test]
    async fn test_create_plan() {
        let service = create_service().await;

        let create = CreatePlan {
            name: "Pro".to_string(),
            description: Some("Pro plan".to_string()),
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(10000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
            included_quantity: None,
            overage_rate: None,
            tenant_id: 1,
        };

        let result = service.create_plan(create).await;
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.name, "Pro");
        assert_eq!(plan.currency, "TRY");
    }

    #[tokio::test]
    async fn test_create_subscription() {
        let service = create_service().await;

        // Create plan first
        let plan_create = CreatePlan {
            name: "Basic".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(5000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
            included_quantity: None,
            overage_rate: None,
            tenant_id: 1,
        };
        let plan = service.create_plan(plan_create).await.unwrap();

        let sub_create = CreateSubscription {
            customer_id: 1,
            plan_id: plan.id,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            status: SubscriptionStatus::Active,
            auto_renew: true,
            next_billing_date: Some(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()),
            trial_start_date: None,
            trial_end_date: None,
            tenant_id: 1,
        };

        let result = service.create_subscription(sub_create).await;
        assert!(result.is_ok());
        let sub = result.unwrap();
        assert_eq!(sub.customer_id, 1);
        assert_eq!(sub.plan_id, plan.id);
    }

    #[tokio::test]
    async fn test_renew_subscription() {
        let service = create_service().await;

        let plan_create = CreatePlan {
            name: "Basic".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(5000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
            included_quantity: None,
            overage_rate: None,
            tenant_id: 1,
        };
        let plan = service.create_plan(plan_create).await.unwrap();

        let sub_create = CreateSubscription {
            customer_id: 1,
            plan_id: plan.id,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: Some(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()),
            status: SubscriptionStatus::Active,
            auto_renew: true,
            next_billing_date: Some(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()),
            trial_start_date: None,
            trial_end_date: None,
            tenant_id: 1,
        };
        let sub = service.create_subscription(sub_create).await.unwrap();

        let result = service.renew_subscription(sub.id, 1).await;
        assert!(result.is_ok());
        let renewed = result.unwrap();
        assert!(renewed.last_billed_at.is_some());
    }

    #[tokio::test]
    async fn test_due_for_billing() {
        let service = create_service().await;

        let plan_create = CreatePlan {
            name: "Basic".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(5000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
            included_quantity: None,
            overage_rate: None,
            tenant_id: 1,
        };
        let plan = service.create_plan(plan_create).await.unwrap();

        let sub_create = CreateSubscription {
            customer_id: 1,
            plan_id: plan.id,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            status: SubscriptionStatus::Active,
            auto_renew: true,
            next_billing_date: Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()),
            trial_start_date: None,
            trial_end_date: None,
            tenant_id: 1,
        };
        service.create_subscription(sub_create).await.unwrap();

        let due = service
            .due_for_billing(1, NaiveDate::from_ymd_opt(2024, 1, 20).unwrap())
            .await
            .unwrap();
        assert_eq!(due.len(), 1);
    }

    #[tokio::test]
    async fn test_calculate_proration_upgrade() {
        let service = create_service().await;

        let basic_plan = CreatePlan {
            name: "Basic".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(5000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
            included_quantity: None,
            overage_rate: None,
            tenant_id: 1,
        };
        let basic = service.create_plan(basic_plan).await.unwrap();

        let pro_plan = CreatePlan {
            name: "Pro".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(10000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
            included_quantity: None,
            overage_rate: None,
            tenant_id: 1,
        };
        let pro = service.create_plan(pro_plan).await.unwrap();

        let sub_create = CreateSubscription {
            customer_id: 1,
            plan_id: basic.id,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            status: SubscriptionStatus::Active,
            auto_renew: true,
            next_billing_date: Some(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()),
            trial_start_date: None,
            trial_end_date: None,
            tenant_id: 1,
        };
        let sub = service.create_subscription(sub_create).await.unwrap();

        let result = service
            .calculate_proration(
                sub.id,
                1,
                pro.id,
                NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            )
            .await;
        assert!(result.is_ok());
        let proration = result.unwrap();
        assert_eq!(proration.original_amount, Decimal::new(5000, 2));
        assert_eq!(proration.direction, ProrationDirection::Charge);
        assert!(proration.refund_or_charge > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_process_trial_conversion() {
        let service = create_service().await;

        let plan_create = CreatePlan {
            name: "Pro".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(10000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
            included_quantity: None,
            overage_rate: None,
            tenant_id: 1,
        };
        let plan = service.create_plan(plan_create).await.unwrap();

        let sub_create = CreateSubscription {
            customer_id: 1,
            plan_id: plan.id,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            status: SubscriptionStatus::Trial,
            auto_renew: true,
            next_billing_date: None,
            trial_start_date: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            trial_end_date: Some(NaiveDate::from_ymd_opt(2024, 1, 14).unwrap()),
            tenant_id: 1,
        };
        let sub = service.create_subscription(sub_create).await.unwrap();

        let result = service.process_trial_conversion(sub.id, 1).await;
        assert!(result.is_ok());
        let conversion = result.unwrap();
        assert_eq!(conversion.previous_status, SubscriptionStatus::Trial);
        assert_eq!(conversion.new_status, SubscriptionStatus::Active);
    }

    #[tokio::test]
    async fn test_cancel_subscription_immediate() {
        let service = create_service().await;

        let plan_create = CreatePlan {
            name: "Basic".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(5000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
            included_quantity: None,
            overage_rate: None,
            tenant_id: 1,
        };
        let plan = service.create_plan(plan_create).await.unwrap();

        let sub_create = CreateSubscription {
            customer_id: 1,
            plan_id: plan.id,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            status: SubscriptionStatus::Active,
            auto_renew: true,
            next_billing_date: Some(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()),
            trial_start_date: None,
            trial_end_date: None,
            tenant_id: 1,
        };
        let sub = service.create_subscription(sub_create).await.unwrap();

        let request = CancelSubscriptionRequest {
            cancel_immediately: true,
            reason: Some("User request".to_string()),
        };
        let result = service.cancel_subscription(sub.id, 1, request).await;
        assert!(result.is_ok());
        let cancellation = result.unwrap();
        assert_eq!(cancellation.status, SubscriptionStatus::Cancelled);
        assert!(cancellation.refund_amount >= Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_record_usage() {
        let service = create_service().await;

        let plan_create = CreatePlan {
            name: "Metered".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(5000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
            included_quantity: Some(100),
            overage_rate: Some(Decimal::new(50, 2)),
            tenant_id: 1,
        };
        let plan = service.create_plan(plan_create).await.unwrap();

        let sub_create = CreateSubscription {
            customer_id: 1,
            plan_id: plan.id,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            status: SubscriptionStatus::Active,
            auto_renew: true,
            next_billing_date: Some(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()),
            trial_start_date: None,
            trial_end_date: None,
            tenant_id: 1,
        };
        let sub = service.create_subscription(sub_create).await.unwrap();

        let request = RecordUsageRequest {
            quantity: 150,
            unit: "api_calls".to_string(),
            billing_period_start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            billing_period_end: NaiveDate::from_ymd_opt(2024, 1, 31).unwrap(),
        };
        let result = service.record_usage(sub.id, 1, request).await;
        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.quantity, 150);
        assert_eq!(record.unit, "api_calls");
    }

    #[tokio::test]
    async fn test_handle_dunning() {
        let service = create_service().await;

        let plan_create = CreatePlan {
            name: "Basic".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(5000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
            included_quantity: None,
            overage_rate: None,
            tenant_id: 1,
        };
        let plan = service.create_plan(plan_create).await.unwrap();

        let sub_create = CreateSubscription {
            customer_id: 1,
            plan_id: plan.id,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            status: SubscriptionStatus::Active,
            auto_renew: true,
            next_billing_date: Some(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()),
            trial_start_date: None,
            trial_end_date: None,
            tenant_id: 1,
        };
        let sub = service.create_subscription(sub_create).await.unwrap();

        // Create a pending invoice to simulate dunning
        let invoice = service
            .repo
            .create_subscription_invoice(
                sub.id,
                1,
                None,
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
                plan.base_amount,
                crate::domain::subscription::model::SubscriptionInvoiceStatus::Failed,
            )
            .await
            .unwrap();

        let result = service.handle_dunning(sub.id, 1, invoice.id).await;
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.attempt_number, 1);
        assert!(entry.retry_at.is_some());

        // Second retry
        let result = service.handle_dunning(sub.id, 1, invoice.id).await;
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.attempt_number, 2);

        // Third retry
        let result = service.handle_dunning(sub.id, 1, invoice.id).await;
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.attempt_number, 3);

        // Fourth retry should fail
        let result = service.handle_dunning(sub.id, 1, invoice.id).await;
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(
            entry.status,
            crate::domain::subscription::model::DunningStatus::Failed
        );
    }

    fn base_plan(tenant: i64, name: &str) -> CreatePlan {
        CreatePlan {
            name: name.to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(5000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
            included_quantity: None,
            overage_rate: None,
            tenant_id: tenant,
        }
    }

    fn base_sub(plan_id: i64, tenant: i64) -> CreateSubscription {
        CreateSubscription {
            customer_id: 1,
            plan_id,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            status: SubscriptionStatus::Active,
            auto_renew: true,
            next_billing_date: Some(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()),
            trial_start_date: None,
            trial_end_date: None,
            tenant_id: tenant,
        }
    }

    /// Like `base_sub` but with an explicit `customer_id` (for the
    /// customer_id precheck tests that vary the cari referent).
    fn base_sub_customer(plan_id: i64, customer_id: i64, tenant: i64) -> CreateSubscription {
        CreateSubscription {
            customer_id,
            plan_id,
            start_date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            end_date: None,
            status: SubscriptionStatus::Active,
            auto_renew: true,
            next_billing_date: Some(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()),
            trial_start_date: None,
            trial_end_date: None,
            tenant_id: tenant,
        }
    }

    /// Rejects a subscription stamped onto a foreign-tenant plan.
    #[tokio::test]
    async fn test_create_subscription_rejects_foreign_plan() {
        let service = create_service().await;
        // Seed a tenant-1 plan (auto-id 1) and a tenant-2 plan (auto-id 2).
        // customer_id=1 is a valid own-tenant cari (seeded in create_service),
        // so it isolates the plan_id precheck.
        let owned_plan = service.create_plan(base_plan(1, "T1")).await.unwrap().id;
        let foreign_plan = service.create_plan(base_plan(2, "T2")).await.unwrap().id;
        assert_ne!(owned_plan, foreign_plan);

        // Same-tenant plan → ok.
        assert!(
            service
                .create_subscription(base_sub(owned_plan, 1))
                .await
                .is_ok(),
            "same-tenant plan must succeed"
        );

        // Foreign plan → NotFound.
        let result = service.create_subscription(base_sub(foreign_plan, 1)).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "tenant-1 must NOT subscribe to a tenant-2 plan, got {:?}",
            result
        );

        // Nonexistent plan → NotFound.
        let result = service.create_subscription(base_sub(999_999, 1)).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "nonexistent plan must be NotFound, got {:?}",
            result
        );
    }

    /// Rejects a subscription stamped onto a foreign-tenant customer (cari).
    /// The plan is a valid own-tenant plan, so the NotFound is uniquely from
    /// the `customer_id` parent-ownership precheck.
    #[tokio::test]
    async fn test_create_subscription_rejects_foreign_customer() {
        let service = create_service().await;
        // Valid own-tenant plan (auto-id 1).
        let owned_plan = service.create_plan(base_plan(1, "T1")).await.unwrap().id;

        // Same-tenant customer (id=1) → ok.
        assert!(
            service
                .create_subscription(base_sub_customer(owned_plan, 1, 1))
                .await
                .is_ok(),
            "same-tenant customer must succeed"
        );

        // Foreign customer (id=2, belongs to tenant 2) → NotFound.
        let result = service
            .create_subscription(base_sub_customer(owned_plan, 2, 1))
            .await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "tenant-1 must NOT create a subscription for a tenant-2 customer, got {:?}",
            result
        );

        // Nonexistent customer → NotFound.
        let result = service
            .create_subscription(base_sub_customer(owned_plan, 999_999, 1))
            .await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "nonexistent customer must be NotFound, got {:?}",
            result
        );
    }
}
