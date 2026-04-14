//! Project domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Project status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ProjectStatus {
    #[default]
    Planning,
    Active,
    OnHold,
    Completed,
    Cancelled,
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectStatus::Planning => write!(f, "planning"),
            ProjectStatus::Active => write!(f, "active"),
            ProjectStatus::OnHold => write!(f, "on_hold"),
            ProjectStatus::Completed => write!(f, "completed"),
            ProjectStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for ProjectStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "planning" => Ok(ProjectStatus::Planning),
            "active" => Ok(ProjectStatus::Active),
            "on_hold" => Ok(ProjectStatus::OnHold),
            "completed" => Ok(ProjectStatus::Completed),
            "cancelled" => Ok(ProjectStatus::Cancelled),
            _ => Err(format!("Invalid project status: {}", s)),
        }
    }
}

/// Project entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub cari_id: Option<i64>,
    pub status: ProjectStatus,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub budget: Decimal,
    pub actual_cost: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Work Breakdown Structure (WBS) item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbsItem {
    pub id: i64,
    pub project_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub code: String,
    pub planned_hours: Decimal,
    pub actual_hours: Decimal,
    pub progress: Decimal,
    pub sort_order: i32,
}

/// Project cost record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCost {
    pub id: i64,
    pub project_id: i64,
    pub wbs_item_id: Option<i64>,
    pub cost_type: CostType,
    pub amount: Decimal,
    pub description: String,
    pub incurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Cost type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CostType {
    Labor,
    Material,
    Equipment,
    Subcontract,
    Other,
}

impl std::fmt::Display for CostType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CostType::Labor => write!(f, "labor"),
            CostType::Material => write!(f, "material"),
            CostType::Equipment => write!(f, "equipment"),
            CostType::Subcontract => write!(f, "subcontract"),
            CostType::Other => write!(f, "other"),
        }
    }
}

impl std::str::FromStr for CostType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "labor" => Ok(CostType::Labor),
            "material" => Ok(CostType::Material),
            "equipment" => Ok(CostType::Equipment),
            "subcontract" => Ok(CostType::Subcontract),
            "other" => Ok(CostType::Other),
            _ => Err(format!("Invalid cost type: {}", s)),
        }
    }
}

/// Project profitability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProfitability {
    pub project_id: i64,
    pub project_name: String,
    pub budget: Decimal,
    pub actual_cost: Decimal,
    pub revenue: Decimal,
    pub profit: Decimal,
    pub profit_margin: Decimal,
}

/// Create project request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProject {
    pub tenant_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub cari_id: Option<i64>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub budget: Decimal,
}

impl CreateProject {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Project name is required".to_string());
        }
        if self.budget < Decimal::ZERO {
            errors.push("Budget cannot be negative".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create WBS item request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWbsItem {
    pub project_id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub code: String,
    pub planned_hours: Decimal,
}

impl CreateWbsItem {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Name is required".to_string());
        }
        if self.code.trim().is_empty() {
            errors.push("Code is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create project cost request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectCost {
    pub project_id: i64,
    pub wbs_item_id: Option<i64>,
    pub cost_type: CostType,
    pub amount: Decimal,
    pub description: String,
    pub incurred_at: DateTime<Utc>,
}

impl CreateProjectCost {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.amount <= Decimal::ZERO {
            errors.push("Amount must be positive".to_string());
        }
        if self.description.trim().is_empty() {
            errors.push("Description is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_create_project_validation() {
        let valid = CreateProject {
            tenant_id: 1,
            name: "Test Project".to_string(),
            description: Some("Description".to_string()),
            cari_id: None,
            start_date: Some(Utc::now()),
            end_date: None,
            budget: dec!(10000),
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateProject {
            tenant_id: 1,
            name: "".to_string(),
            description: None,
            cari_id: None,
            start_date: None,
            end_date: None,
            budget: dec!(-100),
        };
        assert!(invalid.validate().is_err());
    }
}
