//! JWT utilities for authentication

use chrono::Utc;
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::user::model::Role;
use crate::error::ApiError;

/// JWT claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthClaims {
    pub sub: String, // User ID
    pub tenant_id: i64,
    pub username: String,
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cari_id: Option<i64>,
    pub exp: i64,
    pub iat: i64,
    pub aud: String,
    pub iss: String,
}

impl AuthClaims {
    pub fn new(
        user_id: i64,
        tenant_id: i64,
        username: String,
        role: Role,
        expires_in: i64,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            sub: user_id.to_string(),
            tenant_id,
            username,
            role: role.to_string(),
            cari_id: None,
            exp: now + expires_in,
            iat: now,
            aud: "turerp-api".to_string(),
            iss: "turerp-auth".to_string(),
        }
    }

    /// Parse the `sub` claim as a user ID, returning an error on invalid tokens.
    pub fn user_id(&self) -> Result<i64, ApiError> {
        self.sub
            .parse()
            .map_err(|_| ApiError::InvalidToken("Invalid user ID in token".to_string()))
    }
}

/// MFA-pending JWT claims.
///
/// These claims use a **distinct audience** (`turerp-mfa`) so that the main
/// authentication middleware — which validates `aud: "turerp-api"` — rejects
/// them. An MFA-pending token can only be decoded via
/// [`JwtService::decode_mfa_token`], which is called exclusively by the MFA
/// verification endpoint. This prevents the class of bug where a stolen
/// MFA-pending token is accepted as a full access token (see issue #318).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MfaAuthClaims {
    pub sub: String,
    pub tenant_id: i64,
    pub username: String,
    pub exp: i64,
    pub iat: i64,
    pub aud: String,
    pub iss: String,
}

impl MfaAuthClaims {
    pub fn new(user_id: i64, tenant_id: i64, username: String, expires_in: i64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            sub: user_id.to_string(),
            tenant_id,
            username,
            exp: now + expires_in,
            iat: now,
            aud: "turerp-mfa".to_string(),
            iss: "turerp-auth".to_string(),
        }
    }

    pub fn user_id(&self) -> Result<i64, ApiError> {
        self.sub
            .parse()
            .map_err(|_| ApiError::InvalidToken("Invalid user ID in MFA token".to_string()))
    }
}

/// Portal-specific JWT claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortalAuthClaims {
    pub sub: String,
    pub tenant_id: i64,
    pub cari_id: i64,
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    pub aud: String,
    pub iss: String,
}

/// Vendor-specific JWT claims
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VendorAuthClaims {
    pub sub: String,
    pub tenant_id: i64,
    pub cari_id: i64,
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    pub aud: String,
    pub iss: String,
}

impl PortalAuthClaims {
    pub fn new(
        portal_user_id: i64,
        tenant_id: i64,
        cari_id: i64,
        email: String,
        expires_in: i64,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            sub: portal_user_id.to_string(),
            tenant_id,
            cari_id,
            email,
            role: "portal".to_string(),
            exp: now + expires_in,
            iat: now,
            aud: "turerp-portal".to_string(),
            iss: "turerp-auth".to_string(),
        }
    }

    pub fn portal_user_id(&self) -> Result<i64, ApiError> {
        self.sub
            .parse()
            .map_err(|_| ApiError::InvalidToken("Invalid portal user ID in token".to_string()))
    }
}

