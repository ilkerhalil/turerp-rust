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

/// Budget period type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema)]
pub enum BudgetPeriod {
    #[default]
    Monthly,
    Quarterly,
    Yearly,
}

impl std::fmt::Display for BudgetPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetPeriod::Monthly => write!(f, "monthly"),
            BudgetPeriod::Quarterly => write!(f, "quarterly"),
            BudgetPeriod::Yearly => write!(f, "yearly"),
        }
    }
}

impl std::str::FromStr for BudgetPeriod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "monthly" => Ok(BudgetPeriod::Monthly),
            "quarterly" => Ok(BudgetPeriod::Quarterly),
            "yearly" => Ok(BudgetPeriod::Yearly),
            _ => Err(format!("Invalid budget period: {}", s)),
        }
    }
}

/// Budget entity for a cost center
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Budget {
    pub id: i64,
    pub tenant_id: i64,
    pub cost_center_id: i64,
    pub period: BudgetPeriod,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub budgeted_amount: Decimal,
    pub actual_amount: Decimal,
    pub variance: Decimal,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Create a new budget
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBudget {
    pub cost_center_id: i64,
    pub period: BudgetPeriod,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub budgeted_amount: Decimal,
    pub notes: Option<String>,
}

/// Update an existing budget
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateBudget {
    pub period: Option<BudgetPeriod>,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
    pub budgeted_amount: Option<Decimal>,
    pub notes: Option<Option<String>>,
}

/// Budget response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BudgetResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub cost_center_id: i64,
    pub period: BudgetPeriod,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub budgeted_amount: Decimal,
    pub actual_amount: Decimal,
    pub variance: Decimal,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<Budget> for BudgetResponse {
    fn from(budget: Budget) -> Self {
        Self {
            id: budget.id,
            tenant_id: budget.tenant_id,
            cost_center_id: budget.cost_center_id,
            period: budget.period,
            period_start: budget.period_start,
            period_end: budget.period_end,
            budgeted_amount: budget.budgeted_amount,
            actual_amount: budget.actual_amount,
            variance: budget.variance,
            notes: budget.notes,
            created_at: budget.created_at,
            updated_at: budget.updated_at,
        }
    }
}

/// Allocation rule type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema)]
pub enum AllocationRuleType {
    #[default]
    Percentage,
    FixedAmount,
}

impl std::fmt::Display for AllocationRuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AllocationRuleType::Percentage => write!(f, "percentage"),
            AllocationRuleType::FixedAmount => write!(f, "fixed_amount"),
        }
    }
}

impl std::str::FromStr for AllocationRuleType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "percentage" => Ok(AllocationRuleType::Percentage),
            "fixed_amount" => Ok(AllocationRuleType::FixedAmount),
            _ => Err(format!("Invalid allocation rule type: {}", s)),
        }
    }
}

/// Allocation rule for auto-generating cost center allocations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AllocationRule {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub source_type: String,
    pub account_range_start: Option<String>,
    pub account_range_end: Option<String>,
    pub cost_center_id: i64,
    pub rule_type: AllocationRuleType,
    pub percentage: Option<Decimal>,
    pub fixed_amount: Option<Decimal>,
    pub is_active: bool,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Create an allocation rule
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateAllocationRule {
    pub name: String,
    pub source_type: String,
    pub account_range_start: Option<String>,
    pub account_range_end: Option<String>,
    pub cost_center_id: i64,
    pub rule_type: AllocationRuleType,
    pub percentage: Option<Decimal>,
    pub fixed_amount: Option<Decimal>,
    #[serde(default = "default_is_active")]
    pub is_active: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
}

fn default_priority() -> i32 {
    0
}

/// Update an allocation rule
#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct UpdateAllocationRule {
    pub name: Option<String>,
    pub source_type: Option<String>,
    pub account_range_start: Option<Option<String>>,
    pub account_range_end: Option<Option<String>>,
    pub cost_center_id: Option<i64>,
    pub rule_type: Option<AllocationRuleType>,
    pub percentage: Option<Option<Decimal>>,
    pub fixed_amount: Option<Option<Decimal>>,
    pub is_active: Option<bool>,
    pub priority: Option<i32>,
}

