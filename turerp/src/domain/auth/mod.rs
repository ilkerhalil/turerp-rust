//! Auth service

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::config::JwtConfig;
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
    jwt_service: JwtService,
}

impl AuthService {
    pub fn new(user_service: UserService, jwt_service: JwtService) -> Self {
        Self {
            user_service,
            jwt_service,
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

        // Create user
        let create = CreateUser {
            username: request.username,
            email: request.email,
            full_name: request.full_name,
            password: request.password,
            tenant_id,
            role: request.role.or(Some(Role::User)),
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
pub fn create_auth_service(user_service: UserService, jwt_config: &JwtConfig) -> AuthService {
    let jwt_service = JwtService::new(
        jwt_config.secret.clone(),
        jwt_config.access_token_expiration,
        jwt_config.refresh_token_expiration,
    );

    AuthService::new(user_service, jwt_service)
}

/// Create auth service with default configuration (dev/testing only)
#[cfg(any(test, debug_assertions))]
pub fn create_auth_service_dev(user_service: UserService) -> AuthService {
    let jwt_config = JwtConfig::dev();
    create_auth_service(user_service, &jwt_config)
}
