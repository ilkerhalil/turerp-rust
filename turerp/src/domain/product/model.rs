//! Product domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Product entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub unit_id: Option<i64>,
    pub barcode: Option<String>,
    pub purchase_price: f64,
    pub sale_price: f64,
    pub tax_rate: f64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Product category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// Unit of measure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub is_integer: bool,
    pub created_at: DateTime<Utc>,
}

/// Product variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductVariant {
    pub id: i64,
    pub product_id: i64,
    pub name: String,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price_modifier: f64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Create product request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProduct {
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub unit_id: Option<i64>,
    pub barcode: Option<String>,
    pub purchase_price: f64,
    pub sale_price: f64,
    pub tax_rate: f64,
}

impl CreateProduct {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.code.trim().is_empty() {
            errors.push("Product code is required".to_string());
        }
        if self.name.trim().is_empty() {
            errors.push("Product name is required".to_string());
        }
        if self.purchase_price < 0.0 {
            errors.push("Purchase price cannot be negative".to_string());
        }
        if self.sale_price < 0.0 {
            errors.push("Sale price cannot be negative".to_string());
        }
        if self.tax_rate < 0.0 || self.tax_rate > 100.0 {
            errors.push("Tax rate must be between 0 and 100".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update product request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateProduct {
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub unit_id: Option<i64>,
    pub barcode: Option<String>,
    pub purchase_price: Option<f64>,
    pub sale_price: Option<f64>,
    pub tax_rate: Option<f64>,
    pub is_active: Option<bool>,
}

/// Create category request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCategory {
    pub tenant_id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
}

impl CreateCategory {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Category name is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create unit request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUnit {
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub is_integer: bool,
}

impl CreateUnit {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.code.trim().is_empty() {
            errors.push("Unit code is required".to_string());
        }
        if self.name.trim().is_empty() {
            errors.push("Unit name is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Product response (without tenant_id for external API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub unit_id: Option<i64>,
    pub barcode: Option<String>,
    pub purchase_price: f64,
    pub sale_price: f64,
    pub tax_rate: f64,
    pub is_active: bool,
}

impl From<Product> for ProductResponse {
    fn from(p: Product) -> Self {
        Self {
            id: p.id,
            code: p.code,
            name: p.name,
            description: p.description,
            category_id: p.category_id,
            unit_id: p.unit_id,
            barcode: p.barcode,
            purchase_price: p.purchase_price,
            sale_price: p.sale_price,
            tax_rate: p.tax_rate,
            is_active: p.is_active,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_product_validation() {
        let valid = CreateProduct {
            tenant_id: 1,
            code: "P001".to_string(),
            name: "Test Product".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: 100.0,
            sale_price: 150.0,
            tax_rate: 18.0,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateProduct {
            tenant_id: 1,
            code: "".to_string(),
            name: "".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: -10.0,
            sale_price: 150.0,
            tax_rate: 18.0,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_category_validation() {
        let valid = CreateCategory {
            tenant_id: 1,
            name: "Electronics".to_string(),
            parent_id: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateCategory {
            tenant_id: 1,
            name: "".to_string(),
            parent_id: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_unit_validation() {
        let valid = CreateUnit {
            tenant_id: 1,
            code: "PCS".to_string(),
            name: "Piece".to_string(),
            is_integer: true,
        };
        assert!(valid.validate().is_ok());
    }
}
