//! Middleware layer

pub mod api_key;
pub mod audit;
pub mod auth;
pub mod idempotency;
pub mod ip_whitelist;
pub mod metrics;
pub mod rate_limit;
pub mod request_id;
pub mod security_headers;
pub mod tenant;
pub mod tracing;

pub use api_key::{ApiKeyAuth, ApiKeyClaims};
pub use audit::AuditLoggingMiddleware;
pub use auth::{get_auth_claims, AdminUser, AuthUser, JwtAuthMiddleware, PortalUser, PUBLIC_PATHS};
pub use idempotency::{
    IdempotencyMiddleware, IdempotencyStore, InMemoryIdempotencyStore, RedisIdempotencyStore,
};
pub use ip_whitelist::IpWhitelistMiddleware;
pub use metrics::{install_metrics_exporter, render_metrics, MetricsMiddleware};
pub use rate_limit::RateLimitMiddleware;
pub use request_id::RequestIdMiddleware;
pub use security_headers::SecurityHeadersMiddleware;
pub use tenant::{TenantContext, TenantContextExt, TenantMiddleware};
pub use tracing::TracingMiddleware;
