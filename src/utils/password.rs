//! Password utilities

use bcrypt::{hash, verify, DEFAULT_COST};

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
        let hash = hash_password("password123").unwrap();
        assert!(!hash.is_empty());
        assert!(!hash.contains("password123"));
    }

    #[test]
    fn test_verify_password_correct() {
        let hash = hash_password("password123").unwrap();
        let result = verify_password("password123", &hash).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_password_incorrect() {
        let hash = hash_password("password123").unwrap();
        let result = verify_password("wrongpassword", &hash).unwrap();
        assert!(!result);
    }
}
