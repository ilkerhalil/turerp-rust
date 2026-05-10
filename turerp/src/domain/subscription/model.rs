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
    PastDue,
}

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscriptionStatus::Active => write!(f, "active"),
            SubscriptionStatus::Cancelled => write!(f, "cancelled"),
            SubscriptionStatus::Expired => write!(f, "expired"),
            SubscriptionStatus::Trial => write!(f, "trial"),
            SubscriptionStatus::PastDue => write!(f, "past_due"),
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
            "past_due" => Ok(SubscriptionStatus::PastDue),
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

/// Dunning status for payment retry workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum DunningStatus {
    #[default]
    Active,
    Resolved,
    Failed,
}

impl std::fmt::Display for DunningStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DunningStatus::Active => write!(f, "active"),
            DunningStatus::Resolved => write!(f, "resolved"),
            DunningStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for DunningStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(DunningStatus::Active),
            "resolved" => Ok(DunningStatus::Resolved),
            "failed" => Ok(DunningStatus::Failed),
            _ => Err(format!("Invalid dunning status: {}", s)),
        }
    }
}

/// Usage record type for metered billing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum UsageRecordType {
    #[default]
    Metered,
    Overage,
}

impl std::fmt::Display for UsageRecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UsageRecordType::Metered => write!(f, "metered"),
            UsageRecordType::Overage => write!(f, "overage"),
        }
    }
}

impl std::str::FromStr for UsageRecordType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "metered" => Ok(UsageRecordType::Metered),
            "overage" => Ok(UsageRecordType::Overage),
            _ => Err(format!("Invalid usage record type: {}", s)),
        }
    }
}

/// Dunning entry for failed payment retry tracking
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DunningEntry {
    pub id: i64,
    pub tenant_id: i64,
    pub subscription_id: i64,
    pub invoice_id: i64,
    pub attempt_number: i32,
    pub status: DunningStatus,
    pub retry_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Usage record for metered billing
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UsageRecord {
    pub id: i64,
    pub tenant_id: i64,
    pub subscription_id: i64,
    pub record_type: UsageRecordType,
    pub quantity: i64,
    pub unit: String,
    pub recorded_at: DateTime<Utc>,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
}

/// Result of a proration calculation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProrationResult {
    pub original_amount: Decimal,
    pub prorated_amount: Decimal,
    pub unused_days: i64,
    pub total_days: i64,
    pub refund_or_charge: Decimal,
    pub direction: ProrationDirection,
}

/// Direction of proration charge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProrationDirection {
    Refund,
    Charge,
}

/// Result of subscription cancellation with refund logic
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CancellationResult {
    pub subscription_id: i64,
    pub status: SubscriptionStatus,
    pub refund_amount: Decimal,
    pub unused_days: i64,
    pub cancelled_at: DateTime<Utc>,
}

/// Result of trial to paid conversion
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrialConversionResult {
    pub subscription_id: i64,
    pub previous_status: SubscriptionStatus,
    pub new_status: SubscriptionStatus,
    pub billing_start_date: NaiveDate,
    pub next_billing_date: NaiveDate,
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
    pub included_quantity: Option<i64>,
    pub overage_rate: Option<Decimal>,
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

    pub included_quantity: Option<i64>,

    pub overage_rate: Option<Decimal>,

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

    pub included_quantity: Option<i64>,

    pub overage_rate: Option<Decimal>,
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
    pub trial_start_date: Option<NaiveDate>,
    pub trial_end_date: Option<NaiveDate>,
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

    pub trial_start_date: Option<NaiveDate>,

    pub trial_end_date: Option<NaiveDate>,

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

    pub trial_start_date: Option<NaiveDate>,

    pub trial_end_date: Option<NaiveDate>,
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
    pub included_quantity: Option<i64>,
    pub overage_rate: Option<Decimal>,
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
            included_quantity: plan.included_quantity,
            overage_rate: plan.overage_rate,
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
    pub trial_start_date: Option<NaiveDate>,
    pub trial_end_date: Option<NaiveDate>,
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
            trial_start_date: sub.trial_start_date,
            trial_end_date: sub.trial_end_date,
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

/// Dunning entry response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DunningEntryResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub subscription_id: i64,
    pub invoice_id: i64,
    pub attempt_number: i32,
    pub status: DunningStatus,
    pub retry_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl From<DunningEntry> for DunningEntryResponse {
    fn from(entry: DunningEntry) -> Self {
        Self {
            id: entry.id,
            tenant_id: entry.tenant_id,
            subscription_id: entry.subscription_id,
            invoice_id: entry.invoice_id,
            attempt_number: entry.attempt_number,
            status: entry.status,
            retry_at: entry.retry_at,
            created_at: entry.created_at,
            resolved_at: entry.resolved_at,
        }
    }
}

/// Usage record response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UsageRecordResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub subscription_id: i64,
    pub record_type: UsageRecordType,
    pub quantity: i64,
    pub unit: String,
    pub recorded_at: DateTime<Utc>,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
}

impl From<UsageRecord> for UsageRecordResponse {
    fn from(record: UsageRecord) -> Self {
        Self {
            id: record.id,
            tenant_id: record.tenant_id,
            subscription_id: record.subscription_id,
            record_type: record.record_type,
            quantity: record.quantity,
            unit: record.unit,
            recorded_at: record.recorded_at,
            billing_period_start: record.billing_period_start,
            billing_period_end: record.billing_period_end,
        }
    }
}

/// Request to record usage for metered billing
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct RecordUsageRequest {
    pub quantity: i64,
    pub unit: String,
    pub billing_period_start: NaiveDate,
    pub billing_period_end: NaiveDate,
}

/// Request to calculate proration for plan change
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct CalculateProrationRequest {
    pub new_plan_id: i64,
    pub effective_date: NaiveDate,
}

/// Request to cancel a subscription
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct CancelSubscriptionRequest {
    pub cancel_immediately: bool,
    pub reason: Option<String>,
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
