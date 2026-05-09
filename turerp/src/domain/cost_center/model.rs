//! Cost Center / Profit Center domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::impl_soft_deletable;

/// Type of cost center: cost or profit
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema)]
pub enum CostCenterType {
    #[default]
    Cost,
    Profit,
}

impl std::fmt::Display for CostCenterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CostCenterType::Cost => write!(f, "cost"),
            CostCenterType::Profit => write!(f, "profit"),
        }
    }
}

impl std::str::FromStr for CostCenterType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cost" => Ok(CostCenterType::Cost),
            "profit" => Ok(CostCenterType::Profit),
            _ => Err(format!("Invalid cost center type: {}", s)),
        }
    }
}

/// Cost center / profit center entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CostCenter {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub center_type: CostCenterType,
    pub parent_id: Option<i64>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl_soft_deletable!(CostCenter);

/// Cost center allocation record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CostCenterAllocation {
    pub id: i64,
    pub tenant_id: i64,
    pub source_type: String,
    pub source_id: i64,
    pub cost_center_id: i64,
    pub amount: Decimal,
    pub percentage: Decimal,
    pub allocation_date: DateTime<Utc>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Profitability report entry for a cost center
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProfitabilityReport {
    pub cost_center_id: i64,
    pub cost_center_code: String,
    pub cost_center_name: String,
    pub center_type: CostCenterType,
    pub total_income: Decimal,
    pub total_expense: Decimal,
    pub net_profit: Decimal,
    pub allocation_count: i64,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
}

/// Create a new cost center
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCostCenter {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub center_type: CostCenterType,
    pub parent_id: Option<i64>,
    #[serde(default = "default_is_active")]
    pub is_active: bool,
}

fn default_is_active() -> bool {
    true
}

/// Update an existing cost center
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateCostCenter {
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub center_type: Option<CostCenterType>,
    pub parent_id: Option<Option<i64>>,
    pub is_active: Option<bool>,
}

/// Create a new cost center allocation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAllocation {
    pub source_type: String,
    pub source_id: i64,
    pub cost_center_id: i64,
    pub amount: Decimal,
    #[serde(default = "default_percentage")]
    pub percentage: Decimal,
    pub allocation_date: Option<DateTime<Utc>>,
    pub description: Option<String>,
}

fn default_percentage() -> Decimal {
    Decimal::new(100, 0)
}

/// Cost center response (excludes soft-delete metadata)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CostCenterResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub center_type: CostCenterType,
    pub parent_id: Option<i64>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<CostCenter> for CostCenterResponse {
    fn from(center: CostCenter) -> Self {
        Self {
            id: center.id,
            tenant_id: center.tenant_id,
            code: center.code,
            name: center.name,
            description: center.description,
            center_type: center.center_type,
            parent_id: center.parent_id,
            is_active: center.is_active,
            created_at: center.created_at,
            updated_at: center.updated_at,
        }
    }
}

/// Allocation response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AllocationResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub source_type: String,
    pub source_id: i64,
    pub cost_center_id: i64,
    pub amount: Decimal,
    pub percentage: Decimal,
    pub allocation_date: DateTime<Utc>,
    pub description: Option<String>,
}

impl From<CostCenterAllocation> for AllocationResponse {
    fn from(alloc: CostCenterAllocation) -> Self {
        Self {
            id: alloc.id,
            tenant_id: alloc.tenant_id,
            source_type: alloc.source_type,
            source_id: alloc.source_id,
            cost_center_id: alloc.cost_center_id,
            amount: alloc.amount,
            percentage: alloc.percentage,
            allocation_date: alloc.allocation_date,
            description: alloc.description,
        }
    }
}

/// Failed item in a bulk restore operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkRestoreFailed {
    pub id: i64,
    pub reason: String,
}

/// Response for bulk restore operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkRestoreResponse<T> {
    pub restored: usize,
    pub items: Vec<T>,
    pub failed: Vec<BulkRestoreFailed>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_center_type_display() {
        assert_eq!(CostCenterType::Cost.to_string(), "cost");
        assert_eq!(CostCenterType::Profit.to_string(), "profit");
    }

    #[test]
    fn test_cost_center_type_from_str() {
        assert_eq!(
            "cost".parse::<CostCenterType>().unwrap(),
            CostCenterType::Cost
        );
        assert_eq!(
            "profit".parse::<CostCenterType>().unwrap(),
            CostCenterType::Profit
        );
        assert_eq!(
            "COST".parse::<CostCenterType>().unwrap(),
            CostCenterType::Cost
        );
        assert_eq!(
            "PROFIT".parse::<CostCenterType>().unwrap(),
            CostCenterType::Profit
        );
        assert!("invalid".parse::<CostCenterType>().is_err());
    }

    #[test]
    fn test_cost_center_response_from_cost_center() {
        let center = CostCenter {
            id: 1,
            tenant_id: 100,
            code: "CC-001".to_string(),
            name: "Production".to_string(),
            description: Some("Main production line".to_string()),
            center_type: CostCenterType::Cost,
            parent_id: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };

        let resp = CostCenterResponse::from(center);
        assert_eq!(resp.id, 1);
        assert_eq!(resp.code, "CC-001");
        assert_eq!(resp.center_type, CostCenterType::Cost);
    }

    #[test]
    fn test_create_allocation_defaults() {
        let alloc = CreateAllocation {
            source_type: "invoice".to_string(),
            source_id: 1,
            cost_center_id: 1,
            amount: Decimal::new(1000, 0),
            percentage: default_percentage(),
            allocation_date: None,
            description: None,
        };
        assert_eq!(alloc.percentage, Decimal::new(100, 0));
    }
}
