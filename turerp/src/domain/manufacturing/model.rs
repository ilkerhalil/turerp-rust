//! Manufacturing domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Work order status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum WorkOrderStatus {
    Draft,
    Scheduled,
    InProgress,
    OnHold,
    Completed,
    Cancelled,
}

/// Work order priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum WorkOrderPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Work order entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkOrder {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub product_id: i64,
    pub quantity: Decimal,
    pub bom_id: Option<i64>,
    pub routing_id: Option<i64>,
    pub status: WorkOrderStatus,
    pub priority: WorkOrderPriority,
    pub planned_start: Option<DateTime<Utc>>,
    pub planned_end: Option<DateTime<Utc>>,
    pub actual_start: Option<DateTime<Utc>>,
    pub actual_end: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for WorkOrder {
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

/// Work order operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkOrderOperation {
    pub id: i64,
    pub work_order_id: i64,
    pub operation_sequence: i32,
    pub operation_name: String,
    pub work_center_id: Option<i64>,
    pub planned_hours: Decimal,
    pub actual_hours: Decimal,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Work order material
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkOrderMaterial {
    pub id: i64,
    pub work_order_id: i64,
    pub product_id: i64,
    pub quantity_required: Decimal,
    pub quantity_issued: Decimal,
    pub is_issued: bool,
}

/// Create work order request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkOrder {
    pub tenant_id: i64,
    pub name: String,
    pub product_id: i64,
    pub quantity: Decimal,
    pub bom_id: Option<i64>,
    pub routing_id: Option<i64>,
    pub priority: WorkOrderPriority,
    pub planned_start: Option<DateTime<Utc>>,
    pub planned_end: Option<DateTime<Utc>>,
}

impl CreateWorkOrder {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Work order name is required".to_string());
        }
        if self.product_id <= 0 {
            errors.push("Product ID is required".to_string());
        }
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

/// Create work order operation request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkOrderOperation {
    pub work_order_id: i64,
    pub operation_sequence: i32,
    pub operation_name: String,
    pub work_center_id: Option<i64>,
    pub planned_hours: Decimal,
}

