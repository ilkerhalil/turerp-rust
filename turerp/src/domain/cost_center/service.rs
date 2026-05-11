//! Cost Center service — business logic for cost center management

use chrono::{DateTime, Utc};

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::cost_center::model::{
    AllocationRule, Budget, BulkRestoreFailed, BulkRestoreResponse, CostCenter,
    CostCenterAllocation, CostCenterResponse, CostCenterType, CreateAllocation,
    CreateAllocationRule, CreateBudget, CreateCostCenter, ProfitabilityReport,
    UpdateAllocationRule, UpdateBudget, UpdateCostCenter, VarianceReport,
};
use crate::domain::cost_center::repository::BoxCostCenterRepository;
use crate::error::ApiError;

/// Service for managing cost centers and allocations
#[derive(Clone)]
pub struct CostCenterService {
    repo: BoxCostCenterRepository,
}

impl CostCenterService {
    pub fn new(repo: BoxCostCenterRepository) -> Self {
        Self { repo }
    }

    // ---- Cost Center Operations ----

    /// Create a new cost center
    pub async fn create_cost_center(
        &self,
        create: CreateCostCenter,
        tenant_id: i64,
    ) -> Result<CostCenter, ApiError> {
        if create.code.trim().is_empty() {
            tracing::warn!(tenant_id, "Cost center code is empty");
            return Err(ApiError::Validation("Code is required".to_string()));
        }
        if create.name.trim().is_empty() {
            tracing::warn!(tenant_id, "Cost center name is empty");
            return Err(ApiError::Validation("Name is required".to_string()));
        }
        let center = self.repo.create(create, tenant_id).await?;
        tracing::info!(tenant_id, cost_center_id = center.id, "Created cost center");
        Ok(center)
    }

    /// Get a cost center by ID
    pub async fn get_cost_center(&self, id: i64, tenant_id: i64) -> Result<CostCenter, ApiError> {
        self.repo.find_by_id(id, tenant_id).await?.ok_or_else(|| {
            tracing::warn!(tenant_id, cost_center_id = id, "Cost center not found");
            ApiError::NotFound(format!("Cost center {} not found", id))
        })
    }

