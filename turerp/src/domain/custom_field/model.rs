//! Custom field definitions for dynamic module attributes

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Modules that support custom fields
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CustomFieldModule {
    Cari,
    Invoice,
    Stock,
    Sales,
    Purchase,
    Hr,
    Accounting,
    Project,
    Manufacturing,
    Crm,
    Asset,
    Product,
}

impl std::fmt::Display for CustomFieldModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cari => write!(f, "cari"),
            Self::Invoice => write!(f, "invoice"),
            Self::Stock => write!(f, "stock"),
            Self::Sales => write!(f, "sales"),
            Self::Purchase => write!(f, "purchase"),
            Self::Hr => write!(f, "hr"),
            Self::Accounting => write!(f, "accounting"),
            Self::Project => write!(f, "project"),
            Self::Manufacturing => write!(f, "manufacturing"),
            Self::Crm => write!(f, "crm"),
            Self::Asset => write!(f, "asset"),
            Self::Product => write!(f, "product"),
        }
    }
}

impl std::str::FromStr for CustomFieldModule {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cari" => Ok(Self::Cari),
            "invoice" => Ok(Self::Invoice),
            "stock" => Ok(Self::Stock),
            "sales" => Ok(Self::Sales),
            "purchase" => Ok(Self::Purchase),
            "hr" => Ok(Self::Hr),
            "accounting" => Ok(Self::Accounting),
            "project" => Ok(Self::Project),
            "manufacturing" => Ok(Self::Manufacturing),
            "crm" => Ok(Self::Crm),
            "asset" => Ok(Self::Asset),
            "product" => Ok(Self::Product),
            _ => Err(format!("Invalid custom field module: {}", s)),
        }
    }
}

/// Data types for custom field values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CustomFieldType {
    String,
    Number,
    Date,
    Boolean,
    Select,
}

impl std::fmt::Display for CustomFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String => write!(f, "string"),
            Self::Number => write!(f, "number"),
            Self::Date => write!(f, "date"),
            Self::Boolean => write!(f, "boolean"),
            Self::Select => write!(f, "select"),
        }
    }
}

impl std::str::FromStr for CustomFieldType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "string" => Ok(Self::String),
            "number" => Ok(Self::Number),
            "date" => Ok(Self::Date),
            "boolean" => Ok(Self::Boolean),
            "select" => Ok(Self::Select),
            _ => Err(format!("Invalid custom field type: {}", s)),
        }
    }
}

/// Custom field definition entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CustomFieldDefinition {
    pub id: i64,
    pub tenant_id: i64,
    pub module: CustomFieldModule,
    pub field_name: String,
    pub field_label: String,
    pub field_type: CustomFieldType,
    pub required: bool,
    pub options: Vec<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for CustomFieldDefinition {
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

/// DTO for creating a custom field definition
#[derive(Debug, Clone, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateCustomFieldDefinition {
    #[validate(length(min = 1, max = 50))]
    pub module: String,

    #[validate(length(min = 1, max = 100))]
    pub field_name: String,

    #[validate(length(min = 1, max = 200))]
    pub field_label: String,

    #[validate(length(min = 1, max = 20))]
    pub field_type: String,

    #[serde(default)]
    pub required: bool,

    #[serde(default)]
    pub options: Vec<String>,

    #[serde(default)]
    pub sort_order: i32,

    pub tenant_id: i64,
}

/// DTO for updating a custom field definition
#[derive(Debug, Clone, Deserialize, Serialize, Default, Validate, ToSchema)]
pub struct UpdateCustomFieldDefinition {
    #[validate(length(min = 1, max = 200))]
    #[serde(default)]
    pub field_label: Option<String>,

    #[serde(default)]
    pub required: Option<bool>,

    #[serde(default)]
    pub options: Option<Vec<String>>,

    #[serde(default)]
    pub sort_order: Option<i32>,

    #[serde(default)]
    pub is_active: Option<bool>,
}

/// Response DTO for custom field definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CustomFieldDefinitionResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub module: CustomFieldModule,
    pub field_name: String,
    pub field_label: String,
    pub field_type: CustomFieldType,
    pub required: bool,
    pub options: Vec<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<CustomFieldDefinition> for CustomFieldDefinitionResponse {
    fn from(def: CustomFieldDefinition) -> Self {
        Self {
            id: def.id,
            tenant_id: def.tenant_id,
            module: def.module,
            field_name: def.field_name,
            field_label: def.field_label,
            field_type: def.field_type,
            required: def.required,
            options: def.options,
            sort_order: def.sort_order,
            is_active: def.is_active,
            created_at: def.created_at,
            updated_at: def.updated_at,
        }
    }
}

/// Type alias for custom field values (JSONB)
pub type CustomFieldValues = serde_json::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::SoftDeletable;
    use std::str::FromStr;

    #[test]
    fn test_custom_field_module_display() {
        assert_eq!(CustomFieldModule::Cari.to_string(), "cari");
        assert_eq!(CustomFieldModule::Invoice.to_string(), "invoice");
    }

    #[test]
    fn test_custom_field_module_from_str() {
        assert_eq!(
            CustomFieldModule::from_str("cari").unwrap(),
            CustomFieldModule::Cari
        );
        assert_eq!(
            CustomFieldModule::from_str("HR").unwrap(),
            CustomFieldModule::Hr
        );
        assert!(CustomFieldModule::from_str("invalid").is_err());
    }

    #[test]
    fn test_custom_field_type_display() {
        assert_eq!(CustomFieldType::String.to_string(), "string");
        assert_eq!(CustomFieldType::Select.to_string(), "select");
    }

    #[test]
    fn test_custom_field_type_from_str() {
        assert_eq!(
            CustomFieldType::from_str("number").unwrap(),
            CustomFieldType::Number
        );
        assert!(CustomFieldType::from_str("invalid").is_err());
    }

    #[test]
    fn test_soft_delete() {
        let mut def = CustomFieldDefinition {
            id: 1,
            tenant_id: 1,
            module: CustomFieldModule::Cari,
            field_name: "tax_region".to_string(),
            field_label: "Tax Region".to_string(),
            field_type: CustomFieldType::String,
            required: false,
            options: vec![],
            sort_order: 0,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };
        assert!(!def.is_deleted());
        def.mark_deleted(42);
        assert!(def.is_deleted());
        assert_eq!(def.deleted_by(), Some(42));
        def.restore();
        assert!(!def.is_deleted());
    }

    #[test]
    fn test_response_from_definition() {
        let def = CustomFieldDefinition {
            id: 1,
            tenant_id: 1,
            module: CustomFieldModule::Cari,
            field_name: "industry".to_string(),
            field_label: "Industry Code".to_string(),
            field_type: CustomFieldType::Select,
            required: true,
            options: vec!["Tech".to_string(), "Finance".to_string()],
            sort_order: 1,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };
        let resp: CustomFieldDefinitionResponse = def.into();
        assert_eq!(resp.field_name, "industry");
        assert_eq!(resp.options.len(), 2);
    }
}
