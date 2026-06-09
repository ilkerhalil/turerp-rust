//! Password utilities

use crate::ApiError;
use bcrypt::{hash, verify, DEFAULT_COST};
use regex::Regex;
use std::sync::LazyLock;

// Compile regex patterns once for better performance
static UPPERCASE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Z]").expect("UPPERCASE_REGEX is valid"));
static LOWERCASE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[a-z]").expect("LOWERCASE_REGEX is valid"));
static DIGIT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[0-9]").expect("DIGIT_REGEX is valid"));
static SPECIAL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^A-Za-z0-9]").expect("SPECIAL_REGEX is valid"));

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
#[must_use = "The validation result should be checked before proceeding"]
pub fn validate_password(password: &str) -> Result<(), PasswordValidationError> {
    validate_password_with_requirements(password, &PasswordRequirements::default())
}

/// Validate password with custom requirements
#[must_use = "The validation result should be checked before proceeding"]
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

    if requirements.require_uppercase && !UPPERCASE_REGEX.is_match(password) {
        errors.push("Password must contain at least one uppercase letter".to_string());
    }

    if requirements.require_lowercase && !LOWERCASE_REGEX.is_match(password) {
        errors.push("Password must contain at least one lowercase letter".to_string());
    }

    if requirements.require_digit && !DIGIT_REGEX.is_match(password) {
        errors.push("Password must contain at least one digit".to_string());
    }

    if requirements.require_special && !SPECIAL_REGEX.is_match(password) {
        errors.push("Password must contain at least one special character".to_string());
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
#[must_use = "The hashed password should be stored, not discarded"]
pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

/// Verify a password against a hash.
///
/// Returns `Ok(true)` if the password matches the hash, `Ok(false)` if it
/// does not. Use [`check_password`] for authentication code paths — it
/// converts the `Ok(false)` case into an `ApiError::InvalidCredentials`
/// so callers cannot accidentally drop the result on the floor.
#[must_use = "The verification result should be checked for authentication"]
pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

/// Verify a password and return `Ok(())` only if it matches.
///
/// Unlike [`verify_password`], this function never returns `Ok(false)` —
/// a mismatch is reported as `Err(ApiError::InvalidCredentials)`. Callers
/// can use `?` without losing the result. Use this in any code path where
/// the result of password verification is the basis for an authentication
/// decision. Returning the bare `bool` and dropping it is a security
/// regression waiting to happen (see PR #147 for the previous incident).
pub fn check_password(password: &str, hash: &str) -> Result<(), ApiError> {
    verify_password(password, hash)
        .map_err(|_| ApiError::InvalidCredentials)
        .and_then(|valid| {
            if valid {
                Ok(())
            } else {
                Err(ApiError::InvalidCredentials)
            }
        })
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

    // ---- check_password (PR #147 regression suite) ---------------------
    //
    // These tests assert that check_password converts a bcrypt mismatch
    // (Ok(false)) into Err(ApiError::InvalidCredentials). If anyone ever
    // reverts check_password to return Ok(false) on mismatch, every test
    // below must fail.

    use crate::ApiError;

    #[test]
    fn test_check_password_correct() {
        let hash = hash_password("CorrectHorseBattery9!").unwrap();
        let result = check_password("CorrectHorseBattery9!", &hash);
        assert!(result.is_ok(), "correct password must be Ok(())");
    }

    #[test]
    fn test_check_password_incorrect_returns_invalid_credentials() {
        let hash = hash_password("CorrectHorseBattery9!").unwrap();
        let result = check_password("totally-wrong-password", &hash);
        // The whole point: a mismatch must be Err, not silently Ok.
        // This is the exact assertion that would have caught the bug
        // fixed in PR #147 before it shipped.
        assert!(
            matches!(result, Err(ApiError::InvalidCredentials)),
            "wrong password must be Err(InvalidCredentials), got {:?}",
            result
        );
    }

    #[test]
    fn test_check_password_empty_against_real_hash_is_invalid() {
        let hash = hash_password("CorrectHorseBattery9!").unwrap();
        let result = check_password("", &hash);
        assert!(matches!(result, Err(ApiError::InvalidCredentials)));
    }

    #[test]
    fn test_check_password_case_sensitive() {
        let hash = hash_password("CorrectHorseBattery9!").unwrap();
        // bcrypt hashes are case-sensitive: changing case must fail.
        let result = check_password("correcthorsebattery9!", &hash);
        assert!(matches!(result, Err(ApiError::InvalidCredentials)));
    }

    #[test]
    fn test_check_password_garbage_hash_returns_invalid_credentials() {
        // A malformed (non-bcrypt) hash causes bcrypt to return Err.
        // check_password must still surface this as InvalidCredentials,
        // not propagate the bcrypt error or return Ok.
        let result = check_password("anything", "not-a-real-bcrypt-hash");
        assert!(
            matches!(result, Err(ApiError::InvalidCredentials)),
            "garbage hash must be Err(InvalidCredentials), got {:?}",
            result
        );
    }
}
