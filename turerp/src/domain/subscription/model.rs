//! Subscription domain models

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use validator::Validate;

/// Billing cycle for subscription plans
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum BillingCycle {
    #[default]
    Monthly,
    Quarterly,
    Yearly,
}

impl std::fmt::Display for BillingCycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BillingCycle::Monthly => write!(f, "monthly"),
            BillingCycle::Quarterly => write!(f, "quarterly"),
            BillingCycle::Yearly => write!(f, "yearly"),
        }
    }
}

impl std::str::FromStr for BillingCycle {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "monthly" => Ok(BillingCycle::Monthly),
            "quarterly" => Ok(BillingCycle::Quarterly),
            "yearly" => Ok(BillingCycle::Yearly),
            _ => Err(format!("Invalid billing cycle: {}", s)),
        }
    }
}

/// Subscription status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    #[default]
    Active,
    Cancelled,
    Expired,
    Trial,
}

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionStatus::Active => write!(f, "active"),
            SubscriptionStatus::Cancelled => write!(f, "cancelled"),
            SubscriptionStatus::Expired => write!(f, "expired"),
            SubscriptionStatus::Trial => write!(f, "trial"),
        }
    }
}

impl std::str::FromStr for SubscriptionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(SubscriptionStatus::Active),
            "cancelled" => Ok(SubscriptionStatus::Cancelled),
            "expired" => Ok(SubscriptionStatus::Expired),
            "trial" => Ok(SubscriptionStatus::Trial),
            _ => Err(format!("Invalid subscription status: {}", s)),
        }
    }
}

/// Subscription invoice status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionInvoiceStatus {
    #[default]
    Pending,
    Paid,
    Failed,
}

impl std::fmt::Display for SubscriptionInvoiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionInvoiceStatus::Pending => write!(f, "pending"),
            SubscriptionInvoiceStatus::Paid => write!(f, "paid"),
            SubscriptionInvoiceStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for SubscriptionInvoiceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(SubscriptionInvoiceStatus::Pending),
            "paid" => Ok(SubscriptionInvoiceStatus::Paid),
            "failed" => Ok(SubscriptionInvoiceStatus::Failed),
            _ => Err(format!("Invalid subscription invoice status: {}", s)),
        }
    }
}