impl CreateWorkOrderOperation {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.operation_name.trim().is_empty() {
            errors.push("Operation name is required".to_string());
        }
        if self.planned_hours < Decimal::ZERO {
            errors.push("Planned hours cannot be negative".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create work order material request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkOrderMaterial {
    pub work_order_id: i64,
    pub product_id: i64,
    pub quantity_required: Decimal,
}

impl CreateWorkOrderMaterial {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.product_id <= 0 {
            errors.push("Product ID is required".to_string());
        }
        if self.quantity_required <= Decimal::ZERO {
            errors.push("Quantity must be positive".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ==================== BILL OF MATERIALS ====================

/// Bill of Materials entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BillOfMaterials {
    pub id: i64,
    pub tenant_id: i64,
    pub product_id: i64,
    pub version: String,
    pub is_active: bool,
    pub is_primary: bool,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for BillOfMaterials {
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

/// BOM line item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BillOfMaterialsLine {
    pub id: i64,
    pub bom_id: i64,
    pub component_product_id: i64,
    pub quantity: Decimal,
    pub unit_id: Option<i64>,
    pub scrap_percentage: Decimal,
    pub is_optional: bool,
    pub notes: Option<String>,
}

/// Create BOM request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBillOfMaterials {
    pub tenant_id: i64,
    pub product_id: i64,
    pub version: String,
    pub is_active: bool,
    pub is_primary: bool,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
}

impl CreateBillOfMaterials {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.product_id <= 0 {
            errors.push("Product ID is required".to_string());
        }
        if self.version.trim().is_empty() {
            errors.push("Version is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create BOM line request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBillOfMaterialsLine {
    pub bom_id: i64,
    pub component_product_id: i64,
    pub quantity: Decimal,
    pub unit_id: Option<i64>,
    pub scrap_percentage: Decimal,
    pub is_optional: bool,
    pub notes: Option<String>,
}

impl CreateBillOfMaterialsLine {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.component_product_id <= 0 {
            errors.push("Component product ID is required".to_string());
        }
        if self.quantity <= Decimal::ZERO {
            errors.push("Quantity must be positive".to_string());
        }
        if self.scrap_percentage < Decimal::ZERO || self.scrap_percentage > Decimal::ONE_HUNDRED {
            errors.push("Scrap percentage must be between 0 and 100".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ==================== ROUTING ====================

/// Routing entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Routing {
    pub id: i64,
    pub tenant_id: i64,
    pub product_id: i64,
    pub version: String,
    pub is_active: bool,
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for Routing {
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

/// Routing operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoutingOperation {
    pub id: i64,
    pub routing_id: i64,
    pub sequence: i32,
    pub operation_name: String,
    pub work_center_id: Option<i64>,
    pub setup_hours: Decimal,
    pub run_hours: Decimal,
    pub description: Option<String>,
}

/// Create routing request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRouting {
    pub tenant_id: i64,
    pub product_id: i64,
    pub version: String,
    pub is_active: bool,
    pub is_primary: bool,
}

/// Create routing operation request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRoutingOperation {
    pub routing_id: i64,
    pub sequence: i32,
    pub operation_name: String,
    pub work_center_id: Option<i64>,
    pub setup_hours: Decimal,
    pub run_hours: Decimal,
    pub description: Option<String>,
}

impl CreateRoutingOperation {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.routing_id <= 0 {
            errors.push("Routing ID is required".to_string());
        }
        if self.operation_name.trim().is_empty() {
            errors.push("Operation name is required".to_string());
        }
        if self.setup_hours < Decimal::ZERO {
            errors.push("Setup hours cannot be negative".to_string());
        }
        if self.run_hours < Decimal::ZERO {
            errors.push("Run hours cannot be negative".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl CreateRouting {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.product_id <= 0 {
            errors.push("Product ID is required".to_string());
        }
        if self.version.trim().is_empty() {
            errors.push("Version is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create inspection request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateInspection {
    pub tenant_id: i64,
    pub work_order_id: Option<i64>,
    pub product_id: i64,
    pub inspection_type: String,
    pub quantity_inspected: Decimal,
    pub quantity_passed: Decimal,
    pub quantity_failed: Decimal,
    pub status: InspectionStatus,
    pub inspector_id: Option<i64>,
    pub notes: Option<String>,
}

impl CreateInspection {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.product_id <= 0 {
            errors.push("Product ID is required".to_string());
        }
        if self.quantity_inspected <= Decimal::ZERO {
            errors.push("Quantity inspected must be greater than zero".to_string());
        }
        if self.quantity_passed < Decimal::ZERO {
            errors.push("Quantity passed cannot be negative".to_string());
        }
        if self.quantity_failed < Decimal::ZERO {
            errors.push("Quantity failed cannot be negative".to_string());
        }
        if (self.quantity_passed + self.quantity_failed) > self.quantity_inspected {
            errors.push("Passed + failed cannot exceed inspected quantity".to_string());
        }
        if self.inspection_type.trim().is_empty() {
            errors.push("Inspection type is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update inspection request
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateInspection {
    pub status: Option<InspectionStatus>,
    pub quantity_passed: Option<Decimal>,
    pub quantity_failed: Option<Decimal>,
    pub inspector_id: Option<i64>,
    pub notes: Option<String>,
}

/// Create non-conformance report request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateNonConformanceReport {
    pub tenant_id: i64,
    pub inspection_id: Option<i64>,
    pub product_id: i64,
    pub ncr_type: NcrType,
    pub description: String,
    pub root_cause: Option<String>,
    pub corrective_action: Option<String>,
    pub raised_by: i64,
}

impl CreateNonConformanceReport {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.product_id <= 0 {
            errors.push("Product ID is required".to_string());
        }
        if self.description.trim().is_empty() {
            errors.push("Description is required".to_string());
        }
        if self.raised_by <= 0 {
            errors.push("Raised by (user ID) is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update NCR request
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateNonConformanceReport {
    pub ncr_type: Option<NcrType>,
    pub description: Option<String>,
    pub root_cause: Option<String>,
    pub corrective_action: Option<String>,
    pub status: Option<NcrStatus>,
}

// ==================== QUALITY CONTROL ====================

/// Inspection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum InspectionStatus {
    Pending,
    InProgress,
    Passed,
    Failed,
    Rework,
}

/// Inspection entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Inspection {
    pub id: i64,
    pub tenant_id: i64,
    pub work_order_id: Option<i64>,
    pub product_id: i64,
    pub inspection_type: String,
    pub quantity_inspected: Decimal,
    pub quantity_passed: Decimal,
    pub quantity_failed: Decimal,
    pub status: InspectionStatus,
    pub inspector_id: Option<i64>,
    pub inspected_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for Inspection {
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

/// NCR type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum NcrType {
    Minor,
    Major,
    Critical,
}

/// NCR status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum NcrStatus {
    Open,
    UnderReview,
    CorrectiveAction,
    Closed,
    Rejected,
}

/// Non-conformance report
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NonConformanceReport {
    pub id: i64,
    pub tenant_id: i64,
    pub inspection_id: Option<i64>,
    pub product_id: i64,
    pub ncr_type: NcrType,
    pub description: String,
    pub root_cause: Option<String>,
    pub corrective_action: Option<String>,
    pub status: NcrStatus,
    pub raised_by: i64,
    pub raised_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for NonConformanceReport {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::SoftDeletable;
    use rust_decimal_macros::dec;

    #[test]
    fn test_create_work_order_validation() {
        let valid = CreateWorkOrder {
            tenant_id: 1,
            name: "WO-001".to_string(),
            product_id: 1,
            quantity: dec!(100),
            bom_id: None,
            routing_id: None,
            priority: WorkOrderPriority::Normal,
            planned_start: Some(Utc::now()),
            planned_end: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateWorkOrder {
            tenant_id: 1,
            name: "".to_string(),
            product_id: 0,
            quantity: dec!(-10),
            bom_id: None,
            routing_id: None,
            priority: WorkOrderPriority::Normal,
            planned_start: None,
            planned_end: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_bom_validation() {
        let valid = CreateBillOfMaterials {
            tenant_id: 1,
            product_id: 1,
            version: "1.0".to_string(),
            is_active: true,
            is_primary: true,
            valid_from: Some(Utc::now()),
            valid_to: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateBillOfMaterials {
            tenant_id: 1,
            product_id: 0,
            version: "".to_string(),
            is_active: true,
            is_primary: false,
            valid_from: None,
            valid_to: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_bom_line_validation() {
        let valid = CreateBillOfMaterialsLine {
            bom_id: 1,
            component_product_id: 2,
            quantity: dec!(5),
            unit_id: Some(1),
            scrap_percentage: dec!(5),
            is_optional: false,
            notes: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateBillOfMaterialsLine {
            bom_id: 1,
            component_product_id: 0,
            quantity: dec!(-1),
            unit_id: None,
            scrap_percentage: dec!(150),
            is_optional: false,
            notes: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_routing_validation() {
        let valid = CreateRouting {
            tenant_id: 1,
            product_id: 1,
            version: "1.0".to_string(),
            is_active: true,
            is_primary: true,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateRouting {
            tenant_id: 1,
            product_id: 0,
            version: "".to_string(),
            is_active: false,
            is_primary: false,
        };
        assert!(invalid.validate().is_err());
    }
}
