//! MFA service for business logic

use chrono::Utc;
use rand::Rng;
use sha2::{Digest, Sha256};

use super::model::{BackupCodesResponse, MfaMethod, MfaSetupResponse, MfaStatusResponse};
use super::repository::BoxMfaRepository;
use crate::error::ApiError;
use crate::utils::jwt::JwtService;

/// MFA service
#[derive(Clone)]
pub struct MfaService {
    repo: BoxMfaRepository,
    jwt_service: JwtService,
}

impl MfaService {
    pub fn new(repo: BoxMfaRepository, jwt_service: JwtService) -> Self {
        Self { repo, jwt_service }
    }

    /// Generate a new base32 TOTP secret
    pub fn generate_totp_secret() -> String {
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..20).map(|_| rng.gen()).collect();
        base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &bytes)
    }

    /// Generate an otpauth:// URI for QR code generation
    pub fn generate_qr_code_uri(secret: &str, user_email: &str, issuer: &str) -> String {
        format!(
            "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits=6&period=30",
            issuer, user_email, secret, issuer
        )
    }

    /// Verify a TOTP code against a secret
    pub fn verify_totp(secret: &str, code: &str) -> bool {
        if code.len() != 6 {
            return false;
        }

        let totp = match totp_rs::TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            secret.as_bytes().to_vec(),
            None,
            "".to_string(),
        ) {
            Ok(t) => t,
            Err(_) => return false,
        };

        totp.check_current(code).unwrap_or(false)
    }

    /// Generate random backup codes
    pub fn generate_backup_codes(count: usize) -> Vec<String> {
        let mut rng = rand::thread_rng();
        (0..count)
            .map(|_| {
                let bytes: Vec<u8> = (0..4).map(|_| rng.gen()).collect();
                base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &bytes)
            })
            .collect()
    }

    /// Hash a backup code for storage using SHA-256
    pub fn hash_backup_code(code: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(code.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify a backup code against stored hashed codes
    pub fn verify_backup_code_hash(code: &str, hashed_codes: &[String]) -> bool {
        let hash = Self::hash_backup_code(code);
        hashed_codes.iter().any(|hc| hc == &hash)
    }

    /// Check if a user has MFA enabled
    pub async fn is_mfa_enabled(&self, user_id: i64, tenant_id: i64) -> Result<bool, ApiError> {
        let settings = self.repo.find_by_user_id(user_id, tenant_id).await?;
        Ok(settings.map(|s| s.mfa_enabled).unwrap_or(false))
    }

    /// Start MFA setup — generate secret and return QR URI
    pub async fn setup_mfa(
        &self,
        user_id: i64,
        tenant_id: i64,
        user_email: &str,
        issuer: &str,
    ) -> Result<MfaSetupResponse, ApiError> {
        let secret = Self::generate_totp_secret();
        let qr_code_uri = Self::generate_qr_code_uri(&secret, user_email, issuer);

        self.repo
            .update_totp_secret(user_id, tenant_id, Some(secret.clone()))
            .await?;

        Ok(MfaSetupResponse {
            qr_code_uri,
            secret,
        })
    }

    /// Verify setup code and enable MFA
    pub async fn verify_setup(
        &self,
        user_id: i64,
        tenant_id: i64,
        code: &str,
    ) -> Result<MfaStatusResponse, ApiError> {
        let settings = self
            .repo
            .find_by_user_id(user_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::BadRequest("MFA setup not initiated".to_string()))?;

        let secret = settings
            .totp_secret
            .as_ref()
            .ok_or_else(|| ApiError::BadRequest("TOTP secret not set".to_string()))?;

        if !Self::verify_totp(secret, code) {
            return Err(ApiError::Validation("Invalid TOTP code".to_string()));
        }

        // Generate and hash backup codes
        let raw_codes = Self::generate_backup_codes(10);
        let hashed_codes: Vec<String> = raw_codes
            .iter()
            .map(|c| Self::hash_backup_code(c))
            .collect();

        let new_settings = super::model::MfaSettings {
            user_id,
            tenant_id,
            totp_secret: Some(secret.clone()),
            mfa_enabled: true,
            backup_codes: hashed_codes,
            method: MfaMethod::Totp,
            created_at: settings.created_at,
            updated_at: Some(Utc::now()),
        };

        self.repo.save(&new_settings).await?;

        Ok(MfaStatusResponse {
            user_id,
            mfa_enabled: true,
            method: MfaMethod::Totp,
        })
    }

    /// Disable MFA for a user
    pub async fn disable_mfa(
        &self,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<MfaStatusResponse, ApiError> {
        let new_settings = super::model::MfaSettings {
            user_id,
            tenant_id,
            totp_secret: None,
            mfa_enabled: false,
            backup_codes: Vec::new(),
            method: MfaMethod::None,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
        };

        self.repo.save(&new_settings).await?;

        Ok(MfaStatusResponse {
            user_id,
            mfa_enabled: false,
            method: MfaMethod::None,
        })
    }

    /// Get MFA status for a user
    pub async fn get_mfa_status(
        &self,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<MfaStatusResponse, ApiError> {
        let settings = self.repo.find_by_user_id(user_id, tenant_id).await?;
        Ok(settings
            .map(|s| s.into())
            .unwrap_or_else(|| MfaStatusResponse {
                user_id,
                mfa_enabled: false,
                method: MfaMethod::None,
            }))
    }

    /// Generate new backup codes
    pub async fn regenerate_backup_codes(
        &self,
        user_id: i64,
        tenant_id: i64,
    ) -> Result<BackupCodesResponse, ApiError> {
        let settings = self
            .repo
            .find_by_user_id(user_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::BadRequest("MFA not set up".to_string()))?;

        if !settings.mfa_enabled {
            return Err(ApiError::BadRequest("MFA is not enabled".to_string()));
        }

        let raw_codes = Self::generate_backup_codes(10);
        let hashed_codes: Vec<String> = raw_codes
            .iter()
            .map(|c| Self::hash_backup_code(c))
            .collect();

        self.repo
            .add_backup_codes(user_id, tenant_id, hashed_codes)
            .await?;

        Ok(BackupCodesResponse {
            backup_codes: raw_codes,
        })
    }

    /// Validate an MFA challenge (TOTP or backup code)
    pub async fn validate_mfa_challenge(
        &self,
        user_id: i64,
        tenant_id: i64,
        code: &str,
    ) -> Result<bool, ApiError> {
        let settings = self
            .repo
            .find_by_user_id(user_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::BadRequest("MFA not configured".to_string()))?;

        if !settings.mfa_enabled {
            return Ok(true);
        }

        // Try TOTP first
        if let Some(ref secret) = settings.totp_secret {
            if Self::verify_totp(secret, code) {
                return Ok(true);
            }
        }

        // Try backup code
        if Self::verify_backup_code_hash(code, &settings.backup_codes) {
            // Invalidate the used backup code
            self.repo
                .invalidate_backup_code(user_id, tenant_id, code)
                .await?;
            return Ok(true);
        }

        Ok(false)
    }

    /// Generate a short-lived MFA token (temporary JWT)
    pub fn generate_mfa_token(
        &self,
        user_id: i64,
        tenant_id: i64,
        username: String,
    ) -> Result<String, ApiError> {
        // Generate a token with 5 minute expiration using existing JWT service
        // We encode a special MFA claims using the standard auth claims with a short expiry
        let claims = crate::utils::jwt::AuthClaims::new(
            user_id,
            tenant_id,
            username,
            crate::domain::user::model::Role::User,
            300, // 5 minutes
        );

        self.jwt_service.encode_token(&claims)
    }

    /// Decode an MFA token
    pub fn decode_mfa_token(&self, token: &str) -> Result<crate::utils::jwt::AuthClaims, ApiError> {
        self.jwt_service.decode_token(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mfa::repository::InMemoryMfaRepository;
    use std::sync::Arc;

    fn create_service() -> MfaService {
        let repo = Arc::new(InMemoryMfaRepository::new()) as BoxMfaRepository;
        let jwt_service = JwtService::new("test-secret".to_string(), 3600, 604800);
        MfaService::new(repo, jwt_service)
    }

    #[test]
    fn test_generate_totp_secret() {
        let secret = MfaService::generate_totp_secret();
        assert!(!secret.is_empty());
        // Should be valid base32
        assert!(base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret).is_some());
    }

    #[test]
    fn test_generate_qr_code_uri() {
        let uri = MfaService::generate_qr_code_uri("SECRET123", "user@example.com", "Turerp");
        assert!(uri.contains("otpauth://totp/"));
        assert!(uri.contains("SECRET123"));
        assert!(uri.contains("user@example.com"));
        assert!(uri.contains("Turerp"));
    }

    #[test]
    fn test_generate_backup_codes() {
        let codes = MfaService::generate_backup_codes(10);
        assert_eq!(codes.len(), 10);
        for code in &codes {
            assert!(!code.is_empty());
        }
    }

    #[test]
    fn test_hash_backup_code() {
        let hash1 = MfaService::hash_backup_code("code123");
        let hash2 = MfaService::hash_backup_code("code123");
        assert_eq!(hash1, hash2);

        let hash3 = MfaService::hash_backup_code("different");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_verify_backup_code_hash() {
        let hashed = vec![MfaService::hash_backup_code("code123")];
        assert!(MfaService::verify_backup_code_hash("code123", &hashed));
        assert!(!MfaService::verify_backup_code_hash("wrong", &hashed));
    }

    #[tokio::test]
    async fn test_setup_and_verify_mfa() {
        let service = create_service();

        let setup = service
            .setup_mfa(1, 1, "user@test.com", "Turerp")
            .await
            .unwrap();
        assert!(!setup.secret.is_empty());
        assert!(setup.qr_code_uri.contains(&setup.secret));

        // Generate a valid TOTP code
        let totp = totp_rs::TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            setup.secret.as_bytes().to_vec(),
            None,
            "".to_string(),
        )
        .unwrap();
        let code = totp.generate_current().unwrap();

        let result = service.verify_setup(1, 1, &code).await.unwrap();
        assert!(result.mfa_enabled);
        assert_eq!(result.method, MfaMethod::Totp);
    }

    #[tokio::test]
    async fn test_verify_setup_invalid_code() {
        let service = create_service();
        service
            .setup_mfa(1, 1, "user@test.com", "Turerp")
            .await
            .unwrap();

        let result = service.verify_setup(1, 1, "000000").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_disable_mfa() {
        let service = create_service();
        service
            .setup_mfa(1, 1, "user@test.com", "Turerp")
            .await
            .unwrap();

        let totp = totp_rs::TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            service
                .repo
                .find_by_user_id(1, 1)
                .await
                .unwrap()
                .unwrap()
                .totp_secret
                .unwrap()
                .as_bytes()
                .to_vec(),
            None,
            "".to_string(),
        )
        .unwrap();
        let code = totp.generate_current().unwrap();
        service.verify_setup(1, 1, &code).await.unwrap();

        let status = service.disable_mfa(1, 1).await.unwrap();
        assert!(!status.mfa_enabled);
    }

    #[tokio::test]
    async fn test_regenerate_backup_codes() {
        let service = create_service();
        service
            .setup_mfa(1, 1, "user@test.com", "Turerp")
            .await
            .unwrap();

        let totp = totp_rs::TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            service
                .repo
                .find_by_user_id(1, 1)
                .await
                .unwrap()
                .unwrap()
                .totp_secret
                .unwrap()
                .as_bytes()
                .to_vec(),
            None,
            "".to_string(),
        )
        .unwrap();
        let code = totp.generate_current().unwrap();
        service.verify_setup(1, 1, &code).await.unwrap();

        let response = service.regenerate_backup_codes(1, 1).await.unwrap();
        assert_eq!(response.backup_codes.len(), 10);
    }

    #[tokio::test]
    async fn test_validate_mfa_challenge_with_totp() {
        let service = create_service();
        service
            .setup_mfa(1, 1, "user@test.com", "Turerp")
            .await
            .unwrap();

        let totp = totp_rs::TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            service
                .repo
                .find_by_user_id(1, 1)
                .await
                .unwrap()
                .unwrap()
                .totp_secret
                .unwrap()
                .as_bytes()
                .to_vec(),
            None,
            "".to_string(),
        )
        .unwrap();
        let code = totp.generate_current().unwrap();
        service.verify_setup(1, 1, &code).await.unwrap();

        let valid = service.validate_mfa_challenge(1, 1, &code).await.unwrap();
        assert!(valid);
    }

    #[tokio::test]
    async fn test_validate_mfa_challenge_with_backup_code() {
        let service = create_service();
        service
            .setup_mfa(1, 1, "user@test.com", "Turerp")
            .await
            .unwrap();

        let totp = totp_rs::TOTP::new(
            totp_rs::Algorithm::SHA1,
            6,
            1,
            30,
            service
                .repo
                .find_by_user_id(1, 1)
                .await
                .unwrap()
                .unwrap()
                .totp_secret
                .unwrap()
                .as_bytes()
                .to_vec(),
            None,
            "".to_string(),
        )
        .unwrap();
        let code = totp.generate_current().unwrap();
        service.verify_setup(1, 1, &code).await.unwrap();

        let backup_codes = service.regenerate_backup_codes(1, 1).await.unwrap();
        let backup_code = &backup_codes.backup_codes[0];

        let valid = service
            .validate_mfa_challenge(1, 1, backup_code)
            .await
            .unwrap();
        assert!(valid);

        // Backup code should be invalidated after use
        let invalid = service
            .validate_mfa_challenge(1, 1, backup_code)
            .await
            .unwrap();
        assert!(!invalid);
    }

    #[tokio::test]
    async fn test_generate_mfa_token() {
        let service = create_service();
        let token = service
            .generate_mfa_token(1, 1, "testuser".to_string())
            .unwrap();
        assert!(!token.is_empty());

        let claims = service.decode_mfa_token(&token).unwrap();
        assert_eq!(claims.sub, "1");
        assert_eq!(claims.tenant_id, 1);
    }
}
