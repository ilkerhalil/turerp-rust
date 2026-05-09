//! Subscription service for business logic

use chrono::{NaiveDate, Utc};
use validator::Validate;

use crate::domain::subscription::model::{
    CreatePlan, CreateSubscription, SubscriptionInvoiceResponse, SubscriptionPlanResponse,
    SubscriptionResponse, UpdatePlan, UpdateSubscription,
};
use crate::domain::subscription::repository::BoxSubscriptionRepository;
use crate::error::ApiError;

/// Subscription service
#[derive(Clone)]
pub struct SubscriptionService {
    repo: BoxSubscriptionRepository,
}

impl SubscriptionService {
    /// Create a new subscription service
    pub fn new(repo: BoxSubscriptionRepository) -> Self {
        Self { repo }
    }

    // --- Plans ---

    /// Create a subscription plan
    pub async fn create_plan(
        &self,
        create: CreatePlan,
    ) -> Result<SubscriptionPlanResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;
        let plan = self.repo.create_plan(create).await?;
        Ok(plan.into())
    }

    /// Get a plan by ID
    pub async fn get_plan(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<SubscriptionPlanResponse, ApiError> {
        let plan = self
            .repo
            .find_plan_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Plan {} not found", id)))?;
        Ok(plan.into())
    }

    /// List all plans for a tenant
    pub async fn list_plans(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionPlanResponse>, ApiError> {
        let plans = self.repo.find_plans_by_tenant(tenant_id).await?;
        Ok(plans.into_iter().map(|p| p.into()).collect())
    }

    /// Update a plan
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
        Ok(plan.into())
    }

    /// Delete a plan
    pub async fn delete_plan(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.delete_plan(id, tenant_id).await
    }

    // --- Subscriptions ---

    /// Create a subscription
    pub async fn create_subscription(
        &self,
        create: CreateSubscription,
    ) -> Result<SubscriptionResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;
        let sub = self.repo.create_subscription(create).await?;
        Ok(sub.into())
    }

    /// Get a subscription by ID
    pub async fn get_subscription(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<SubscriptionResponse, ApiError> {
        let sub = self
            .repo
            .find_subscription_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Subscription {} not found", id)))?;
        Ok(sub.into())
    }

    /// List all subscriptions for a tenant
    pub async fn list_subscriptions(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<SubscriptionResponse>, ApiError> {
        let subs = self.repo.find_subscriptions_by_tenant(tenant_id).await?;
        Ok(subs.into_iter().map(|s| s.into()).collect())
    }

    /// List active subscriptions for a customer
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
        Ok(sub.into())
    }

    /// Delete a subscription
    pub async fn delete_subscription(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.delete_subscription(id, tenant_id).await
    }

    // --- Billing ---

    /// Renew a subscription (extend by billing cycle)
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
            .unwrap_or_else(|| Utc::now().date_naive());
        let billing_period_end = sub.next_billing_date.unwrap_or(billing_period_start);

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

        Ok(sub.into())
    }

    /// Find subscriptions due for billing on or before a date
    pub async fn due_for_billing(
        &self,
        tenant_id: i64,
        date: NaiveDate,
    ) -> Result<Vec<SubscriptionResponse>, ApiError> {
        let subs = self.repo.find_due_for_billing(tenant_id, date).await?;
        Ok(subs.into_iter().map(|s| s.into()).collect())
    }

    /// Get invoices for a subscription
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::subscription::model::{BillingCycle, SubscriptionStatus};
    use crate::domain::subscription::repository::InMemorySubscriptionRepository;
    use rust_decimal::Decimal;
    use std::sync::Arc;

    fn create_service() -> SubscriptionService {
        let repo = Arc::new(InMemorySubscriptionRepository::new()) as BoxSubscriptionRepository;
        SubscriptionService::new(repo)
    }

    #[tokio::test]
    async fn test_create_plan() {
        let service = create_service();

        let create = CreatePlan {
            name: "Pro".to_string(),
            description: Some("Pro plan".to_string()),
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(10000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
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
        let service = create_service();

        // Create plan first
        let plan_create = CreatePlan {
            name: "Basic".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(5000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
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
        let service = create_service();

        let plan_create = CreatePlan {
            name: "Basic".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(5000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
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
        let service = create_service();

        let plan_create = CreatePlan {
            name: "Basic".to_string(),
            description: None,
            billing_cycle: BillingCycle::Monthly,
            base_amount: Decimal::new(5000, 2),
            currency: "TRY".to_string(),
            features: None,
            is_active: true,
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
            tenant_id: 1,
        };
        service.create_subscription(sub_create).await.unwrap();

        let due = service
            .due_for_billing(1, NaiveDate::from_ymd_opt(2024, 1, 20).unwrap())
            .await
            .unwrap();
        assert_eq!(due.len(), 1);
    }
}
