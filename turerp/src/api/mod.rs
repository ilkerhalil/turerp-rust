//! API layer

pub mod auth;
pub mod users;

// Explicit re-exports to avoid ambiguity
pub use auth::configure as auth_configure;
pub use users::configure as users_configure;

use crate::domain::auth::{LoginRequest, LoginResponse, RefreshTokenRequest, RegisterRequest};
use crate::domain::user::{CreateUser, UpdateUser, UserResponse};
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::Modify;
use utoipa::OpenApi;

/// OpenAPI specification for the API
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Turerp ERP API",
        description = "Multi-tenant SaaS ERP system API\n\n## Authentication\n\nAll endpoints except `/api/auth/login`, `/api/auth/register`, and `/api/auth/refresh` require JWT Bearer token authentication.\n\n## Rate Limiting\n\nAuthentication endpoints are rate limited to 10 requests per minute per IP address with a burst of 3 requests.",
        version = "0.1.0",
        contact(
            name = "Turerp Team",
            email = "info@turerp.com"
        )
    ),
    paths(
        crate::api::auth::register,
        crate::api::auth::login,
        crate::api::auth::refresh_token,
        crate::api::auth::me,
        crate::api::users::create_user,
        crate::api::users::get_user,
        crate::api::users::get_users,
        crate::api::users::update_user,
        crate::api::users::delete_user,
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
        )
    ),
    security(
        ("bearer_auth" = [])
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication endpoints (login, register, token refresh)"),
        (name = "Users", description = "User management endpoints (CRUD operations)")
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
