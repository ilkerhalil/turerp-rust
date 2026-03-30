//! User domain model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// User role enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Role {
    Admin,
    #[default]
    User,
    Viewer,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::User => write!(f, "user"),
            Role::Viewer => write!(f, "viewer"),
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Role::Admin),
            "user" => Ok(Role::User),
            "viewer" => Ok(Role::Viewer),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub hashed_password: String,
    pub tenant_id: i64,
    pub role: Role,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl User {
    /// Create a new user (for testing/in-memory)
    pub fn new(
        id: i64,
        username: String,
        email: String,
        full_name: String,
        hashed_password: String,
        tenant_id: i64,
    ) -> Self {
        Self {
            id,
            username,
            email,
            full_name,
            hashed_password,
            tenant_id,
            role: Role::default(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        }
    }
}

/// Data for creating a new user
#[derive(Debug, Clone, Deserialize, Serialize, validator::Validate, ToSchema)]
pub struct CreateUser {
    #[validate(length(min = 3, max = 50))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 1, max = 100))]
    pub full_name: String,

    #[validate(length(min = 12))]
    pub password: String,

    pub tenant_id: i64,

    pub role: Option<Role>,
}

impl CreateUser {
    /// Validate password complexity
    pub fn validate_password(&self) -> Result<(), String> {
        crate::utils::password::validate_password(&self.password).map_err(|e| e.message)
    }
}

/// Data for updating an existing user
#[derive(Debug, Clone, Deserialize, Serialize, Default, validator::Validate, ToSchema)]
pub struct UpdateUser {
    #[validate(length(min = 3, max = 50))]
    #[serde(default)]
    pub username: Option<String>,

    #[validate(email)]
    #[serde(default)]
    pub email: Option<String>,

    #[validate(length(min = 1, max = 100))]
    #[serde(default)]
    pub full_name: Option<String>,

    #[serde(default)]
    pub is_active: Option<bool>,

    #[serde(default)]
    pub role: Option<Role>,
}

/// User response (without sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub tenant_id: i64,
    pub role: Role,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            full_name: user.full_name,
            tenant_id: user.tenant_id,
            role: user.role,
            is_active: user.is_active,
            created_at: user.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use validator::Validate;

    #[test]
    fn test_role_display() {
        assert_eq!(Role::Admin.to_string(), "admin");
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Viewer.to_string(), "viewer");
    }

    #[test]
    fn test_role_from_str() {
        assert_eq!("admin".parse::<Role>().unwrap(), Role::Admin);
        assert_eq!("USER".parse::<Role>().unwrap(), Role::User);
        assert_eq!("Viewer".parse::<Role>().unwrap(), Role::Viewer);
        assert!(Role::from_str("invalid").is_err());
        assert!(Role::from_str("").is_err());
    }

    #[test]
    fn test_role_from_str_error_message() {
        let result = Role::from_str("superadmin");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid role"));
        assert!(err.contains("superadmin"));
    }

    #[test]
    fn test_role_serialization() {
        let admin = Role::Admin;
        let json = serde_json::to_string(&admin).unwrap();
        assert_eq!(json, "\"admin\"");

        let user = Role::User;
        let json = serde_json::to_string(&user).unwrap();
        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn test_role_deserialization() {
        let admin: Role = serde_json::from_str("\"admin\"").unwrap();
        assert_eq!(admin, Role::Admin);

        let user: Role = serde_json::from_str("\"user\"").unwrap();
        assert_eq!(user, Role::User);
    }

    #[test]
    fn test_role_default() {
        let role = Role::default();
        assert_eq!(role, Role::User);
    }

    #[test]
    fn test_role_equality() {
        assert_eq!(Role::Admin, Role::Admin);
        assert_eq!(Role::User, Role::User);
        assert_ne!(Role::Admin, Role::User);
    }

    #[test]
    fn test_user_new() {
        let user = User::new(
            42,
            "testuser".to_string(),
            "test@example.com".to_string(),
            "Test User".to_string(),
            "hashed_password".to_string(),
            1,
        );

        assert_eq!(user.id, 42);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.full_name, "Test User");
        assert_eq!(user.tenant_id, 1);
        assert_eq!(user.role, Role::User);
        assert!(user.is_active);
        assert!(user.created_at <= Utc::now());
    }

    #[test]
    fn test_user_response_from_user() {
        let user = User::new(
            1,
            "testuser".to_string(),
            "test@test.com".to_string(),
            "Test User".to_string(),
            "hash".to_string(),
            1,
        );

        let response: UserResponse = user.into();
        assert_eq!(response.username, "testuser");
        assert_eq!(response.email, "test@test.com");
        assert_eq!(response.full_name, "Test User");
        assert_eq!(response.tenant_id, 1);
        assert_eq!(response.role, Role::User);
        assert!(response.is_active);
    }

    #[test]
    fn test_user_response_no_password() {
        let user = User::new(
            1,
            "testuser".to_string(),
            "test@test.com".to_string(),
            "Test User".to_string(),
            "secret_hash".to_string(),
            1,
        );

        let response: UserResponse = user.into();
        let json = serde_json::to_string(&response).unwrap();

        // Password should not be in response
        assert!(!json.contains("secret_hash"));
        assert!(!json.contains("hashed_password"));
        assert!(!json.contains("password"));
    }

    #[test]
    fn test_create_user_validation() {
        let create = CreateUser {
            username: "validuser".to_string(),
            email: "valid@email.com".to_string(),
            full_name: "Valid Name".to_string(),
            password: "ValidPassword123!".to_string(),
            tenant_id: 1,
            role: None,
        };

        assert!(create.validate().is_ok());
    }

    #[test]
    fn test_create_user_short_username() {
        let create = CreateUser {
            username: "ab".to_string(), // Too short (min 3)
            email: "valid@email.com".to_string(),
            full_name: "Valid Name".to_string(),
            password: "ValidPassword123!".to_string(),
            tenant_id: 1,
            role: None,
        };

        assert!(create.validate().is_err());
    }

    #[test]
    fn test_create_user_invalid_email() {
        let create = CreateUser {
            username: "validuser".to_string(),
            email: "not-an-email".to_string(),
            full_name: "Valid Name".to_string(),
            password: "ValidPassword123!".to_string(),
            tenant_id: 1,
            role: None,
        };

        assert!(create.validate().is_err());
    }

    #[test]
    fn test_create_user_short_password() {
        let create = CreateUser {
            username: "validuser".to_string(),
            email: "valid@email.com".to_string(),
            full_name: "Valid Name".to_string(),
            password: "short".to_string(), // Too short (min 12)
            tenant_id: 1,
            role: None,
        };

        assert!(create.validate().is_err());
    }

    #[test]
    fn test_update_user_validation() {
        let update = UpdateUser {
            username: Some("newusername".to_string()),
            email: Some("new@email.com".to_string()),
            full_name: Some("New Name".to_string()),
            is_active: Some(false),
            role: Some(Role::Admin),
        };

        assert!(update.validate().is_ok());
    }

    #[test]
    fn test_update_user_default() {
        let update = UpdateUser::default();
        assert!(update.username.is_none());
        assert!(update.email.is_none());
        assert!(update.full_name.is_none());
        assert!(update.is_active.is_none());
        assert!(update.role.is_none());
    }

    #[test]
    fn test_user_is_active_default() {
        let user = User::new(
            1,
            "test".to_string(),
            "test@test.com".to_string(),
            "Test".to_string(),
            "hash".to_string(),
            1,
        );

        assert!(user.is_active);
    }
}
