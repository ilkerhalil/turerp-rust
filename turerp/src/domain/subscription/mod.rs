//! Subscription / SaaS Billing module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    BillingCycle, CreatePlan, CreateSubscription, Subscription, SubscriptionInvoice,
    SubscriptionInvoiceResponse, SubscriptionInvoiceStatus, SubscriptionPlan,
    SubscriptionPlanResponse, SubscriptionResponse, SubscriptionStatus, UpdatePlan,
    UpdateSubscription,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresSubscriptionRepository;
pub use repository::{
    BoxSubscriptionRepository, InMemorySubscriptionRepository, SubscriptionRepository,
};
pub use service::SubscriptionService;
