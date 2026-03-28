//! Project domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Project status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProjectStatus {
    Planning,
    Active,
    OnHold,
    Completed,
    Cancelled,
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
    pub budget: f64,
    pub actual_cost: f64,
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
    pub planned_hours: f64,
    pub actual_hours: f64,
    pub progress: f64,
    pub sort_order: i32,
}

/// Project cost record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCost {
    pub id: i64,
    pub project_id: i64,
    pub wbs_item_id: Option<i64>,
    pub cost_type: CostType,
    pub amount: f64,
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

/// Project profitability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProfitability {
    pub project_id: i64,
    pub project_name: String,
    pub budget: f64,
    pub actual_cost: f64,
    pub revenue: f64,
    pub profit: f64,
    pub profit_margin: f64,
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
    pub budget: f64,
}

impl CreateProject {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Project name is required".to_string());
        }
        if self.budget < 0.0 {
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
    pub planned_hours: f64,
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
    pub amount: f64,
    pub description: String,
    pub incurred_at: DateTime<Utc>,
}

impl CreateProjectCost {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.amount <= 0.0 {
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

    #[test]
    fn test_create_project_validation() {
        let valid = CreateProject {
            tenant_id: 1,
            name: "Test Project".to_string(),
            description: Some("Description".to_string()),
            cari_id: None,
            start_date: Some(Utc::now()),
            end_date: None,
            budget: 10000.0,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateProject {
            tenant_id: 1,
            name: "".to_string(),
            description: None,
            cari_id: None,
            start_date: None,
            end_date: None,
            budget: -100.0,
        };
        assert!(invalid.validate().is_err());
    }
}
