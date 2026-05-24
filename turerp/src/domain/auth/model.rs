//! Auth data models

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::user::model::{Role, UserResponse};
use crate::utils::jwt::TokenPair;

/// Login request
#[derive(Debug, Clone, Deserialize, Serialize, validator::Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(length(min = 1))]
    pub username: String,

    #[validate(length(min = 1))]
    pub password: String,

    #[serde(default)]
    pub mfa_code: Option<String>,
}

/// Register request
#[derive(Debug, Clone, Deserialize, Serialize, validator::Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 1, max = 100))]
    pub full_name: String,

    #[validate(length(min = 12))]
    pub password: String,

    /// Tenant ID for the new user (required for registration)
    /// SECURITY: Must be explicitly provided - no default tenant to prevent
    /// accidental exposure of system tenant (id=1)
    pub tenant_id: i64,

    #[serde(default)]
    pub role: Option<Role>,
}

impl RegisterRequest {
    /// Validate password complexity
    pub fn validate_password(&self) -> Result<(), String> {
        crate::utils::password::validate_password(&self.password).map_err(|e| e.message)
    }
}

/// Token refresh request
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Login response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub tokens: TokenPair,
}

/// Logout request
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: String,
}