/// Subscription plan entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionPlan {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub billing_cycle: BillingCycle,
    pub base_amount: Decimal,
    pub currency: String,
    pub features: Option<Value>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for SubscriptionPlan {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

/// Data for creating a subscription plan
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreatePlan {
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    #[validate(length(max = 1000))]
    pub description: Option<String>,

    pub billing_cycle: BillingCycle,

    pub base_amount: Decimal,

    #[validate(length(min = 3, max = 3))]
    #[serde(default = "default_currency")]
    pub currency: String,

    pub features: Option<Value>,

    #[serde(default = "default_true")]
    pub is_active: bool,

    pub tenant_id: i64,
}

/// Data for updating a subscription plan
#[derive(Debug, Clone, Deserialize, Serialize, Default, Validate, ToSchema)]
pub struct UpdatePlan {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,

    #[validate(length(max = 1000))]
    pub description: Option<String>,

    pub billing_cycle: Option<BillingCycle>,

    pub base_amount: Option<Decimal>,

    #[validate(length(min = 3, max = 3))]
    pub currency: Option<String>,

    pub features: Option<Value>,

    pub is_active: Option<bool>,
}

/// Subscription entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Subscription {
    pub id: i64,
    pub tenant_id: i64,
    pub customer_id: i64,
    pub plan_id: i64,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub status: SubscriptionStatus,
    pub auto_renew: bool,
    pub last_billed_at: Option<DateTime<Utc>>,
    pub next_billing_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for Subscription {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

/// Data for creating a subscription
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateSubscription {
    pub customer_id: i64,

    pub plan_id: i64,

    pub start_date: NaiveDate,

    pub end_date: Option<NaiveDate>,

    #[serde(default)]
    pub status: SubscriptionStatus,

    #[serde(default = "default_true")]
    pub auto_renew: bool,

    pub next_billing_date: Option<NaiveDate>,

    pub tenant_id: i64,
}

/// Data for updating a subscription
#[derive(Debug, Clone, Deserialize, Serialize, Default, Validate, ToSchema)]
pub struct UpdateSubscription {
    pub plan_id: Option<i64>,

    pub start_date: Option<NaiveDate>,

    pub end_date: Option<NaiveDate>,

    pub status: Option<SubscriptionStatus>,

    pub auto_renew: Option<bool>,

    pub next_billing_date: Option<NaiveDate>,
}

/// Subscription invoice entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionInvoice {
    pub id: i64,
    pub tenant_id: i64,
    pub subscription_id: i64,
    pub invoice_id: Option<i64>,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
    pub amount: Decimal,
    pub status: SubscriptionInvoiceStatus,
    pub created_at: DateTime<Utc>,
}

/// Subscription plan response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionPlanResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub billing_cycle: BillingCycle,
    pub base_amount: Decimal,
    pub currency: String,
    pub features: Option<Value>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<SubscriptionPlan> for SubscriptionPlanResponse {
    fn from(plan: SubscriptionPlan) -> Self {
        Self {
            id: plan.id,
            tenant_id: plan.tenant_id,
            name: plan.name,
            description: plan.description,
            billing_cycle: plan.billing_cycle,
            base_amount: plan.base_amount,
            currency: plan.currency,
            features: plan.features,
            is_active: plan.is_active,
            created_at: plan.created_at,
            updated_at: plan.updated_at,
        }
    }
}

/// Subscription response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub customer_id: i64,
    pub plan_id: i64,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub status: SubscriptionStatus,
    pub auto_renew: bool,
    pub last_billed_at: Option<DateTime<Utc>>,
    pub next_billing_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Subscription> for SubscriptionResponse {
    fn from(sub: Subscription) -> Self {
        Self {
            id: sub.id,
            tenant_id: sub.tenant_id,
            customer_id: sub.customer_id,
            plan_id: sub.plan_id,
            start_date: sub.start_date,
            end_date: sub.end_date,
            status: sub.status,
            auto_renew: sub.auto_renew,
            last_billed_at: sub.last_billed_at,
            next_billing_date: sub.next_billing_date,
            created_at: sub.created_at,
            updated_at: sub.updated_at,
        }
    }
}

/// Subscription invoice response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubscriptionInvoiceResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub subscription_id: i64,
    pub invoice_id: Option<i64>,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
    pub amount: Decimal,
    pub status: SubscriptionInvoiceStatus,
    pub created_at: DateTime<Utc>,
}

impl From<SubscriptionInvoice> for SubscriptionInvoiceResponse {
    fn from(inv: SubscriptionInvoice) -> Self {
        Self {
            id: inv.id,
            tenant_id: inv.tenant_id,
            subscription_id: inv.subscription_id,
            invoice_id: inv.invoice_id,
            billing_period_start: inv.billing_period_start,
            billing_period_end: inv.billing_period_end,
            amount: inv.amount,
            status: inv.status,
            created_at: inv.created_at,
        }
    }
}

fn default_currency() -> String {
    "TRY".to_string()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_billing_cycle_display() {
        assert_eq!(BillingCycle::Monthly.to_string(), "monthly");
        assert_eq!(BillingCycle::Quarterly.to_string(), "quarterly");
        assert_eq!(BillingCycle::Yearly.to_string(), "yearly");
    }

    #[test]
    fn test_billing_cycle_from_str() {
        assert_eq!(
            BillingCycle::from_str("monthly").unwrap(),
            BillingCycle::Monthly
        );
        assert_eq!(
            BillingCycle::from_str("QUARTERLY").unwrap(),
            BillingCycle::Quarterly
        );
        assert!(BillingCycle::from_str("invalid").is_err());
    }

    #[test]
    fn test_subscription_status_default() {
        let status: SubscriptionStatus = Default::default();
        assert_eq!(status, SubscriptionStatus::Active);
    }

    #[test]
    fn test_subscription_invoice_status_from_str() {
        assert_eq!(
            SubscriptionInvoiceStatus::from_str("paid").unwrap(),
            SubscriptionInvoiceStatus::Paid
        );
        assert!(SubscriptionInvoiceStatus::from_str("unknown").is_err());
    }
}
