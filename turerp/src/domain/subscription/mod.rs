//! Subscription / SaaS Billing module

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    BillingCycle, CalculateProrationRequest, CancelSubscriptionRequest, CancellationResult,
    CreatePlan, CreateSubscription, DunningEntry, DunningEntryResponse, DunningStatus,
    ProrationDirection, ProrationResult, RecordUsageRequest, Subscription, SubscriptionInvoice,
    SubscriptionInvoiceResponse, SubscriptionInvoiceStatus, SubscriptionPlan,
    SubscriptionPlanResponse, SubscriptionResponse, SubscriptionStatus, TrialConversionResult,
    UpdatePlan, UpdateSubscription, UsageRecord, UsageRecordResponse, UsageRecordType,
};
pub use postgres_repository::PostgresSubscriptionRepository;
pub use repository::{
    BoxSubscriptionRepository, InMemorySubscriptionRepository, SubscriptionRepository,
};
pub use service::SubscriptionService;
