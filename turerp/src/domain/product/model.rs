//! Product domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::impl_soft_deletable;

/// Product entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Product {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub unit_id: Option<i64>,
    pub barcode: Option<String>,
    pub purchase_price: Decimal,
    pub sale_price: Decimal,
    pub tax_rate: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(Product);

/// Product category
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Category {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(Category);

/// Unit of measure
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Unit {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub is_integer: bool,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(Unit);

/// Product variant
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductVariant {
    pub id: i64,
    pub product_id: i64,
    pub name: String,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price_modifier: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(ProductVariant);

/// Create product variant request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateProductVariant {
    pub product_id: i64,
    pub name: String,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price_modifier: Decimal,
}

impl CreateProductVariant {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.name.trim().is_empty() {
            errors.push("Variant name is required".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update product variant request
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateProductVariant {
    pub name: Option<String>,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price_modifier: Option<Decimal>,
    pub is_active: Option<bool>,
}

/// Product variant response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductVariantResponse {
    pub id: i64,
    pub product_id: i64,
    pub name: String,
    pub sku: Option<String>,
    pub barcode: Option<String>,
    pub price_modifier: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<ProductVariant> for ProductVariantResponse {
    fn from(v: ProductVariant) -> Self {
        Self {
            id: v.id,
            product_id: v.product_id,
            name: v.name,
            sku: v.sku,
            barcode: v.barcode,
            price_modifier: v.price_modifier,
            is_active: v.is_active,
            created_at: v.created_at,
        }
    }
}

/// Create product request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateProduct {
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub unit_id: Option<i64>,
    pub barcode: Option<String>,
    pub purchase_price: Decimal,
    pub sale_price: Decimal,
    pub tax_rate: Decimal,
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
        if self.purchase_price < Decimal::ZERO {
            errors.push("Purchase price cannot be negative".to_string());
        }
        if self.sale_price < Decimal::ZERO {
            errors.push("Sale price cannot be negative".to_string());
        }
        if self.tax_rate < Decimal::ZERO || self.tax_rate > Decimal::ONE_HUNDRED {
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
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateProduct {
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub unit_id: Option<i64>,
    pub barcode: Option<String>,
    pub purchase_price: Option<Decimal>,
    pub sale_price: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub is_active: Option<bool>,
}

/// Create category request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

/// Update category request
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateCategory {
    pub name: Option<String>,
    pub parent_id: Option<i64>,
}

/// Create unit request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

/// Update unit request
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateUnit {
    pub code: Option<String>,
    pub name: Option<String>,
    pub is_integer: Option<bool>,
}

/// Product response (without tenant_id/deleted fields for external API)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProductResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub unit_id: Option<i64>,
    pub barcode: Option<String>,
    pub purchase_price: Decimal,
    pub sale_price: Decimal,
    pub tax_rate: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
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
            created_at: p.created_at,
        }
    }
}

/// Category response (without tenant_id/deleted fields for external API)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CategoryResponse {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl From<Category> for CategoryResponse {
    fn from(c: Category) -> Self {
        Self {
            id: c.id,
            name: c.name,
            parent_id: c.parent_id,
            created_at: c.created_at,
        }
    }
}

/// Unit response (without tenant_id/deleted fields for external API)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UnitResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub is_integer: bool,
    pub created_at: DateTime<Utc>,
}

impl From<Unit> for UnitResponse {
    fn from(u: Unit) -> Self {
        Self {
            id: u.id,
            code: u.code,
            name: u.name,
            is_integer: u.is_integer,
            created_at: u.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::SoftDeletable;
    use rust_decimal_macros::dec;

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
            purchase_price: dec!(100.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
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
            purchase_price: dec!(-10.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
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
