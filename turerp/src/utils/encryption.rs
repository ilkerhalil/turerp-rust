//! Encryption utilities for sensitive data
//!
//! Uses AES-256-GCM for authenticated encryption.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use thiserror::Error;

/// Encryption error types
#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Invalid key length: expected 32 bytes, got {0}")]
    InvalidKeyLength(usize),
    #[error("Invalid ciphertext format")]
    InvalidCiphertext,
}

/// Result type for encryption operations
pub type EncryptionResult<T> = Result<T, EncryptionError>;

/// Encrypts data using AES-256-GCM
///
/// The key must be exactly 32 bytes (256 bits).
/// Returns a base64-encoded string containing the nonce + ciphertext.
///
/// # Arguments
/// * `plaintext` - The data to encrypt
/// * `key` - The encryption key (32 bytes)
///
/// # Returns
/// Base64-encoded ciphertext (nonce + encrypted data)
pub fn encrypt(plaintext: &str, key: &[u8]) -> EncryptionResult<String> {
    if key.len() != 32 {
        return Err(EncryptionError::InvalidKeyLength(key.len()));
    }

    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

    // Combine nonce + ciphertext and encode as base64
    let mut combined = nonce_bytes.to_vec();
    combined.extend(ciphertext);

    Ok(BASE64.encode(&combined))
}

/// Decrypts data encrypted with AES-256-GCM
///
/// # Arguments
/// * `ciphertext` - Base64-encoded ciphertext (nonce + encrypted data)
/// * `key` - The decryption key (32 bytes)
///
/// # Returns
/// Decrypted plaintext string
pub fn decrypt(ciphertext: &str, key: &[u8]) -> EncryptionResult<String> {
    if key.len() != 32 {
        return Err(EncryptionError::InvalidKeyLength(key.len()));
    }

    // Decode base64
    let combined = BASE64
        .decode(ciphertext)
        .map_err(|_| EncryptionError::InvalidCiphertext)?;

    // Split into nonce and ciphertext
    if combined.len() < 12 {
        return Err(EncryptionError::InvalidCiphertext);
    }

    let (nonce_bytes, encrypted_data) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Create cipher
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, encrypted_data)
        .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

    String::from_utf8(plaintext).map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))
}

/// Generate a new encryption key
///
/// # Returns
/// A 32-byte random key suitable for AES-256-GCM
pub fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = generate_key();
        let plaintext = "secret data";

        let encrypted = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&encrypted, &key).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertext() {
        let key = generate_key();
        let plaintext = "secret data";

        let encrypted1 = encrypt(plaintext, &key).unwrap();
        let encrypted2 = encrypt(plaintext, &key).unwrap();

        // Due to random nonce, ciphertext should be different
        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_invalid_key_length() {
        let short_key = [0u8; 16];
        let result = encrypt("test", &short_key);
        assert!(matches!(result, Err(EncryptionError::InvalidKeyLength(16))));
    }

    #[test]
    fn test_decrypt_invalid_ciphertext() {
        let key = generate_key();
        let result = decrypt("not-valid-base64!@#", &key);
        assert!(matches!(result, Err(EncryptionError::InvalidCiphertext)));
    }
}
