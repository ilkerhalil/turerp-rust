//! IP whitelist domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// IP whitelist entry for tenant-scoped access control
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IpWhitelistEntry {
    pub id: i64,
    pub tenant_id: i64,
    pub ip_address: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Request to create a new IP whitelist entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateIpWhitelistEntry {
    pub tenant_id: i64,
    pub ip_address: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_active")]
    pub is_active: bool,
}

fn default_active() -> bool {
    true
}

impl CreateIpWhitelistEntry {
    /// Validate the create request
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.ip_address.trim().is_empty() {
            errors.push("IP address is required".to_string());
        } else if !is_valid_ip_or_cidr(&self.ip_address) {
            errors.push(format!(
                "'{}' is not a valid IP address or CIDR range",
                self.ip_address
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Request to update an existing IP whitelist entry
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateIpWhitelistEntry {
    #[serde(default)]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub description: Option<Option<String>>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Response payload for IP whitelist operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IpWhitelistEntryResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub ip_address: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<IpWhitelistEntry> for IpWhitelistEntryResponse {
    fn from(entry: IpWhitelistEntry) -> Self {
        Self {
            id: entry.id,
            tenant_id: entry.tenant_id,
            ip_address: entry.ip_address,
            description: entry.description,
            is_active: entry.is_active,
            created_at: entry.created_at,
        }
    }
}

/// Result of checking an IP against a tenant's whitelist
#[derive(Debug, Clone)]
pub struct IpWhitelistCheckResult {
    pub allowed: bool,
    pub matched_entry: Option<IpWhitelistEntry>,
}

/// Validate that a string is a valid IP address or CIDR notation
pub fn is_valid_ip_or_cidr(s: &str) -> bool {
    if s.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    if let Ok(net) = s.parse::<ipnetwork::IpNetwork>() {
        return net.prefix() > 0;
    }
    false
}

/// Check if a client IP matches a whitelist entry (supports CIDR)
pub fn ip_matches_entry(client_ip: &std::net::IpAddr, entry_ip: &str) -> bool {
    if let Ok(entry_addr) = entry_ip.parse::<std::net::IpAddr>() {
        return client_ip == &entry_addr;
    }
    if let Ok(network) = entry_ip.parse::<ipnetwork::IpNetwork>() {
        return network.contains(*client_ip);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_ip_or_cidr() {
        assert!(is_valid_ip_or_cidr("192.168.1.1"));
        assert!(is_valid_ip_or_cidr("10.0.0.0/8"));
        assert!(is_valid_ip_or_cidr("192.168.1.0/24"));
        assert!(is_valid_ip_or_cidr("::1"));
        assert!(is_valid_ip_or_cidr("2001:db8::/32"));
        assert!(!is_valid_ip_or_cidr("not-an-ip"));
        assert!(!is_valid_ip_or_cidr(""));
        assert!(!is_valid_ip_or_cidr("192.168.1.1/33"));
    }

    #[test]
    fn test_ip_matches_entry_exact() {
        let client: std::net::IpAddr = "192.168.1.1".parse().unwrap();
        assert!(ip_matches_entry(&client, "192.168.1.1"));
        assert!(!ip_matches_entry(&client, "192.168.1.2"));
    }

    #[test]
    fn test_ip_matches_entry_cidr() {
        let client1: std::net::IpAddr = "192.168.1.1".parse().unwrap();
        let client2: std::net::IpAddr = "192.168.1.255".parse().unwrap();
        let client3: std::net::IpAddr = "10.0.0.1".parse().unwrap();

        assert!(ip_matches_entry(&client1, "192.168.1.0/24"));
        assert!(ip_matches_entry(&client2, "192.168.1.0/24"));
        assert!(!ip_matches_entry(&client3, "192.168.1.0/24"));
    }

    #[test]
    fn test_ip_matches_ipv6_cidr() {
        let client: std::net::IpAddr = "2001:db8::1".parse().unwrap();
        assert!(ip_matches_entry(&client, "2001:db8::/32"));
        assert!(!ip_matches_entry(&client, "2001:db9::/32"));
    }

    #[test]
    fn test_create_validation() {
        let valid = CreateIpWhitelistEntry {
            tenant_id: 1,
            ip_address: "192.168.1.0/24".to_string(),
            description: Some("Office network".to_string()),
            is_active: true,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateIpWhitelistEntry {
            tenant_id: 1,
            ip_address: "not-an-ip".to_string(),
            description: None,
            is_active: true,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_response_from_entry() {
        let entry = IpWhitelistEntry {
            id: 1,
            tenant_id: 2,
            ip_address: "10.0.0.0/8".to_string(),
            description: Some("VPN".to_string()),
            is_active: true,
            created_at: Utc::now(),
        };
        let resp = IpWhitelistEntryResponse::from(entry.clone());
        assert_eq!(resp.id, 1);
        assert_eq!(resp.tenant_id, 2);
        assert_eq!(resp.ip_address, "10.0.0.0/8");
        assert_eq!(resp.description, Some("VPN".to_string()));
        assert!(resp.is_active);
    }
}
