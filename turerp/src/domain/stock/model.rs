//! Stock domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Warehouse entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warehouse {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub address: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Stock level for a product in a warehouse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockLevel {
    pub id: i64,
    pub warehouse_id: i64,
    pub product_id: i64,
    pub quantity: Decimal,
    pub reserved_quantity: Decimal,
    pub updated_at: DateTime<Utc>,
}

/// Stock movement types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MovementType {
    Purchase,      // Stock in from purchase order
    Sale,          // Stock out from sales order
    Return,        // Stock in from customer return
    Adjustment,    // Manual adjustment
    Transfer,      // Transfer between warehouses
    ProductionIn,  // Stock in from production
    ProductionOut, // Stock out for production
    Waste,         // Stock out due to waste/damage
}

impl std::fmt::Display for MovementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MovementType::Purchase => write!(f, "Purchase"),
            MovementType::Sale => write!(f, "Sale"),
            MovementType::Return => write!(f, "Return"),
            MovementType::Adjustment => write!(f, "Adjustment"),
            MovementType::Transfer => write!(f, "Transfer"),
            MovementType::ProductionIn => write!(f, "ProductionIn"),
            MovementType::ProductionOut => write!(f, "ProductionOut"),
            MovementType::Waste => write!(f, "Waste"),
        }
    }
}

impl std::str::FromStr for MovementType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Purchase" => Ok(MovementType::Purchase),
            "Sale" => Ok(MovementType::Sale),
            "Return" => Ok(MovementType::Return),
            "Adjustment" => Ok(MovementType::Adjustment),
            "Transfer" => Ok(MovementType::Transfer),
            "ProductionIn" => Ok(MovementType::ProductionIn),
            "ProductionOut" => Ok(MovementType::ProductionOut),
            "Waste" => Ok(MovementType::Waste),
            _ => Err(format!("Invalid movement type: {}", s)),
        }
    }
}

/// Stock movement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMovement {
    pub id: i64,
    pub warehouse_id: i64,
    pub product_id: i64,
    pub movement_type: MovementType,
    pub quantity: Decimal,
    pub reference_type: Option<String>,
    pub reference_id: Option<i64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: i64,
}

/// Stock valuation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockValuation {
    pub product_id: i64,
    pub warehouse_id: i64,
    pub total_quantity: Decimal,
    pub avg_cost: Decimal,
    pub total_value: Decimal,
}

/// Create warehouse request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWarehouse {
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub address: Option<String>,
}

impl CreateWarehouse {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.code.trim().is_empty() {
            errors.push("Warehouse code is required".to_string());
        }
        if self.name.trim().is_empty() {
            errors.push("Warehouse name is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create stock movement request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStockMovement {
    pub warehouse_id: i64,
    pub product_id: i64,
    pub movement_type: MovementType,
    pub quantity: Decimal,
    pub reference_type: Option<String>,
    pub reference_id: Option<i64>,
    pub notes: Option<String>,
    pub created_by: i64,
}

impl CreateStockMovement {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.quantity <= Decimal::ZERO {
            errors.push("Quantity must be positive".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Stock summary response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockSummary {
    pub product_id: i64,
    pub total_quantity: Decimal,
    pub reserved_quantity: Decimal,
    pub available_quantity: Decimal,
    pub warehouses: Vec<WarehouseStock>,
}

/// Stock per warehouse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseStock {
    pub warehouse_id: i64,
    pub warehouse_name: String,
    pub quantity: Decimal,
    pub reserved_quantity: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_create_warehouse_validation() {
        let valid = CreateWarehouse {
            tenant_id: 1,
            code: "WH001".to_string(),
            name: "Main Warehouse".to_string(),
            address: Some("Address".to_string()),
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateWarehouse {
            tenant_id: 1,
            code: "".to_string(),
            name: "".to_string(),
            address: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_stock_movement_validation() {
        let valid = CreateStockMovement {
            warehouse_id: 1,
            product_id: 1,
            movement_type: MovementType::Purchase,
            quantity: dec!(100),
            reference_type: Some("PO".to_string()),
            reference_id: Some(1),
            notes: None,
            created_by: 1,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateStockMovement {
            warehouse_id: 1,
            product_id: 1,
            movement_type: MovementType::Sale,
            quantity: dec!(-10),
            reference_type: None,
            reference_id: None,
            notes: None,
            created_by: 1,
        };
        assert!(invalid.validate().is_err());
    }
}
