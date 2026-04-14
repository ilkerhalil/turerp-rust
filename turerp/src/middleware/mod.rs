//! Middleware layer

pub mod audit;
pub mod auth;
pub mod metrics;
pub mod rate_limit;
pub mod request_id;
pub mod tenant;

pub use audit::AuditLoggingMiddleware;
pub use auth::{get_auth_claims, AdminUser, AuthUser, JwtAuthMiddleware, PUBLIC_PATHS};
pub use metrics::{install_metrics_exporter, render_metrics, MetricsMiddleware};
pub use rate_limit::RateLimitMiddleware;
pub use request_id::RequestIdMiddleware;
pub use tenant::{TenantContext, TenantContextExt, TenantMiddleware};
