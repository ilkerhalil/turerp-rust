//! Audit log domain models
//!
//! Represents audit log entries that track all API requests for compliance
//! and security monitoring.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Audit log entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLog {
    /// Unique identifier
    pub id: i64,
    /// Tenant ID that the request belongs to
    pub tenant_id: i64,
    /// User ID that made the request
    pub user_id: i64,
    /// Username of the user that made the request
    pub username: String,
    /// HTTP method (e.g., GET, POST, PUT, DELETE)
    pub action: String,
    /// Request path
    pub path: String,
    /// HTTP response status code
    pub status_code: i16,
    /// Unique request identifier for tracing
    pub request_id: String,
    /// Client IP address
    pub ip_address: Option<String>,
    /// Client User-Agent header
    pub user_agent: Option<String>,
    /// Timestamp when the request was made
    pub created_at: DateTime<Utc>,
}

/// Query parameters for filtering and paginating audit logs
#[derive(Debug, Clone, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct AuditLogQueryParams {
    /// Filter by user ID
    pub user_id: Option<i64>,
    /// Filter by request path (substring match)
    pub path: Option<String>,
    /// Filter logs from this date onwards
    pub from_date: Option<DateTime<Utc>>,
    /// Filter logs up to this date
    pub to_date: Option<DateTime<Utc>>,
    /// Page number (1-based)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Number of items per page
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

impl Default for AuditLogQueryParams {
    fn default() -> Self {
        Self {
            user_id: None,
            path: None,
            from_date: None,
            to_date: None,
            page: default_page(),
            per_page: default_per_page(),
        }
    }
}

/// Create audit log request (used for inserting new audit log entries)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAuditLog {
    /// Tenant ID that the request belongs to
    pub tenant_id: i64,
    /// User ID that made the request
    pub user_id: i64,
    /// Username of the user that made the request
    pub username: String,
    /// HTTP method (e.g., GET, POST, PUT, DELETE)
    pub action: String,
    /// Request path
    pub path: String,
    /// HTTP response status code
    pub status_code: i16,
    /// Unique request identifier for tracing
    pub request_id: String,
    /// Client IP address
    pub ip_address: Option<String>,
    /// Client User-Agent header
    pub user_agent: Option<String>,
    /// Timestamp when the request was made
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_query_params_default() {
        let params = AuditLogQueryParams::default();
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 20);
        assert!(params.user_id.is_none());
        assert!(params.path.is_none());
        assert!(params.from_date.is_none());
        assert!(params.to_date.is_none());
    }

    #[test]
    fn test_create_audit_log_construction() {
        let now = Utc::now();
        let log = CreateAuditLog {
            tenant_id: 1,
            user_id: 42,
            username: "admin".to_string(),
            action: "POST".to_string(),
            path: "/api/v1/users".to_string(),
            status_code: 201,
            request_id: "req-123".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("curl/8.0".to_string()),
            created_at: now,
        };
        assert_eq!(log.tenant_id, 1);
        assert_eq!(log.user_id, 42);
        assert_eq!(log.action, "POST");
        assert_eq!(log.status_code, 201);
    }

    #[test]
    fn test_audit_log_serialization() {
        let now = Utc::now();
        let log = AuditLog {
            id: 1,
            tenant_id: 1,
            user_id: 42,
            username: "admin".to_string(),
            action: "GET".to_string(),
            path: "/api/v1/users".to_string(),
            status_code: 200,
            request_id: "req-456".to_string(),
            ip_address: Some("10.0.0.1".to_string()),
            user_agent: None,
            created_at: now,
        };

        let json = serde_json::to_string(&log).unwrap();
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"action\":\"GET\""));
    }
}
