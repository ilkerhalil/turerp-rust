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
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

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

/// Login attempt tracker for brute-force protection
#[derive(Clone)]
struct LoginAttemptTracker {
    attempts: u32,
    locked_until: Option<Instant>,
}

/// Auth service
#[derive(Clone)]
pub struct AuthService {
    user_service: UserService,
    pub jwt_service: JwtService,
    mfa_service: MfaService,
    login_attempts: Arc<Mutex<HashMap<String, LoginAttemptTracker>>>,
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
            login_attempts: Arc::new(Mutex::new(HashMap::new())),
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

        // Check if account is temporarily locked
        {
            let mut attempts = self.login_attempts.lock();
            if let Some(tracker) = attempts.get_mut(&request.username) {
                if let Some(locked_until) = tracker.locked_until {
                    let now = Instant::now();
                    if now < locked_until {
                        let remaining_secs = (locked_until - now).as_secs();
                        let remaining_mins = (remaining_secs / 60).max(1);
                        return Err(ApiError::Unauthorized(format!(
                            "Account temporarily locked due to too many failed attempts. Try again in {} minutes.",
                            remaining_mins
                        )));
                    } else {
                        // Lock has expired — clear the tracker
                        tracker.locked_until = None;
                        tracker.attempts = 0;
                    }
                }
            }
        }

        // Verify credentials
        let user_result = self
            .user_service
            .verify_credentials(&request.username, &request.password, tenant_id)
            .await;

        match user_result {
            Ok(user) => {
                // Reset failed attempts on successful login
                {
                    let mut attempts = self.login_attempts.lock();
                    attempts.remove(&request.username);
                }

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
            Err(ApiError::InvalidCredentials) => {
                // Increment failed attempt counter
                {
                    let mut attempts = self.login_attempts.lock();
                    let tracker =
                        attempts
                            .entry(request.username.clone())
                            .or_insert(LoginAttemptTracker {
                                attempts: 0,
                                locked_until: None,
                            });
                    tracker.attempts += 1;
                    if tracker.attempts >= 5 {
                        tracker.locked_until = Some(Instant::now() + Duration::from_secs(900));
                    }
                }
                Err(ApiError::InvalidCredentials)
            }
            Err(e) => Err(e),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::Arc;

    use crate::domain::mfa::repository::{BoxMfaRepository, InMemoryMfaRepository};
    use crate::domain::mfa::service::MfaService;
    use crate::domain::user::repository::{BoxUserRepository, InMemoryUserRepository};
    use crate::domain::user::service::UserService;
    use crate::utils::jwt::JwtService;

    fn create_test_auth_service() -> AuthService {
        let user_repo = Arc::new(InMemoryUserRepository::new()) as BoxUserRepository;
        let user_service = UserService::new(user_repo);
        let mfa_repo = Arc::new(InMemoryMfaRepository::new()) as BoxMfaRepository;
        let jwt_service = JwtService::new("test-secret".to_string(), 3600, 86400);
        let mfa_service = MfaService::new(mfa_repo, jwt_service);
        create_auth_service_dev(user_service, mfa_service)
    }

    #[test]
    fn test_valid_password() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            full_name: "Test User".to_string(),
            password: "ValidPass123!".to_string(),
            tenant_id: 1,
            role: None,
        };
        assert!(req.validate_password().is_ok());
    }

    #[test]
    fn test_short_password() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            full_name: "Test User".to_string(),
            password: "Short1!".to_string(),
            tenant_id: 1,
            role: None,
        };
        assert!(req.validate_password().is_err());
    }

    #[test]
    fn test_password_missing_uppercase() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            full_name: "Test User".to_string(),
            password: "invalidpass123!".to_string(),
            tenant_id: 1,
            role: None,
        };
        assert!(req.validate_password().is_err());
    }

    #[test]
    fn test_password_missing_lowercase() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            full_name: "Test User".to_string(),
            password: "INVALIDPASS123!".to_string(),
            tenant_id: 1,
            role: None,
        };
        assert!(req.validate_password().is_err());
    }

    #[test]
    fn test_password_missing_digit() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            full_name: "Test User".to_string(),
            password: "InvalidPass!".to_string(),
            tenant_id: 1,
            role: None,
        };
        assert!(req.validate_password().is_err());
    }

    #[test]
    fn test_password_missing_special() {
        let req = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            full_name: "Test User".to_string(),
            password: "InvalidPass123".to_string(),
            tenant_id: 1,
            role: None,
        };
        assert!(req.validate_password().is_err());
    }

    #[tokio::test]
    async fn test_self_registration_admin_forced_to_user() {
        let auth_service = create_test_auth_service();
        let req = RegisterRequest {
            username: "adminuser".to_string(),
            email: "admin@example.com".to_string(),
            full_name: "Admin User".to_string(),
            password: "ValidPass123!".to_string(),
            tenant_id: 1,
            role: Some(Role::Admin),
        };
        let result = auth_service.register(req).await.unwrap();
        assert_eq!(result.user.role, Role::User);
    }

    #[tokio::test]
    async fn test_self_registration_user_stays_user() {
        let auth_service = create_test_auth_service();
        let req = RegisterRequest {
            username: "normaluser".to_string(),
            email: "user@example.com".to_string(),
            full_name: "Normal User".to_string(),
            password: "ValidPass123!".to_string(),
            tenant_id: 1,
            role: Some(Role::User),
        };
        let result = auth_service.register(req).await.unwrap();
        assert_eq!(result.user.role, Role::User);
    }

    #[tokio::test]
    async fn test_self_registration_no_role_defaults_to_user() {
        let auth_service = create_test_auth_service();
        let req = RegisterRequest {
            username: "defaultuser".to_string(),
            email: "default@example.com".to_string(),
            full_name: "Default User".to_string(),
            password: "ValidPass123!".to_string(),
            tenant_id: 1,
            role: None,
        };
        let result = auth_service.register(req).await.unwrap();
        assert_eq!(result.user.role, Role::User);
    }

    #[test]
    fn test_login_request_empty_username() {
        let req = LoginRequest {
            username: "".to_string(),
            password: "somepassword".to_string(),
            mfa_code: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_login_request_empty_password() {
        let req = LoginRequest {
            username: "someuser".to_string(),
            password: "".to_string(),
            mfa_code: None,
        };
        let result = req.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_auth_service_creation() {
        let _service = create_test_auth_service();
    }

    #[test]
    fn test_refresh_token_request_serialization() {
        let req = RefreshTokenRequest {
            refresh_token: "test-refresh-token".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("test-refresh-token"));

        let deserialized: RefreshTokenRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.refresh_token, "test-refresh-token");
    }

    #[test]
    fn test_login_response_serialization() {
        let user = UserResponse {
            id: 1,
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            full_name: "Test User".to_string(),
            tenant_id: 1,
            role: Role::User,
            is_active: true,
            created_at: Utc::now(),
        };
        let tokens = TokenPair {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
        };
        let response = LoginResponse { user, tokens };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("access"));

        let deserialized: LoginResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user.username, "testuser");
        assert_eq!(deserialized.tokens.access_token, "access");
    }
}
