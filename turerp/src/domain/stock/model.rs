//! Stock domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Warehouse entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Warehouse {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub address: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for Warehouse {
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

/// Warehouse response (without deleted_at/deleted_by)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub address: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<Warehouse> for WarehouseResponse {
    fn from(w: Warehouse) -> Self {
        Self {
            id: w.id,
            tenant_id: w.tenant_id,
            code: w.code,
            name: w.name,
            address: w.address,
            is_active: w.is_active,
            created_at: w.created_at,
        }
    }
}

/// Stock level for a product in a warehouse
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockLevel {
    pub id: i64,
    pub warehouse_id: i64,
    pub product_id: i64,
    pub quantity: Decimal,
    pub reserved_quantity: Decimal,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for StockLevel {
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

/// StockLevel response (without deleted_at/deleted_by)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockLevelResponse {
    pub id: i64,
    pub warehouse_id: i64,
    pub product_id: i64,
    pub quantity: Decimal,
    pub reserved_quantity: Decimal,
    pub updated_at: DateTime<Utc>,
}

impl From<StockLevel> for StockLevelResponse {
    fn from(l: StockLevel) -> Self {
        Self {
            id: l.id,
            warehouse_id: l.warehouse_id,
            product_id: l.product_id,
            quantity: l.quantity,
            reserved_quantity: l.reserved_quantity,
            updated_at: l.updated_at,
        }
    }
}

/// Stock movement types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for StockMovement {
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

/// StockMovement response (without deleted_at/deleted_by)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockMovementResponse {
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

impl From<StockMovement> for StockMovementResponse {
    fn from(m: StockMovement) -> Self {
        Self {
            id: m.id,
            warehouse_id: m.warehouse_id,
            product_id: m.product_id,
            movement_type: m.movement_type,
            quantity: m.quantity,
            reference_type: m.reference_type,
            reference_id: m.reference_id,
            notes: m.notes,
            created_at: m.created_at,
            created_by: m.created_by,
        }
    }
}

/// Stock valuation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockValuation {
    pub product_id: i64,
    pub warehouse_id: i64,
    pub total_quantity: Decimal,
    pub avg_cost: Decimal,
    pub total_value: Decimal,
}

/// Create warehouse request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StockSummary {
    pub product_id: i64,
    pub total_quantity: Decimal,
    pub reserved_quantity: Decimal,
    pub available_quantity: Decimal,
    pub warehouses: Vec<WarehouseStock>,
}

/// Stock per warehouse
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseStock {
    pub warehouse_id: i64,
    pub warehouse_name: String,
    pub quantity: Decimal,
    pub reserved_quantity: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::SoftDeletable;
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

    #[test]
    fn test_warehouse_soft_delete() {
        let mut warehouse = Warehouse {
            id: 1,
            tenant_id: 1,
            code: "WH001".to_string(),
            name: "Main".to_string(),
            address: None,
            is_active: true,
            created_at: Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };
        assert!(!warehouse.is_deleted());
        warehouse.mark_deleted(42);
        assert!(warehouse.is_deleted());
        assert_eq!(warehouse.deleted_by(), Some(42));
        warehouse.restore();
        assert!(!warehouse.is_deleted());
    }
}
