//! API layer

pub mod v1;

// Legacy modules (deprecated, will be removed in v2)
pub mod auth;
pub mod users;

// Explicit re-exports to avoid ambiguity
pub use auth::configure as auth_configure;
pub use users::configure as users_configure;

// V1 re-exports
pub use v1::accounting_configure as v1_accounting_configure;
pub use v1::assets_configure as v1_assets_configure;
pub use v1::audit_configure as v1_audit_configure;
pub use v1::auth_configure as v1_auth_configure;
pub use v1::cari_configure as v1_cari_configure;
pub use v1::crm_configure as v1_crm_configure;
pub use v1::feature_flags_configure as v1_feature_flags_configure;
pub use v1::hr_configure as v1_hr_configure;
pub use v1::invoice_configure as v1_invoice_configure;
pub use v1::manufacturing_configure as v1_manufacturing_configure;
pub use v1::product_variants_configure as v1_product_variants_configure;
pub use v1::project_configure as v1_project_configure;
pub use v1::purchase_requests_configure as v1_purchase_requests_configure;
pub use v1::sales_configure as v1_sales_configure;
pub use v1::stock_configure as v1_stock_configure;
pub use v1::tenant_configure as v1_tenant_configure;
pub use v1::users_configure as v1_users_configure;

use crate::domain::auth::{LoginRequest, LoginResponse, RefreshTokenRequest, RegisterRequest};
use crate::domain::feature::{
    CreateFeatureFlag, FeatureFlagResponse, FeatureFlagStatus, UpdateFeatureFlag,
};
use crate::domain::user::{CreateUser, UpdateUser, UserResponse};
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::Modify;
use utoipa::OpenApi;

/// OpenAPI specification for the API (legacy - uses v1 internally)
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Turerp ERP API",
        description = "Multi-tenant SaaS ERP system API\n\n## Authentication\n\nAll endpoints except `/api/v1/auth/login`, `/api/v1/auth/register`, and `/api/v1/auth/refresh` require JWT Bearer token authentication.\n\n## Rate Limiting\n\nAuthentication endpoints are rate limited to 10 requests per minute per IP address with a burst of 3 requests.\n\n## API Versioning\n\n- `/api/v1/` - Current stable API (recommended)\n- `/api/auth/`, `/api/users/` - Legacy routes (deprecated, will be removed in v2)",
        version = "1.0.0",
        contact(
            name = "Turerp Team",
            email = "info@turerp.com"
        )
    ),
    paths(
        // V1 paths (primary)
        crate::api::v1::auth::register,
        crate::api::v1::auth::login,
        crate::api::v1::auth::refresh_token,
        crate::api::v1::auth::me,
        crate::api::v1::users::create_user,
        crate::api::v1::users::get_user,
        crate::api::v1::users::get_users,
        crate::api::v1::users::update_user,
        crate::api::v1::users::delete_user,
        crate::api::v1::feature_flags::create_flag,
        crate::api::v1::feature_flags::get_flags,
        crate::api::v1::feature_flags::get_flag_by_id,
        crate::api::v1::feature_flags::update_flag,
        crate::api::v1::feature_flags::delete_flag,
        crate::api::v1::feature_flags::enable_flag,
        crate::api::v1::feature_flags::disable_flag,
        crate::api::v1::feature_flags::check_feature,
    ),
    components(
        schemas(
            LoginRequest,
            LoginResponse,
            RefreshTokenRequest,
            RegisterRequest,
            CreateUser,
            UpdateUser,
            UserResponse,
            CreateFeatureFlag,
            UpdateFeatureFlag,
            FeatureFlagResponse,
            FeatureFlagStatus,
        )
    ),
    security(
        ("bearer_auth" = [])
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication endpoints (login, register, token refresh)"),
        (name = "Users", description = "User management endpoints (CRUD operations)"),
        (name = "Feature Flags", description = "Feature flag management endpoints (enable/disable features, gradual rollout)")
    )
)]
pub struct ApiDoc;

/// Security scheme addon for OpenAPI
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}
