//! API layer

pub mod auth;
pub mod users;

// Explicit re-exports to avoid ambiguity
pub use auth::configure as auth_configure;
pub use users::configure as users_configure;

use crate::domain::auth::{LoginRequest, LoginResponse, RefreshTokenRequest, RegisterRequest};
use crate::domain::user::{CreateUser, UpdateUser, UserResponse};
use utoipa::OpenApi;

/// OpenAPI specification for the API
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Turerp ERP API",
        description = "Multi-tenant SaaS ERP system API",
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
    tags(
        (name = "Auth", description = "Authentication endpoints"),
        (name = "Users", description = "User management endpoints")
    )
)]
pub struct ApiDoc;
