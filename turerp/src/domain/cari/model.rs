//! Cari (Customer/Vendor) domain model

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Cari account type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum CariType {
    #[default]
    Customer,
    Vendor,
    Both,
}

impl std::fmt::Display for CariType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CariType::Customer => write!(f, "customer"),
            CariType::Vendor => write!(f, "vendor"),
            CariType::Both => write!(f, "both"),
        }
    }
}

impl std::str::FromStr for CariType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "customer" => Ok(CariType::Customer),
            "vendor" => Ok(CariType::Vendor),
            "both" => Ok(CariType::Both),
            _ => Err(format!("Invalid cari type: {}", s)),
        }
    }
}

/// Cari status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum CariStatus {
    #[default]
    Active,
    Passive,
    Blocked,
}

impl std::fmt::Display for CariStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CariStatus::Active => write!(f, "active"),
            CariStatus::Passive => write!(f, "passive"),
            CariStatus::Blocked => write!(f, "blocked"),
        }
    }
}

impl std::str::FromStr for CariStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(CariStatus::Active),
            "passive" => Ok(CariStatus::Passive),
            "blocked" => Ok(CariStatus::Blocked),
            _ => Err(format!("Invalid cari status: {}", s)),
        }
    }
}

/// Cari entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Cari {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub cari_type: CariType,
    pub tax_number: Option<String>,
    pub tax_office: Option<String>,
    pub identity_number: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub credit_limit: Decimal,
    pub current_balance: Decimal,
    pub default_currency: String,
    pub status: CariStatus,
    pub tenant_id: i64,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for Cari {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

impl Cari {
    /// Create a new cari (for testing/in-memory)
    pub fn new(
        id: i64,
        code: String,
        name: String,
        cari_type: CariType,
        tenant_id: i64,
        created_by: i64,
    ) -> Self {
        Self {
            id,
            code,
            name,
            cari_type,
            tax_number: None,
            tax_office: None,
            identity_number: None,
            email: None,
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: Decimal::ZERO,
            current_balance: Decimal::ZERO,
            default_currency: "TRY".to_string(),
            status: CariStatus::Active,
            tenant_id,
            created_by,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        }
    }
}

/// Data for creating a new cari
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateCari {
    #[validate(length(min = 1, max = 50))]
    pub code: String,

    #[validate(length(min = 1, max = 200))]
    pub name: String,

    #[serde(default)]
    pub cari_type: CariType,

    #[validate(length(min = 10, max = 20))]
    #[serde(default)]
    pub tax_number: Option<String>,

    #[validate(length(max = 100))]
    #[serde(default)]
    pub tax_office: Option<String>,

    #[validate(length(min = 11, max = 11))]
    #[serde(default)]
    pub identity_number: Option<String>,

    #[validate(email)]
    #[serde(default)]
    pub email: Option<String>,

    #[validate(length(min = 10, max = 20))]
    #[serde(default)]
    pub phone: Option<String>,

    #[validate(length(max = 500))]
    #[serde(default)]
    pub address: Option<String>,

    #[validate(length(max = 100))]
    #[serde(default)]
    pub city: Option<String>,

    #[validate(length(max = 100))]
    #[serde(default)]
    pub country: Option<String>,

    #[validate(length(max = 20))]
    #[serde(default)]
    pub postal_code: Option<String>,

    #[serde(default = "default_credit_limit")]
    pub credit_limit: Decimal,

    #[serde(default = "default_cari_currency")]
    pub default_currency: String,

    pub tenant_id: i64,

    pub created_by: i64,
}

fn default_cari_currency() -> String {
    "TRY".to_string()
}

fn default_credit_limit() -> Decimal {
    Decimal::ZERO
}

/// Data for updating an existing cari
#[derive(Debug, Clone, Deserialize, Serialize, Default, Validate, ToSchema)]
pub struct UpdateCari {
    #[validate(length(min = 1, max = 50))]
    #[serde(default)]
    pub code: Option<String>,

    #[validate(length(min = 1, max = 200))]
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub cari_type: Option<CariType>,

    #[validate(length(min = 10, max = 20))]
    #[serde(default)]
    pub tax_number: Option<String>,

    #[validate(length(max = 100))]
    #[serde(default)]
    pub tax_office: Option<String>,

    #[validate(length(min = 11, max = 11))]
    #[serde(default)]
    pub identity_number: Option<String>,

    #[validate(email)]
    #[serde(default)]
    pub email: Option<String>,

    #[validate(length(min = 10, max = 20))]
    #[serde(default)]
    pub phone: Option<String>,

    #[validate(length(max = 500))]
    #[serde(default)]
    pub address: Option<String>,

    #[validate(length(max = 100))]
    #[serde(default)]
    pub city: Option<String>,

    #[validate(length(max = 100))]
    #[serde(default)]
    pub country: Option<String>,

    #[validate(length(max = 20))]
    #[serde(default)]
    pub postal_code: Option<String>,

    #[serde(default)]
    pub credit_limit: Option<Decimal>,

    #[serde(default)]
    pub status: Option<CariStatus>,

    #[serde(default)]
    pub default_currency: Option<String>,
}

/// Cari response (without sensitive internal data)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CariResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub cari_type: CariType,
    pub tax_number: Option<String>,
    pub tax_office: Option<String>,
    pub identity_number: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub credit_limit: Decimal,
    pub current_balance: Decimal,
    pub default_currency: String,
    pub status: CariStatus,
    pub tenant_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Cari> for CariResponse {
    fn from(cari: Cari) -> Self {
        Self {
            id: cari.id,
            code: cari.code,
            name: cari.name,
            cari_type: cari.cari_type,
            tax_number: cari.tax_number,
            tax_office: cari.tax_office,
            identity_number: cari.identity_number,
            email: cari.email,
            phone: cari.phone,
            address: cari.address,
            city: cari.city,
            country: cari.country,
            postal_code: cari.postal_code,
            credit_limit: cari.credit_limit,
            current_balance: cari.current_balance,
            default_currency: cari.default_currency,
            status: cari.status,
            tenant_id: cari.tenant_id,
            created_at: cari.created_at,
            updated_at: cari.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::SoftDeletable;
    use rust_decimal_macros::dec;
    use std::str::FromStr;

    #[test]
    fn test_cari_type_display() {
        assert_eq!(CariType::Customer.to_string(), "customer");
        assert_eq!(CariType::Vendor.to_string(), "vendor");
        assert_eq!(CariType::Both.to_string(), "both");
    }

    #[test]
    fn test_cari_type_from_str() {
        assert_eq!(CariType::from_str("customer").unwrap(), CariType::Customer);
        assert_eq!(CariType::from_str("VENDOR").unwrap(), CariType::Vendor);
        assert!(CariType::from_str("invalid").is_err());
    }

    #[test]
    fn test_cari_status_default() {
        let cari = Cari::new(1, "C001".into(), "Test".into(), CariType::Customer, 1, 1);
        assert_eq!(cari.status, CariStatus::Active);
        assert_eq!(cari.credit_limit, Decimal::ZERO);
    }

    #[test]
    fn test_cari_response_from_cari() {
        let cari = Cari::new(1, "C001".into(), "Test".into(), CariType::Customer, 1, 1);

        let response: CariResponse = cari.into();
        assert_eq!(response.code, "C001");
        assert_eq!(response.name, "Test");
        assert_eq!(response.cari_type, CariType::Customer);
    }

    #[test]
    fn test_create_cari_validation() {
        let create = CreateCari {
            code: "C001".to_string(),
            name: "Test Customer".to_string(),
            cari_type: CariType::Customer,
            tax_number: Some("1234567890".to_string()),
            tax_office: None,
            identity_number: None,
            email: Some("test@example.com".to_string()),
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: dec!(1000),
            default_currency: "TRY".to_string(),
            tenant_id: 1,
            created_by: 1,
        };

        assert!(create.validate().is_ok());
    }

    #[test]
    fn test_create_cari_validation_fails() {
        let create = CreateCari {
            code: "".to_string(), // Invalid: empty
            name: "Test".to_string(),
            cari_type: CariType::Customer,
            tax_number: None,
            tax_office: None,
            identity_number: None,
            email: Some("invalid-email".to_string()), // Invalid
            phone: None,
            address: None,
            city: None,
            country: None,
            postal_code: None,
            credit_limit: Decimal::ZERO,
            default_currency: "TRY".to_string(),
            tenant_id: 1,
            created_by: 1,
        };

        assert!(create.validate().is_err());
    }
}