impl VendorAuthClaims {
    pub fn new(
        vendor_user_id: i64,
        tenant_id: i64,
        cari_id: i64,
        email: String,
        expires_in: i64,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            sub: vendor_user_id.to_string(),
            tenant_id,
            cari_id,
            email,
            role: "vendor".to_string(),
            exp: now + expires_in,
            iat: now,
            aud: "turerp-vendor".to_string(),
            iss: "turerp-auth".to_string(),
        }
    }

    pub fn vendor_user_id(&self) -> Result<i64, ApiError> {
        self.sub
            .parse()
            .map_err(|_| ApiError::InvalidToken("Invalid vendor user ID in token".to_string()))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// JWT service
#[derive(Clone)]
pub struct JwtService {
    secret: String,
    access_token_expiration: i64,
    refresh_token_expiration: i64,
    algorithm: Algorithm,
}

impl JwtService {
    #[must_use]
    pub fn new(
        secret: String,
        access_token_expiration: i64,
        refresh_token_expiration: i64,
    ) -> Self {
        Self {
            secret,
            access_token_expiration,
            refresh_token_expiration,
            algorithm: Algorithm::HS256,
        }
    }

    /// Generate access and refresh tokens
    #[must_use = "The generated tokens should be returned to the user"]
    pub fn generate_tokens(
        &self,
        user_id: i64,
        tenant_id: i64,
        username: String,
        role: Role,
    ) -> Result<TokenPair, ApiError> {
        let access_claims = AuthClaims::new(
            user_id,
            tenant_id,
            username.clone(),
            role,
            self.access_token_expiration,
        );

        let refresh_claims = AuthClaims::new(
            user_id,
            tenant_id,
            username,
            role,
            self.refresh_token_expiration,
        );

        let access_token = self.encode_token(&access_claims)?;
        let refresh_token = self.encode_token(&refresh_claims)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_expiration,
        })
    }

    /// Generate a single token
    pub fn encode_token(&self, claims: &AuthClaims) -> Result<String, ApiError> {
        encode(
            &Header::new(self.algorithm),
            claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| ApiError::Internal(format!("Failed to encode token: {}", e)))
    }

    /// Decode and validate a token
    #[must_use = "The decoded claims should be used for authentication"]
    pub fn decode_token(&self, token: &str) -> Result<AuthClaims, ApiError> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_audience(&["turerp-api"]);
        validation.set_issuer(&["turerp-auth"]);

        let token_data: TokenData<AuthClaims> = decode(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::TokenExpired,
            _ => ApiError::InvalidToken(e.to_string()),
        })?;

        Ok(token_data.claims)
    }

    /// Refresh tokens using a refresh token
    #[must_use = "The refreshed tokens should be returned to the user"]
    pub fn refresh_tokens(&self, refresh_token: &str) -> Result<TokenPair, ApiError> {
        let claims = self.decode_token(refresh_token)?;

        let user_id: i64 = claims
            .sub
            .parse()
            .map_err(|_| ApiError::InvalidToken("Invalid user ID in token".to_string()))?;

        let role: Role = claims
            .role
            .parse()
            .map_err(|_| ApiError::InvalidToken("Invalid role in token".to_string()))?;

        self.generate_tokens(user_id, claims.tenant_id, claims.username, role)
    }

    /// Get expiration time in seconds
    #[must_use = "The expiration time should be used for client-side token management"]
    pub fn access_token_expiration(&self) -> i64 {
        self.access_token_expiration
    }

    /// Encode an MFA-pending token (audience `turerp-mfa`).
    ///
    /// This token is **not** accepted by the main authentication middleware
    /// (which validates `aud: "turerp-api"`). It can only be decoded via
    /// [`Self::decode_mfa_token`], used by the MFA verification endpoint.
    pub fn encode_mfa_token(&self, claims: &MfaAuthClaims) -> Result<String, ApiError> {
        encode(
            &Header::new(self.algorithm),
            claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| ApiError::Internal(format!("Failed to encode MFA token: {}", e)))
    }

    /// Decode and validate an MFA-pending token (audience `turerp-mfa`).
    pub fn decode_mfa_token(&self, token: &str) -> Result<MfaAuthClaims, ApiError> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_audience(&["turerp-mfa"]);
        validation.set_issuer(&["turerp-auth"]);

        let token_data: TokenData<MfaAuthClaims> = decode(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::TokenExpired,
            _ => ApiError::InvalidToken(e.to_string()),
        })?;

        Ok(token_data.claims)
    }

    /// Encode a portal-specific token
    pub fn encode_portal_token(&self, claims: &PortalAuthClaims) -> Result<String, ApiError> {
        encode(
            &Header::new(self.algorithm),
            claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| ApiError::Internal(format!("Failed to encode portal token: {}", e)))
    }

    /// Decode and validate a portal-specific token
    pub fn decode_portal_token(&self, token: &str) -> Result<PortalAuthClaims, ApiError> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_audience(&["turerp-portal"]);
        validation.set_issuer(&["turerp-auth"]);

        let token_data: TokenData<PortalAuthClaims> = decode(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::TokenExpired,
            _ => ApiError::InvalidToken(e.to_string()),
        })?;

        Ok(token_data.claims)
    }

    /// Encode a vendor-specific token
    pub fn encode_vendor_token(&self, claims: &VendorAuthClaims) -> Result<String, ApiError> {
        encode(
            &Header::new(self.algorithm),
            claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| ApiError::Internal(format!("Failed to encode vendor token: {}", e)))
    }

    /// Decode and validate a vendor-specific token
    pub fn decode_vendor_token(&self, token: &str) -> Result<VendorAuthClaims, ApiError> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_audience(&["turerp-vendor"]);
        validation.set_issuer(&["turerp-auth"]);

        let token_data: TokenData<VendorAuthClaims> = decode(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => ApiError::TokenExpired,
            _ => ApiError::InvalidToken(e.to_string()),
        })?;

        Ok(token_data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_service() -> JwtService {
        JwtService::new("test-secret-key-for-testing-only".to_string(), 3600, 604800)
    }

    #[test]
    fn test_generate_tokens() {
        let service = create_service();
        let result = service.generate_tokens(1, 1, "testuser".to_string(), Role::Admin);

        assert!(result.is_ok());
        let tokens = result.unwrap();
        assert!(!tokens.access_token.is_empty());
        assert!(!tokens.refresh_token.is_empty());
        assert_eq!(tokens.token_type, "Bearer");
    }

    #[test]
    fn test_decode_token() {
        let service = create_service();
        let tokens = service
            .generate_tokens(1, 1, "testuser".to_string(), Role::User)
            .unwrap();

        let claims = service.decode_token(&tokens.access_token).unwrap();
        assert_eq!(claims.sub, "1");
        assert_eq!(claims.tenant_id, 1);
        assert_eq!(claims.username, "testuser");
    }

    #[test]
    fn test_refresh_tokens() {
        let service = create_service();
        let tokens = service
            .generate_tokens(1, 1, "testuser".to_string(), Role::User)
            .unwrap();

        let new_tokens = service.refresh_tokens(&tokens.refresh_token).unwrap();
        assert!(!new_tokens.access_token.is_empty());
        assert!(!new_tokens.refresh_token.is_empty());
    }

    #[test]
    fn test_invalid_token() {
        let service = create_service();
        let result = service.decode_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_mfa_token_rejected_by_decode_token() {
        // Regression test for issue #318: an MFA-pending token must NOT be
        // accepted by decode_token (the path used by JwtAuthMiddleware).
        // decode_token validates audience "turerp-api"; the MFA token uses
        // "turerp-mfa", so it must be rejected.
        let service = create_service();
        let mfa_claims = crate::utils::jwt::MfaAuthClaims::new(1, 1, "testuser".to_string(), 300);
        let mfa_token = service.encode_mfa_token(&mfa_claims).unwrap();

        // The main auth path must reject it
        assert!(service.decode_token(&mfa_token).is_err());

        // The MFA-specific path must accept it
        let decoded = service.decode_mfa_token(&mfa_token).unwrap();
        assert_eq!(decoded.sub, "1");
        assert_eq!(decoded.aud, "turerp-mfa");
    }
}
