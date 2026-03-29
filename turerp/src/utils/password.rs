//! Password utilities

use bcrypt::{hash, verify, DEFAULT_COST};
use regex::Regex;

/// Minimum password length
pub const MIN_PASSWORD_LENGTH: usize = 12;

/// Password complexity requirements
#[derive(Debug, Clone)]
pub struct PasswordRequirements {
    /// Minimum length (default: 12)
    pub min_length: usize,
    /// Require at least one uppercase letter
    pub require_uppercase: bool,
    /// Require at least one lowercase letter
    pub require_lowercase: bool,
    /// Require at least one digit
    pub require_digit: bool,
    /// Require at least one special character
    pub require_special: bool,
}

impl Default for PasswordRequirements {
    fn default() -> Self {
        Self {
            min_length: MIN_PASSWORD_LENGTH,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: true,
        }
    }
}

/// Password validation error
#[derive(Debug, Clone)]
pub struct PasswordValidationError {
    pub message: String,
}

impl std::fmt::Display for PasswordValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PasswordValidationError {}

/// Validate password complexity
pub fn validate_password(password: &str) -> Result<(), PasswordValidationError> {
    validate_password_with_requirements(password, &PasswordRequirements::default())
}

/// Validate password with custom requirements
pub fn validate_password_with_requirements(
    password: &str,
    requirements: &PasswordRequirements,
) -> Result<(), PasswordValidationError> {
    let mut errors = Vec::new();

    if password.len() < requirements.min_length {
        errors.push(format!(
            "Password must be at least {} characters long",
            requirements.min_length
        ));
    }

    if requirements.require_uppercase {
        let uppercase_regex = Regex::new(r"[A-Z]").unwrap();
        if !uppercase_regex.is_match(password) {
            errors.push("Password must contain at least one uppercase letter".to_string());
        }
    }

    if requirements.require_lowercase {
        let lowercase_regex = Regex::new(r"[a-z]").unwrap();
        if !lowercase_regex.is_match(password) {
            errors.push("Password must contain at least one lowercase letter".to_string());
        }
    }

    if requirements.require_digit {
        let digit_regex = Regex::new(r"[0-9]").unwrap();
        if !digit_regex.is_match(password) {
            errors.push("Password must contain at least one digit".to_string());
        }
    }

    if requirements.require_special {
        let special_regex = Regex::new(r"[^A-Za-z0-9]").unwrap();
        if !special_regex.is_match(password) {
            errors.push("Password must contain at least one special character".to_string());
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(PasswordValidationError {
            message: errors.join(". "),
        })
    }
}

/// Hash a password using bcrypt
pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password() {
        let hash = hash_password("Password123!").unwrap();
        assert!(!hash.is_empty());
        assert!(!hash.contains("Password123!"));
    }

    #[test]
    fn test_verify_password_correct() {
        let hash = hash_password("Password123!").unwrap();
        let result = verify_password("Password123!", &hash).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_password_incorrect() {
        let hash = hash_password("Password123!").unwrap();
        let result = verify_password("wrongpassword", &hash).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_validate_password_success() {
        let result = validate_password("Password123!");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_password_too_short() {
        let result = validate_password("Pass1!");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("12 characters"));
    }

    #[test]
    fn test_validate_password_no_uppercase() {
        let result = validate_password("password123!");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("uppercase"));
    }

    #[test]
    fn test_validate_password_no_lowercase() {
        let result = validate_password("PASSWORD123!");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("lowercase"));
    }

    #[test]
    fn test_validate_password_no_digit() {
        let result = validate_password("Password!!!@");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("digit"));
    }

    #[test]
    fn test_validate_password_no_special() {
        let result = validate_password("Password123");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("special character"));
    }

    #[test]
    fn test_validate_password_weak() {
        // Common weak password "12345678" should fail multiple checks
        let result = validate_password("12345678");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.message.contains("12 characters") || error.message.contains("uppercase"));
    }
}