/// Allocation rule response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AllocationRuleResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub source_type: String,
    pub account_range_start: Option<String>,
    pub account_range_end: Option<String>,
    pub cost_center_id: i64,
    pub rule_type: AllocationRuleType,
    pub percentage: Option<Decimal>,
    pub fixed_amount: Option<Decimal>,
    pub is_active: bool,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<AllocationRule> for AllocationRuleResponse {
    fn from(rule: AllocationRule) -> Self {
        Self {
            id: rule.id,
            tenant_id: rule.tenant_id,
            name: rule.name,
            source_type: rule.source_type,
            account_range_start: rule.account_range_start,
            account_range_end: rule.account_range_end,
            cost_center_id: rule.cost_center_id,
            rule_type: rule.rule_type,
            percentage: rule.percentage,
            fixed_amount: rule.fixed_amount,
            is_active: rule.is_active,
            priority: rule.priority,
            created_at: rule.created_at,
            updated_at: rule.updated_at,
        }
    }
}

/// Variance report line item for a single period / cost center
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VarianceReportLine {
    pub cost_center_id: i64,
    pub cost_center_code: String,
    pub cost_center_name: String,
    pub period: BudgetPeriod,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub budgeted_amount: Decimal,
    pub actual_amount: Decimal,
    pub variance: Decimal,
    pub variance_percentage: Decimal,
}

/// Budget vs Actual variance report
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VarianceReport {
    pub tenant_id: i64,
    pub total_budgeted: Decimal,
    pub total_actual: Decimal,
    pub total_variance: Decimal,
    pub lines: Vec<VarianceReportLine>,
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

    #[test]
    fn test_budget_period_display() {
        assert_eq!(BudgetPeriod::Monthly.to_string(), "monthly");
        assert_eq!(BudgetPeriod::Quarterly.to_string(), "quarterly");
        assert_eq!(BudgetPeriod::Yearly.to_string(), "yearly");
    }

    #[test]
    fn test_budget_period_from_str() {
        assert_eq!(
            "monthly".parse::<BudgetPeriod>().unwrap(),
            BudgetPeriod::Monthly
        );
        assert_eq!(
            "QUARTERLY".parse::<BudgetPeriod>().unwrap(),
            BudgetPeriod::Quarterly
        );
        assert!("invalid".parse::<BudgetPeriod>().is_err());
    }

    #[test]
    fn test_allocation_rule_type_display() {
        assert_eq!(AllocationRuleType::Percentage.to_string(), "percentage");
        assert_eq!(AllocationRuleType::FixedAmount.to_string(), "fixed_amount");
    }

    #[test]
    fn test_allocation_rule_type_from_str() {
        assert_eq!(
            "percentage".parse::<AllocationRuleType>().unwrap(),
            AllocationRuleType::Percentage
        );
        assert_eq!(
            "FIXED_AMOUNT".parse::<AllocationRuleType>().unwrap(),
            AllocationRuleType::FixedAmount
        );
        assert!("invalid".parse::<AllocationRuleType>().is_err());
    }

    #[test]
    fn test_budget_response_from_budget() {
        let budget = Budget {
            id: 1,
            tenant_id: 100,
            cost_center_id: 5,
            period: BudgetPeriod::Monthly,
            period_start: Utc::now(),
            period_end: Utc::now(),
            budgeted_amount: Decimal::new(5000, 0),
            actual_amount: Decimal::new(4500, 0),
            variance: Decimal::new(500, 0),
            notes: Some("Q1 budget".to_string()),
            created_at: Utc::now(),
            updated_at: None,
        };
        let resp = BudgetResponse::from(budget);
        assert_eq!(resp.id, 1);
        assert_eq!(resp.budgeted_amount, Decimal::new(5000, 0));
    }

    #[test]
    fn test_create_allocation_rule_defaults() {
        let rule = CreateAllocationRule {
            name: "Payroll Split".to_string(),
            source_type: "payroll".to_string(),
            account_range_start: None,
            account_range_end: None,
            cost_center_id: 1,
            rule_type: AllocationRuleType::Percentage,
            percentage: Some(Decimal::new(50, 0)),
            fixed_amount: None,
            is_active: default_is_active(),
            priority: default_priority(),
        };
        assert!(rule.is_active);
        assert_eq!(rule.priority, 0);
    }
}
