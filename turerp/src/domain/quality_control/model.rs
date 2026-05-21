//! Quality control domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