    /// List cost centers with optional type filter and pagination
    pub async fn list_cost_centers(
        &self,
        tenant_id: i64,
        center_type: Option<CostCenterType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<CostCenter>, ApiError> {
        self.repo.find_all(tenant_id, center_type, params).await
    }

    /// Find cost centers by type
    pub async fn list_by_type(
        &self,
        tenant_id: i64,
        center_type: CostCenterType,
    ) -> Result<Vec<CostCenter>, ApiError> {
        self.repo.find_by_type(tenant_id, center_type).await
    }

    /// Update a cost center
    pub async fn update_cost_center(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCostCenter,
    ) -> Result<CostCenter, ApiError> {
        let center = self.repo.update(id, tenant_id, update).await?;
        tracing::info!(tenant_id, cost_center_id = id, "Updated cost center");
        Ok(center)
    }

    /// Soft delete a cost center
    pub async fn delete_cost_center(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete(id, tenant_id, deleted_by).await?;
        tracing::info!(tenant_id, cost_center_id = id, "Deleted cost center");
        Ok(())
    }

    /// Restore a soft-deleted cost center
    pub async fn restore_cost_center(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<CostCenter, ApiError> {
        let center = self.repo.restore(id, tenant_id).await?;
        tracing::info!(tenant_id, cost_center_id = id, "Restored cost center");
        Ok(center)
    }

    /// Bulk restore soft-deleted cost centers
    pub async fn bulk_restore_cost_centers(
        &self,
        ids: Vec<i64>,
        tenant_id: i64,
    ) -> Result<BulkRestoreResponse<CostCenterResponse>, ApiError> {
        let mut restored = Vec::new();
        let mut failed = Vec::new();
        for id in ids {
            match self.repo.restore(id, tenant_id).await {
                Ok(center) => restored.push(CostCenterResponse::from(center)),
                Err(e) => {
                    tracing::warn!("Failed to restore cost center {}: {}", id, e);
                    failed.push(BulkRestoreFailed {
                        id,
                        reason: e.to_string(),
                    });
                }
            }
        }
        tracing::info!(
            tenant_id,
            restored = restored.len(),
            failed = failed.len(),
            "Bulk restored cost centers"
        );
        Ok(BulkRestoreResponse {
            restored: restored.len(),
            items: restored,
            failed,
        })
    }

    /// List soft-deleted cost centers
    pub async fn list_deleted_cost_centers(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<CostCenter>, ApiError> {
        self.repo.find_deleted(tenant_id).await
    }

    /// Permanently destroy a soft-deleted cost center
    pub async fn destroy_cost_center(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id, tenant_id).await?;
        tracing::info!(tenant_id, cost_center_id = id, "Destroyed cost center");
        Ok(())
    }

    // ---- Allocation Operations ----

    /// Create a new allocation
    pub async fn create_allocation(
        &self,
        allocation: CreateAllocation,
        tenant_id: i64,
    ) -> Result<CostCenterAllocation, ApiError> {
        // Verify the cost center exists
        self.get_cost_center(allocation.cost_center_id, tenant_id)
            .await?;

        if allocation.amount < rust_decimal::Decimal::ZERO {
            tracing::warn!(tenant_id, "Allocation amount is negative");
            return Err(ApiError::Validation(
                "Amount cannot be negative".to_string(),
            ));
        }
        if allocation.percentage <= rust_decimal::Decimal::ZERO
            || allocation.percentage > rust_decimal::Decimal::new(100, 0)
        {
            tracing::warn!(tenant_id, "Allocation percentage is invalid");
            return Err(ApiError::Validation(
                "Percentage must be between 0 and 100".to_string(),
            ));
        }

        let cost_center_id = allocation.cost_center_id;
        let alloc = self.repo.create_allocation(allocation, tenant_id).await?;
        tracing::info!(tenant_id, cost_center_id, "Created cost center allocation");
        Ok(alloc)
    }

    /// Get allocations for a cost center
    pub async fn get_allocations(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<CostCenterAllocation>, ApiError> {
        self.get_cost_center(cost_center_id, tenant_id).await?;
        self.repo.get_allocations(cost_center_id, tenant_id).await
    }

    // ---- Profitability Report ----

    /// Get profitability report for a cost center
    pub async fn get_profitability_report(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
        period_start: Option<DateTime<Utc>>,
        period_end: Option<DateTime<Utc>>,
    ) -> Result<ProfitabilityReport, ApiError> {
        self.get_cost_center(cost_center_id, tenant_id).await?;
        self.repo
            .get_profitability_report(cost_center_id, tenant_id, period_start, period_end)
            .await
    }

    // ---- Budget Operations ----

    /// Create a new budget
    pub async fn create_budget(
        &self,
        budget: CreateBudget,
        tenant_id: i64,
    ) -> Result<Budget, ApiError> {
        self.get_cost_center(budget.cost_center_id, tenant_id)
            .await?;
        if budget.budgeted_amount < rust_decimal::Decimal::ZERO {
            return Err(ApiError::Validation(
                "Budgeted amount cannot be negative".to_string(),
            ));
        }
        if budget.period_end <= budget.period_start {
            return Err(ApiError::Validation(
                "Period end must be after period start".to_string(),
            ));
        }
        self.repo.create_budget(budget, tenant_id).await
    }

    /// Get a budget by ID
    pub async fn get_budget(&self, id: i64, tenant_id: i64) -> Result<Budget, ApiError> {
        self.repo
            .find_budget_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Budget {} not found", id)))
    }

    /// List budgets for a cost center
    pub async fn list_budgets(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Budget>, ApiError> {
        self.get_cost_center(cost_center_id, tenant_id).await?;
        self.repo
            .find_budgets_by_cost_center(cost_center_id, tenant_id)
            .await
    }

    /// Update a budget
    pub async fn update_budget(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateBudget,
    ) -> Result<Budget, ApiError> {
        let existing = self.get_budget(id, tenant_id).await?;
        if let Some(amount) = update.budgeted_amount {
            if amount < rust_decimal::Decimal::ZERO {
                return Err(ApiError::Validation(
                    "Budgeted amount cannot be negative".to_string(),
                ));
            }
        }
        if update.period_start.is_some() || update.period_end.is_some() {
            let start = update.period_start.unwrap_or(existing.period_start);
            let end = update.period_end.unwrap_or(existing.period_end);
            if end <= start {
                return Err(ApiError::Validation(
                    "Period end must be after period start".to_string(),
                ));
            }
        }
        self.repo.update_budget(id, tenant_id, update).await
    }

    /// Delete a budget
    pub async fn delete_budget(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.get_budget(id, tenant_id).await?;
        self.repo.delete_budget(id, tenant_id).await
    }

    /// Generate variance report for a cost center
    pub async fn generate_variance_report(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
        period_start: Option<DateTime<Utc>>,
        period_end: Option<DateTime<Utc>>,
    ) -> Result<VarianceReport, ApiError> {
        self.get_cost_center(cost_center_id, tenant_id).await?;
        self.repo
            .get_variance_report(cost_center_id, tenant_id, period_start, period_end)
            .await
    }

    // ---- Allocation Rule Operations ----

    /// Create an allocation rule
    pub async fn create_allocation_rule(
        &self,
        rule: CreateAllocationRule,
        tenant_id: i64,
    ) -> Result<AllocationRule, ApiError> {
        self.get_cost_center(rule.cost_center_id, tenant_id).await?;
        if rule.name.trim().is_empty() {
            return Err(ApiError::Validation("Rule name is required".to_string()));
        }
        if rule.source_type.trim().is_empty() {
            return Err(ApiError::Validation("Source type is required".to_string()));
        }
        match rule.rule_type {
            crate::domain::cost_center::model::AllocationRuleType::Percentage => {
                let Some(pct) = rule.percentage else {
                    return Err(ApiError::Validation(
                        "Percentage is required for percentage rules".to_string(),
                    ));
                };
                if pct <= rust_decimal::Decimal::ZERO || pct > rust_decimal::Decimal::new(100, 0) {
                    return Err(ApiError::Validation(
                        "Percentage must be between 0 and 100".to_string(),
                    ));
                }
            }
            crate::domain::cost_center::model::AllocationRuleType::FixedAmount => {
                if rule.fixed_amount.is_none() {
                    return Err(ApiError::Validation(
                        "Fixed amount is required for fixed amount rules".to_string(),
                    ));
                }
            }
        }
        self.repo.create_allocation_rule(rule, tenant_id).await
    }

    /// Get an allocation rule by ID
    pub async fn get_allocation_rule(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<AllocationRule, ApiError> {
        self.repo
            .find_allocation_rule_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Allocation rule {} not found", id)))
    }

    /// List allocation rules for a cost center
    pub async fn list_allocation_rules(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<AllocationRule>, ApiError> {
        self.get_cost_center(cost_center_id, tenant_id).await?;
        self.repo
            .find_allocation_rules_by_cost_center(cost_center_id, tenant_id)
            .await
    }

    /// List active allocation rules for a source type
    pub async fn get_rules_for_source(
        &self,
        source_type: &str,
        tenant_id: i64,
    ) -> Result<Vec<AllocationRule>, ApiError> {
        self.repo
            .find_allocation_rules_by_source(source_type, tenant_id)
            .await
    }

    /// Update an allocation rule
    pub async fn update_allocation_rule(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateAllocationRule,
    ) -> Result<AllocationRule, ApiError> {
        let existing = self.get_allocation_rule(id, tenant_id).await?;
        if let Some(rule_type) = &update.rule_type {
            match rule_type {
                crate::domain::cost_center::model::AllocationRuleType::Percentage => {
                    let pct = update.percentage.unwrap_or(existing.percentage);
                    let Some(p) = pct else {
                        return Err(ApiError::Validation(
                            "Percentage is required for percentage rules".to_string(),
                        ));
                    };
                    if p <= rust_decimal::Decimal::ZERO || p > rust_decimal::Decimal::new(100, 0) {
                        return Err(ApiError::Validation(
                            "Percentage must be between 0 and 100".to_string(),
                        ));
                    }
                }
                crate::domain::cost_center::model::AllocationRuleType::FixedAmount => {
                    let fixed = update.fixed_amount.unwrap_or(existing.fixed_amount);
                    if fixed.is_none() {
                        return Err(ApiError::Validation(
                            "Fixed amount is required for fixed amount rules".to_string(),
                        ));
                    }
                }
            }
        }
        self.repo
            .update_allocation_rule(id, tenant_id, update)
            .await
    }

    /// Delete an allocation rule
    pub async fn delete_allocation_rule(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.get_allocation_rule(id, tenant_id).await?;
        self.repo.delete_allocation_rule(id, tenant_id).await
    }

    /// Apply matching allocation rules to a source and amount, generating allocations
    pub async fn apply_allocation_rules(
        &self,
        source_type: &str,
        source_id: i64,
        base_amount: rust_decimal::Decimal,
        account_code: Option<&str>,
        tenant_id: i64,
    ) -> Result<Vec<CostCenterAllocation>, ApiError> {
        let rules = self.get_rules_for_source(source_type, tenant_id).await?;
        let mut allocations = Vec::new();

        for rule in rules {
            // Check account range filter
            if let Some(ref start) = rule.account_range_start {
                if let Some(code) = account_code {
                    if code < start.as_str() {
                        continue;
                    }
                }
            }
            if let Some(ref end) = rule.account_range_end {
                if let Some(code) = account_code {
                    if code > end.as_str() {
                        continue;
                    }
                }
            }

            let amount = match rule.rule_type {
                crate::domain::cost_center::model::AllocationRuleType::Percentage => {
                    let pct = rule.percentage.unwrap_or(rust_decimal::Decimal::ZERO);
                    base_amount * (pct / rust_decimal::Decimal::new(100, 0))
                }
                crate::domain::cost_center::model::AllocationRuleType::FixedAmount => {
                    rule.fixed_amount.unwrap_or(rust_decimal::Decimal::ZERO)
                }
            };

            if amount > rust_decimal::Decimal::ZERO {
                let alloc = self
                    .create_allocation(
                        CreateAllocation {
                            source_type: source_type.to_string(),
                            source_id,
                            cost_center_id: rule.cost_center_id,
                            amount,
                            percentage: rule
                                .percentage
                                .unwrap_or(rust_decimal::Decimal::new(100, 0)),
                            allocation_date: Some(chrono::Utc::now()),
                            description: Some(format!("Auto-allocation via rule: {}", rule.name)),
                        },
                        tenant_id,
                    )
                    .await?;
                allocations.push(alloc);
            }
        }

        Ok(allocations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cost_center::repository::InMemoryCostCenterRepository;
    use rust_decimal::Decimal;
    use std::sync::Arc;

    fn make_service() -> CostCenterService {
        let repo = Arc::new(InMemoryCostCenterRepository::new());
        CostCenterService::new(repo)
    }

    #[tokio::test]
    async fn test_create_and_get_cost_center() {
        let svc = make_service();

        let create = CreateCostCenter {
            code: "CC-001".to_string(),
            name: "Production".to_string(),
            description: None,
            center_type: CostCenterType::Cost,
            parent_id: None,
            is_active: true,
        };

        let center = svc.create_cost_center(create, 1).await.unwrap();
        assert_eq!(center.code, "CC-001");
        assert_eq!(center.center_type, CostCenterType::Cost);

        let found = svc.get_cost_center(center.id, 1).await.unwrap();
        assert_eq!(found.id, center.id);
    }

    #[tokio::test]
    async fn test_create_validation() {
        let svc = make_service();

        let empty_code = CreateCostCenter {
            code: "  ".to_string(),
            name: "Production".to_string(),
            description: None,
            center_type: CostCenterType::Cost,
            parent_id: None,
            is_active: true,
        };
        assert!(svc.create_cost_center(empty_code, 1).await.is_err());

        let empty_name = CreateCostCenter {
            code: "CC-001".to_string(),
            name: "  ".to_string(),
            description: None,
            center_type: CostCenterType::Cost,
            parent_id: None,
            is_active: true,
        };
        assert!(svc.create_cost_center(empty_name, 1).await.is_err());
    }

    #[tokio::test]
    async fn test_list_cost_centers() {
        let svc = make_service();

        for i in 1..=3 {
            let create = CreateCostCenter {
                code: format!("CC-00{}", i),
                name: format!("Center {}", i),
                description: None,
                center_type: if i % 2 == 0 {
                    CostCenterType::Profit
                } else {
                    CostCenterType::Cost
                },
                parent_id: None,
                is_active: true,
            };
            svc.create_cost_center(create, 1).await.unwrap();
        }

        let params = PaginationParams::default();
        let all = svc.list_cost_centers(1, None, params).await.unwrap();
        assert_eq!(all.items.len(), 3);

        let params = PaginationParams::default();
        let costs = svc
            .list_cost_centers(1, Some(CostCenterType::Cost), params)
            .await
            .unwrap();
        assert_eq!(costs.items.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_and_restore() {
        let svc = make_service();

        let create = CreateCostCenter {
            code: "CC-001".to_string(),
            name: "Production".to_string(),
            description: None,
            center_type: CostCenterType::Cost,
            parent_id: None,
            is_active: true,
        };
        let center = svc.create_cost_center(create, 1).await.unwrap();

        svc.delete_cost_center(center.id, 1, 1).await.unwrap();
        let result = svc.get_cost_center(center.id, 1).await;
        assert!(result.is_err());

        let restored = svc.restore_cost_center(center.id, 1).await.unwrap();
        assert_eq!(restored.id, center.id);
    }

    #[tokio::test]
    async fn test_allocations() {
        let svc = make_service();

        let create = CreateCostCenter {
            code: "PC-001".to_string(),
            name: "Sales".to_string(),
            description: None,
            center_type: CostCenterType::Profit,
            parent_id: None,
            is_active: true,
        };
        let center = svc.create_cost_center(create, 1).await.unwrap();

        let alloc = CreateAllocation {
            source_type: "invoice".to_string(),
            source_id: 1,
            cost_center_id: center.id,
            amount: Decimal::new(1000, 0),
            percentage: Decimal::new(100, 0),
            allocation_date: None,
            description: None,
        };
        let created = svc.create_allocation(alloc, 1).await.unwrap();
        assert_eq!(created.amount, Decimal::new(1000, 0));

        let allocs = svc.get_allocations(center.id, 1).await.unwrap();
        assert_eq!(allocs.len(), 1);
    }

    #[tokio::test]
    async fn test_allocation_validation() {
        let svc = make_service();

        let create = CreateCostCenter {
            code: "CC-001".to_string(),
            name: "Production".to_string(),
            description: None,
            center_type: CostCenterType::Cost,
            parent_id: None,
            is_active: true,
        };
        let center = svc.create_cost_center(create, 1).await.unwrap();

        let negative_amount = CreateAllocation {
            source_type: "invoice".to_string(),
            source_id: 1,
            cost_center_id: center.id,
            amount: Decimal::new(-100, 0),
            percentage: Decimal::new(100, 0),
            allocation_date: None,
            description: None,
        };
        assert!(svc.create_allocation(negative_amount, 1).await.is_err());

        let bad_percentage = CreateAllocation {
            source_type: "invoice".to_string(),
            source_id: 1,
            cost_center_id: center.id,
            amount: Decimal::new(100, 0),
            percentage: Decimal::new(101, 0),
            allocation_date: None,
            description: None,
        };
        assert!(svc.create_allocation(bad_percentage, 1).await.is_err());
    }

    #[tokio::test]
    async fn test_profitability_report() {
        let svc = make_service();

        let create = CreateCostCenter {
            code: "PC-001".to_string(),
            name: "Sales East".to_string(),
            description: None,
            center_type: CostCenterType::Profit,
            parent_id: None,
            is_active: true,
        };
        let center = svc.create_cost_center(create, 1).await.unwrap();

        // Income
        svc.create_allocation(
            CreateAllocation {
                source_type: "invoice".to_string(),
                source_id: 1,
                cost_center_id: center.id,
                amount: Decimal::new(10000, 0),
                percentage: Decimal::new(100, 0),
                allocation_date: None,
                description: None,
            },
            1,
        )
        .await
        .unwrap();

        // Expense
        svc.create_allocation(
            CreateAllocation {
                source_type: "payroll".to_string(),
                source_id: 2,
                cost_center_id: center.id,
                amount: Decimal::new(3000, 0),
                percentage: Decimal::new(100, 0),
                allocation_date: None,
                description: None,
            },
            1,
        )
        .await
        .unwrap();

        let report = svc
            .get_profitability_report(center.id, 1, None, None)
            .await
            .unwrap();
        assert_eq!(report.total_income, Decimal::new(10000, 0));
        assert_eq!(report.total_expense, Decimal::new(3000, 0));
        assert_eq!(report.net_profit, Decimal::new(7000, 0));
        assert_eq!(report.allocation_count, 2);
    }

    #[tokio::test]
    async fn test_budget_crud() {
        let svc = make_service();

        let center = svc
            .create_cost_center(
                CreateCostCenter {
                    code: "CC-001".to_string(),
                    name: "Production".to_string(),
                    description: None,
                    center_type: CostCenterType::Cost,
                    parent_id: None,
                    is_active: true,
                },
                1,
            )
            .await
            .unwrap();

        let now = Utc::now();
        let budget = svc
            .create_budget(
                CreateBudget {
                    cost_center_id: center.id,
                    period: crate::domain::cost_center::model::BudgetPeriod::Monthly,
                    period_start: now,
                    period_end: now + chrono::Duration::days(30),
                    budgeted_amount: Decimal::new(10000, 0),
                    notes: Some("Jan budget".to_string()),
                },
                1,
            )
            .await
            .unwrap();
        assert_eq!(budget.budgeted_amount, Decimal::new(10000, 0));

        let found = svc.get_budget(budget.id, 1).await.unwrap();
        assert_eq!(found.id, budget.id);

        let budgets = svc.list_budgets(center.id, 1).await.unwrap();
        assert_eq!(budgets.len(), 1);

        let updated = svc
            .update_budget(
                budget.id,
                1,
                UpdateBudget {
                    budgeted_amount: Some(Decimal::new(12000, 0)),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.budgeted_amount, Decimal::new(12000, 0));

        svc.delete_budget(budget.id, 1).await.unwrap();
        assert!(svc.get_budget(budget.id, 1).await.is_err());
    }

    #[tokio::test]
    async fn test_budget_validation() {
        let svc = make_service();

        let center = svc
            .create_cost_center(
                CreateCostCenter {
                    code: "CC-001".to_string(),
                    name: "Production".to_string(),
                    description: None,
                    center_type: CostCenterType::Cost,
                    parent_id: None,
                    is_active: true,
                },
                1,
            )
            .await
            .unwrap();

        let now = Utc::now();
        let negative = CreateBudget {
            cost_center_id: center.id,
            period: crate::domain::cost_center::model::BudgetPeriod::Monthly,
            period_start: now,
            period_end: now + chrono::Duration::days(30),
            budgeted_amount: Decimal::new(-100, 0),
            notes: None,
        };
        assert!(svc.create_budget(negative, 1).await.is_err());

        let bad_dates = CreateBudget {
            cost_center_id: center.id,
            period: crate::domain::cost_center::model::BudgetPeriod::Monthly,
            period_start: now,
            period_end: now - chrono::Duration::days(1),
            budgeted_amount: Decimal::new(1000, 0),
            notes: None,
        };
        assert!(svc.create_budget(bad_dates, 1).await.is_err());
    }

    #[tokio::test]
    async fn test_variance_report() {
        let svc = make_service();

        let center = svc
            .create_cost_center(
                CreateCostCenter {
                    code: "CC-001".to_string(),
                    name: "Production".to_string(),
                    description: None,
                    center_type: CostCenterType::Cost,
                    parent_id: None,
                    is_active: true,
                },
                1,
            )
            .await
            .unwrap();

        let now = Utc::now();
        svc.create_budget(
            CreateBudget {
                cost_center_id: center.id,
                period: crate::domain::cost_center::model::BudgetPeriod::Monthly,
                period_start: now,
                period_end: now + chrono::Duration::days(30),
                budgeted_amount: Decimal::new(10000, 0),
                notes: None,
            },
            1,
        )
        .await
        .unwrap();

        svc.create_allocation(
            CreateAllocation {
                source_type: "payroll".to_string(),
                source_id: 1,
                cost_center_id: center.id,
                amount: Decimal::new(3000, 0),
                percentage: Decimal::new(100, 0),
                allocation_date: Some(now + chrono::Duration::days(5)),
                description: None,
            },
            1,
        )
        .await
        .unwrap();

        let report = svc
            .generate_variance_report(
                center.id,
                1,
                Some(now),
                Some(now + chrono::Duration::days(30)),
            )
            .await
            .unwrap();
        assert_eq!(report.total_budgeted, Decimal::new(10000, 0));
        assert_eq!(report.total_actual, Decimal::new(3000, 0));
        assert_eq!(report.total_variance, Decimal::new(7000, 0));
        assert_eq!(report.lines.len(), 1);
    }

    #[tokio::test]
    async fn test_allocation_rule_crud() {
        let svc = make_service();

        let center = svc
            .create_cost_center(
                CreateCostCenter {
                    code: "CC-001".to_string(),
                    name: "Production".to_string(),
                    description: None,
                    center_type: CostCenterType::Cost,
                    parent_id: None,
                    is_active: true,
                },
                1,
            )
            .await
            .unwrap();

        let rule = svc
            .create_allocation_rule(
                CreateAllocationRule {
                    name: "Payroll Split".to_string(),
                    source_type: "payroll".to_string(),
                    account_range_start: Some("6000".to_string()),
                    account_range_end: Some("6999".to_string()),
                    cost_center_id: center.id,
                    rule_type: crate::domain::cost_center::model::AllocationRuleType::Percentage,
                    percentage: Some(Decimal::new(50, 0)),
                    fixed_amount: None,
                    is_active: true,
                    priority: 1,
                },
                1,
            )
            .await
            .unwrap();
        assert_eq!(rule.name, "Payroll Split");

        let found = svc.get_allocation_rule(rule.id, 1).await.unwrap();
        assert_eq!(found.id, rule.id);

        let rules = svc.list_allocation_rules(center.id, 1).await.unwrap();
        assert_eq!(rules.len(), 1);

        let updated = svc
            .update_allocation_rule(
                rule.id,
                1,
                UpdateAllocationRule {
                    percentage: Some(Some(Decimal::new(75, 0))),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.percentage, Some(Decimal::new(75, 0)));

        svc.delete_allocation_rule(rule.id, 1).await.unwrap();
        assert!(svc.get_allocation_rule(rule.id, 1).await.is_err());
    }

    #[tokio::test]
    async fn test_allocation_rule_validation() {
        let svc = make_service();

        let center = svc
            .create_cost_center(
                CreateCostCenter {
                    code: "CC-001".to_string(),
                    name: "Production".to_string(),
                    description: None,
                    center_type: CostCenterType::Cost,
                    parent_id: None,
                    is_active: true,
                },
                1,
            )
            .await
            .unwrap();

        let missing_pct = CreateAllocationRule {
            name: "Split".to_string(),
            source_type: "payroll".to_string(),
            account_range_start: None,
            account_range_end: None,
            cost_center_id: center.id,
            rule_type: crate::domain::cost_center::model::AllocationRuleType::Percentage,
            percentage: None,
            fixed_amount: None,
            is_active: true,
            priority: 0,
        };
        assert!(svc.create_allocation_rule(missing_pct, 1).await.is_err());

        let bad_pct = CreateAllocationRule {
            name: "Split".to_string(),
            source_type: "payroll".to_string(),
            account_range_start: None,
            account_range_end: None,
            cost_center_id: center.id,
            rule_type: crate::domain::cost_center::model::AllocationRuleType::Percentage,
            percentage: Some(Decimal::new(101, 0)),
            fixed_amount: None,
            is_active: true,
            priority: 0,
        };
        assert!(svc.create_allocation_rule(bad_pct, 1).await.is_err());
    }

    #[tokio::test]
    async fn test_apply_allocation_rules() {
        let svc = make_service();

        let center = svc
            .create_cost_center(
                CreateCostCenter {
                    code: "CC-001".to_string(),
                    name: "Production".to_string(),
                    description: None,
                    center_type: CostCenterType::Cost,
                    parent_id: None,
                    is_active: true,
                },
                1,
            )
            .await
            .unwrap();

        svc.create_allocation_rule(
            CreateAllocationRule {
                name: "Payroll Split".to_string(),
                source_type: "payroll".to_string(),
                account_range_start: None,
                account_range_end: None,
                cost_center_id: center.id,
                rule_type: crate::domain::cost_center::model::AllocationRuleType::Percentage,
                percentage: Some(Decimal::new(50, 0)),
                fixed_amount: None,
                is_active: true,
                priority: 1,
            },
            1,
        )
        .await
        .unwrap();

        let allocs = svc
            .apply_allocation_rules("payroll", 1, Decimal::new(10000, 0), Some("6100"), 1)
            .await
            .unwrap();
        assert_eq!(allocs.len(), 1);
        assert_eq!(allocs[0].amount, Decimal::new(5000, 0));

        // No match for different source type
        let none = svc
            .apply_allocation_rules("invoice", 1, Decimal::new(10000, 0), None, 1)
            .await
            .unwrap();
        assert_eq!(none.len(), 0);
    }
}
