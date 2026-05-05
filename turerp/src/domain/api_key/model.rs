//! API Key domain model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// API Key permission scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyScope {
    /// Full access to all resources
    All,
    /// Read-only access to cari (contacts)
    CariRead,
    /// Read-write access to cari
    CariWrite,
    /// Read-only access to invoices
    InvoiceRead,
    /// Read-write access to invoices
    InvoiceWrite,
    /// Read-only access to stock
    StockRead,
    /// Read-write access to stock
    StockWrite,
    /// Read-only access to sales
    SalesRead,
    /// Read-write access to sales
    SalesWrite,
    /// Read-only access to products
    ProductRead,
    /// Read-write access to products
    ProductWrite,
    /// Read-only access to reports
    ReportRead,
}

impl std::fmt::Display for ApiKeyScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiKeyScope::All => write!(f, "all"),
            ApiKeyScope::CariRead => write!(f, "cari:read"),
            ApiKeyScope::CariWrite => write!(f, "cari:write"),
            ApiKeyScope::InvoiceRead => write!(f, "invoice:read"),
            ApiKeyScope::InvoiceWrite => write!(f, "invoice:write"),
            ApiKeyScope::StockRead => write!(f, "stock:read"),
            ApiKeyScope::StockWrite => write!(f, "stock:write"),
            ApiKeyScope::SalesRead => write!(f, "sales:read"),
            ApiKeyScope::SalesWrite => write!(f, "sales:write"),
            ApiKeyScope::ProductRead => write!(f, "product:read"),
            ApiKeyScope::ProductWrite => write!(f, "product:write"),
            ApiKeyScope::ReportRead => write!(f, "report:read"),
        }
    }
}

impl std::str::FromStr for ApiKeyScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "all" => Ok(ApiKeyScope::All),
            "cari:read" => Ok(ApiKeyScope::CariRead),
            "cari:write" => Ok(ApiKeyScope::CariWrite),
            "invoice:read" => Ok(ApiKeyScope::InvoiceRead),
            "invoice:write" => Ok(ApiKeyScope::InvoiceWrite),
            "stock:read" => Ok(ApiKeyScope::StockRead),
            "stock:write" => Ok(ApiKeyScope::StockWrite),
            "sales:read" => Ok(ApiKeyScope::SalesRead),
            "sales:write" => Ok(ApiKeyScope::SalesWrite),
            "product:read" => Ok(ApiKeyScope::ProductRead),
            "product:write" => Ok(ApiKeyScope::ProductWrite),
            "report:read" => Ok(ApiKeyScope::ReportRead),
            _ => Err(format!("Invalid API key scope: {}", s)),
        }
    }
}

/// API Key entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: i64,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub tenant_id: i64,
    pub user_id: i64,
    pub scopes: Vec<ApiKeyScope>,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// API Key response (never exposes the full key)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyResponse {
    pub id: i64,
    pub name: String,
    pub key_prefix: String,
    pub tenant_id: i64,
    pub user_id: i64,
    pub scopes: Vec<ApiKeyScope>,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<ApiKey> for ApiKeyResponse {
    fn from(key: ApiKey) -> Self {
        ApiKeyResponse {
            id: key.id,
            name: key.name,
            key_prefix: key.key_prefix,
            tenant_id: key.tenant_id,
            user_id: key.user_id,
            scopes: key.scopes,
            is_active: key.is_active,
            expires_at: key.expires_at,
            last_used_at: key.last_used_at,
            created_at: key.created_at,
        }
    }
}

/// Create API Key request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateApiKey {
    pub name: String,
    pub tenant_id: i64,
    pub user_id: i64,
    pub scopes: Vec<ApiKeyScope>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Update API Key request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateApiKey {
    pub name: Option<String>,
    pub scopes: Option<Vec<ApiKeyScope>>,
    pub is_active: Option<bool>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

/// API Key creation result (includes the plain key only once)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyCreationResult {
    pub api_key: ApiKeyResponse,
    pub plain_key: String,
}

/// Validate API Key creation
impl CreateApiKey {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Name is required".to_string());
        }
        if self.name.len() > 255 {
            errors.push("Name must be at most 255 characters".to_string());
        }
        if self.scopes.is_empty() {
            errors.push("At least one scope is required".to_string());
        }
        if let Some(expires) = self.expires_at {
            if expires < Utc::now() {
                errors.push("Expiration date must be in the future".to_string());
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Generate a new API key string
pub fn generate_api_key() -> String {
    use rand::Rng;
    let rng = &mut rand::thread_rng();
    let prefix: String = (0..8).map(|_| rng.gen_range(b'a'..=b'z') as char).collect();
    let secret: String = (0..32)
        .map(|_| rng.gen_range(b'a'..=b'z') as char)
        .collect();
    format!("tuk_{}_{}", prefix, secret)
}

/// Extract the prefix (first 12 chars) from an API key
pub fn extract_prefix(key: &str) -> String {
    key.chars().take(12).collect()
}

/// Hash an API key for storage using SHA-256
pub fn hash_api_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_display() {
        assert_eq!(ApiKeyScope::All.to_string(), "all");
        assert_eq!(ApiKeyScope::CariRead.to_string(), "cari:read");
        assert_eq!(ApiKeyScope::InvoiceWrite.to_string(), "invoice:write");
    }

    #[test]
    fn test_scope_from_str() {
        assert_eq!("all".parse::<ApiKeyScope>(), Ok(ApiKeyScope::All));
        assert_eq!(
            "cari:read".parse::<ApiKeyScope>(),
            Ok(ApiKeyScope::CariRead)
        );
        assert!("invalid".parse::<ApiKeyScope>().is_err());
    }

    #[test]
    fn test_validate_create_api_key() {
        let create = CreateApiKey {
            name: "Test Key".to_string(),
            tenant_id: 1,
            user_id: 1,
            scopes: vec![ApiKeyScope::All],
            expires_at: None,
        };
        assert!(create.validate().is_ok());
    }

    #[test]
    fn test_validate_create_api_key_empty_name() {
        let create = CreateApiKey {
            name: "  ".to_string(),
            tenant_id: 1,
            user_id: 1,
            scopes: vec![ApiKeyScope::All],
            expires_at: None,
        };
        assert!(create.validate().is_err());
    }

    #[test]
    fn test_validate_create_api_key_empty_scopes() {
        let create = CreateApiKey {
            name: "Test Key".to_string(),
            tenant_id: 1,
            user_id: 1,
            scopes: vec![],
            expires_at: None,
        };
        assert!(create.validate().is_err());
    }

    #[test]
    fn test_generate_api_key() {
        let key = generate_api_key();
        assert!(key.starts_with("tuk_"));
        assert!(key.len() > 20);
    }

    #[test]
    fn test_hash_api_key_deterministic() {
        let key = "test-key-123";
        let hash1 = hash_api_key(key);
        let hash2 = hash_api_key(key);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_extract_prefix() {
        let key = "tuk_abcdefgh_abcdefghijklmnopqrstuvwxyz123456";
        let prefix = extract_prefix(key);
        assert_eq!(prefix, "tuk_abcdefgh");
        assert_eq!(prefix.len(), 12);
    }
}
