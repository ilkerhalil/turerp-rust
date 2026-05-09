//! Company domain model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Company entity (sub-tenant)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Company {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub tax_number: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub currency: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for Company {
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

impl Company {
    pub fn new(id: i64, tenant_id: i64, code: String, name: String) -> Self {
        Self {
            id,
            tenant_id,
            code,
            name,
            tax_number: None,
            address: None,
            city: None,
            country: None,
            currency: "TRY".to_string(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        }
    }
}

/// Data for creating a new company
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateCompany {
    #[validate(length(min = 1, max = 50))]
    pub code: String,
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    #[validate(length(min = 10, max = 20))]
    #[serde(default)]
    pub tax_number: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default = "default_company_currency")]
    pub currency: String,
    pub tenant_id: i64,
}

fn default_company_currency() -> String {
    "TRY".to_string()
}

/// Data for updating a company
#[derive(Debug, Clone, Deserialize, Serialize, Default, Validate, ToSchema)]
pub struct UpdateCompany {
    #[validate(length(min = 1, max = 50))]
    #[serde(default)]
    pub code: Option<String>,
    #[validate(length(min = 1, max = 200))]
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tax_number: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Company response for API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompanyResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub tax_number: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub currency: String,
    pub is_active: bool,
    pub tenant_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Company> for CompanyResponse {
    fn from(c: Company) -> Self {
        Self {
            id: c.id,
            code: c.code,
            name: c.name,
            tax_number: c.tax_number,
            address: c.address,
            city: c.city,
            country: c.country,
            currency: c.currency,
            is_active: c.is_active,
            tenant_id: c.tenant_id,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_company_new() {
        let c = Company::new(1, 1, "HQ".into(), "Headquarters".into());
        assert_eq!(c.code, "HQ");
        assert_eq!(c.name, "Headquarters");
        assert_eq!(c.currency, "TRY");
        assert!(c.is_active);
    }

    #[test]
    fn test_create_company_validation() {
        let create = CreateCompany {
            code: "HQ".to_string(),
            name: "Headquarters".to_string(),
            tax_number: Some("1234567890".to_string()),
            address: None,
            city: None,
            country: None,
            currency: "TRY".to_string(),
            tenant_id: 1,
        };
        assert!(create.validate().is_ok());
    }

    #[test]
    fn test_company_response_from() {
        let c = Company::new(1, 1, "HQ".into(), "Headquarters".into());
        let resp: CompanyResponse = c.into();
        assert_eq!(resp.code, "HQ");
    }
}
