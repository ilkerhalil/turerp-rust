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
            exp: now + expires_in,
            iat: now,
            aud: "turerp-api".to_string(),
            iss: "turerp-auth".to_string(),
        }
    }
}

/// JWT token pair
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
    fn encode_token(&self, claims: &AuthClaims) -> Result<String, ApiError> {
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
}
