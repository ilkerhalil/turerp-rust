//! Auth service

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::config::JwtConfig;
use crate::domain::mfa::service::MfaService;
use crate::domain::user::model::{CreateUser, Role, UserResponse};
use crate::domain::user::service::UserService;
use crate::error::ApiError;
use crate::utils::jwt::{JwtService, TokenPair};

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

/// Auth service
#[derive(Clone)]
pub struct AuthService {
    user_service: UserService,
    pub jwt_service: JwtService,
    mfa_service: MfaService,
}

impl AuthService {
    pub fn new(
        user_service: UserService,
        jwt_service: JwtService,
        mfa_service: MfaService,
    ) -> Self {
        Self {
            user_service,
            jwt_service,
            mfa_service,
        }
    }

    /// Register a new user
    pub async fn register(&self, request: RegisterRequest) -> Result<LoginResponse, ApiError> {
        // Validate input
        request
            .validate()
            .map_err(|e: validator::ValidationErrors| ApiError::Validation(e.to_string()))?;

        // Validate password complexity
        request.validate_password().map_err(ApiError::Validation)?;

        // SECURITY: tenant_id is required and explicitly provided by the caller
        // No default is used to prevent accidental exposure of system tenant
        let tenant_id = request.tenant_id;

        // SECURITY: Only admins can create admin accounts.
        // Self-registration defaults to Role::User regardless of requested role.
        let role = if request.role == Some(Role::Admin) {
            tracing::warn!(
                "Self-registration requested Admin role for tenant {} - forcing User role",
                tenant_id
            );
            Some(Role::User)
        } else {
            request.role.or(Some(Role::User))
        };

        // Create user
        let create = CreateUser {
            username: request.username,
            email: request.email,
            full_name: request.full_name,
            password: request.password,
            tenant_id,
            role,
        };

        let user_response = self.user_service.create_user(create).await?;

        // Get full user for token generation
        let user = self
            .user_service
            .get_user_by_username(&user_response.username, tenant_id)
            .await?;

        // Generate tokens
        let tokens =
            self.jwt_service
                .generate_tokens(user.id, user.tenant_id, user.username, user.role)?;

        Ok(LoginResponse {
            user: user_response,
            tokens,
        })
    }

    /// Login user
    pub async fn login(
        &self,
        request: LoginRequest,
        tenant_id: i64,
    ) -> Result<LoginResponse, ApiError> {
        // Validate input
        request
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;

        // Verify credentials
        let user = self
            .user_service
            .verify_credentials(&request.username, &request.password, tenant_id)
            .await?;

        // Check if MFA is enabled for this user
        let mfa_enabled = self.mfa_service.is_mfa_enabled(user.id, tenant_id).await?;

        if mfa_enabled {
            // If MFA code provided, verify it
            if let Some(mfa_code) = request.mfa_code {
                let valid = self
                    .mfa_service
                    .validate_mfa_challenge(user.id, tenant_id, &mfa_code)
                    .await?;
                if !valid {
                    return Err(ApiError::Unauthorized("Invalid MFA code".to_string()));
                }
            } else {
                // MFA required but no code provided — return temporary MFA token
                let mfa_token = self.mfa_service.generate_mfa_token(
                    user.id,
                    tenant_id,
                    user.username.clone(),
                )?;

                return Err(ApiError::MfaRequired(mfa_token));
            }
        }

        // Generate tokens
        let (user_id, tenant_id, username, role) =
            (user.id, user.tenant_id, user.username.clone(), user.role);
        let tokens = self
            .jwt_service
            .generate_tokens(user_id, tenant_id, username, role)?;

        Ok(LoginResponse {
            user: user.into(),
            tokens,
        })
    }

    /// Refresh access token
    pub async fn refresh_token(&self, request: RefreshTokenRequest) -> Result<TokenPair, ApiError> {
        self.jwt_service.refresh_tokens(&request.refresh_token)
    }

    /// Validate access token
    pub fn validate_token(&self, token: &str) -> Result<crate::utils::jwt::AuthClaims, ApiError> {
        self.jwt_service.decode_token(token)
    }
}

/// Create auth service with JWT configuration
pub fn create_auth_service(
    user_service: UserService,
    mfa_service: MfaService,
    jwt_config: &JwtConfig,
) -> AuthService {
    let jwt_service = JwtService::new(
        jwt_config.secret.clone(),
        jwt_config.access_token_expiration,
        jwt_config.refresh_token_expiration,
    );

    AuthService::new(user_service, jwt_service, mfa_service)
}

/// Create auth service with default configuration (dev/testing only)
#[cfg(any(test, debug_assertions))]
pub fn create_auth_service_dev(user_service: UserService, mfa_service: MfaService) -> AuthService {
    let jwt_config = JwtConfig::dev();
    create_auth_service(user_service, mfa_service, &jwt_config)
}
