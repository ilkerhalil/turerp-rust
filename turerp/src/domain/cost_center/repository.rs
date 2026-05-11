//! Cost Center repository traits and in-memory implementations

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::common::soft_delete::SoftDeletable;
use crate::domain::cost_center::model::{
    AllocationRule, Budget, CostCenter, CostCenterAllocation, CostCenterType, CreateAllocation,
    CreateAllocationRule, CreateBudget, CreateCostCenter, ProfitabilityReport,
    UpdateAllocationRule, UpdateBudget, UpdateCostCenter, VarianceReport, VarianceReportLine,
};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// CostCenterRepository
// ---------------------------------------------------------------------------

/// Repository trait for cost center operations
#[async_trait]
pub trait CostCenterRepository: Send + Sync {
    /// Create a new cost center
    async fn create(
        &self,
        create: CreateCostCenter,
        tenant_id: i64,
    ) -> Result<CostCenter, ApiError>;

    /// Find a cost center by ID
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<CostCenter>, ApiError>;

    /// Find all cost centers with optional type filter and pagination
    async fn find_all(
        &self,
        tenant_id: i64,
        center_type: Option<CostCenterType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<CostCenter>, ApiError>;

    /// Find cost centers by type
    async fn find_by_type(
        &self,
        tenant_id: i64,
        center_type: CostCenterType,
    ) -> Result<Vec<CostCenter>, ApiError>;

    /// Update a cost center
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCostCenter,
    ) -> Result<CostCenter, ApiError>;

    /// Soft delete a cost center
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    /// Restore a soft-deleted cost center
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<CostCenter, ApiError>;

    /// Find soft-deleted cost centers (admin use)
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<CostCenter>, ApiError>;

    /// Hard delete a cost center (permanent destruction — admin only)
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Create an allocation
    async fn create_allocation(
        &self,
        allocation: CreateAllocation,
        tenant_id: i64,
    ) -> Result<CostCenterAllocation, ApiError>;

    /// Get allocations for a cost center
    async fn get_allocations(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<CostCenterAllocation>, ApiError>;

    /// Get profitability report for a cost center
    async fn get_profitability_report(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
        period_start: Option<DateTime<Utc>>,
        period_end: Option<DateTime<Utc>>,
    ) -> Result<ProfitabilityReport, ApiError>;

    // ---- Budget Operations ----

    /// Create a budget
    async fn create_budget(&self, budget: CreateBudget, tenant_id: i64)
        -> Result<Budget, ApiError>;

    /// Find a budget by ID
    async fn find_budget_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Budget>, ApiError>;

    /// Find budgets for a cost center
    async fn find_budgets_by_cost_center(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Budget>, ApiError>;

    /// Update a budget
    async fn update_budget(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateBudget,
    ) -> Result<Budget, ApiError>;

    /// Delete a budget
    async fn delete_budget(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    /// Get variance report for a cost center
    async fn get_variance_report(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
        period_start: Option<DateTime<Utc>>,
        period_end: Option<DateTime<Utc>>,
    ) -> Result<VarianceReport, ApiError>;

    // ---- Allocation Rule Operations ----

    /// Create an allocation rule
    async fn create_allocation_rule(
        &self,
        rule: CreateAllocationRule,
        tenant_id: i64,
    ) -> Result<AllocationRule, ApiError>;

    /// Find an allocation rule by ID
    async fn find_allocation_rule_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<AllocationRule>, ApiError>;

    /// Find active allocation rules for a source type
    async fn find_allocation_rules_by_source(
        &self,
        source_type: &str,
        tenant_id: i64,
    ) -> Result<Vec<AllocationRule>, ApiError>;

    /// Find allocation rules for a cost center
    async fn find_allocation_rules_by_cost_center(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<AllocationRule>, ApiError>;

    /// Update an allocation rule
    async fn update_allocation_rule(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateAllocationRule,
    ) -> Result<AllocationRule, ApiError>;

    /// Delete an allocation rule
    async fn delete_allocation_rule(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type alias for boxed CostCenterRepository
pub type BoxCostCenterRepository = Arc<dyn CostCenterRepository>;

// ---------------------------------------------------------------------------
// InMemoryCostCenterRepository
// ---------------------------------------------------------------------------

struct CostCenterInner {
    centers: HashMap<i64, CostCenter>,
    allocations: HashMap<i64, CostCenterAllocation>,
    budgets: HashMap<i64, Budget>,
    rules: HashMap<i64, AllocationRule>,
    next_center_id: AtomicI64,
    next_allocation_id: AtomicI64,
    next_budget_id: AtomicI64,
    next_rule_id: AtomicI64,
}

/// In-memory cost center repository for testing and development
pub struct InMemoryCostCenterRepository {
    inner: Mutex<CostCenterInner>,
}

impl InMemoryCostCenterRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(CostCenterInner {
                centers: HashMap::new(),
                allocations: HashMap::new(),
                budgets: HashMap::new(),
                rules: HashMap::new(),
                next_center_id: AtomicI64::new(1),
                next_allocation_id: AtomicI64::new(1),
                next_budget_id: AtomicI64::new(1),
                next_rule_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryCostCenterRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CostCenterRepository for InMemoryCostCenterRepository {
    async fn create(
        &self,
        create: CreateCostCenter,
        tenant_id: i64,
    ) -> Result<CostCenter, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_center_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now();

        let center = CostCenter {
            id,
            tenant_id,
            code: create.code,
            name: create.name,
            description: create.description,
            center_type: create.center_type,
            parent_id: create.parent_id,
            is_active: create.is_active,
            created_at: now,
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };

        inner.centers.insert(id, center.clone());
        Ok(center)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<CostCenter>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .centers
            .get(&id)
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .cloned())
    }

    async fn find_all(
        &self,
        tenant_id: i64,
        center_type: Option<CostCenterType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<CostCenter>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<CostCenter> = inner
            .centers
            .values()
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .filter(|c| match &center_type {
                Some(ct) => c.center_type == *ct,
                None => true,
            })
            .cloned()
            .collect();

        items.sort_by(|a, b| a.code.cmp(&b.code));
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<CostCenter> = items
            .into_iter()
            .skip(start as usize)
            .take(params.per_page as usize)
            .collect();
        Ok(PaginatedResult::new(
            paginated,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn find_by_type(
        &self,
        tenant_id: i64,
        center_type: CostCenterType,
    ) -> Result<Vec<CostCenter>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<CostCenter> = inner
            .centers
            .values()
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .filter(|c| c.center_type == center_type)
            .cloned()
            .collect();
        items.sort_by(|a, b| a.code.cmp(&b.code));
        Ok(items)
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCostCenter,
    ) -> Result<CostCenter, ApiError> {
        let mut inner = self.inner.lock();

        let center = inner
            .centers
            .get_mut(&id)
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .ok_or_else(|| ApiError::NotFound(format!("Cost center {} not found", id)))?;

        if let Some(code) = update.code {
            center.code = code;
        }
        if let Some(name) = update.name {
            center.name = name;
        }
        if let Some(description) = update.description {
            center.description = description;
        }
        if let Some(center_type) = update.center_type {
            center.center_type = center_type;
        }
        if let Some(parent_id) = update.parent_id {
            center.parent_id = parent_id;
        }
        if let Some(is_active) = update.is_active {
            center.is_active = is_active;
        }
        center.updated_at = Some(chrono::Utc::now());

        Ok(center.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let center = inner
            .centers
            .get_mut(&id)
            .filter(|c| c.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Cost center {} not found", id)))?;

        if center.is_deleted() {
            return Err(ApiError::Conflict(format!(
                "Cost center {} is already deleted",
                id
            )));
        }

        center.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<CostCenter, ApiError> {
        let mut inner = self.inner.lock();

        let center = inner
            .centers
            .get_mut(&id)
            .filter(|c| c.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Cost center {} not found", id)))?;

        if !center.is_deleted() {
            return Err(ApiError::BadRequest(format!(
                "Cost center {} is not deleted",
                id
            )));
        }

        center.restore();
        Ok(center.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<CostCenter>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<CostCenter> = inner
            .centers
            .values()
            .filter(|c| c.tenant_id == tenant_id && c.is_deleted())
            .cloned()
            .collect();
        items.sort_by_key(|a| a.deleted_at);
        Ok(items)
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let len_before = inner.centers.len();
        inner
            .centers
            .retain(|_, c| !(c.id == id && c.tenant_id == tenant_id && c.is_deleted()));

        if inner.centers.len() == len_before {
            return Err(ApiError::NotFound(format!(
                "Deleted cost center {} not found",
                id
            )));
        }
        Ok(())
    }

    async fn create_allocation(
        &self,
        allocation: CreateAllocation,
        tenant_id: i64,
    ) -> Result<CostCenterAllocation, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_allocation_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now();

        let alloc = CostCenterAllocation {
            id,
            tenant_id,
            source_type: allocation.source_type,
            source_id: allocation.source_id,
            cost_center_id: allocation.cost_center_id,
            amount: allocation.amount,
            percentage: allocation.percentage,
            allocation_date: allocation.allocation_date.unwrap_or(now),
            description: allocation.description,
            created_at: now,
        };

        inner.allocations.insert(id, alloc.clone());
        Ok(alloc)
    }

    async fn get_allocations(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<CostCenterAllocation>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<CostCenterAllocation> = inner
            .allocations
            .values()
            .filter(|a| a.tenant_id == tenant_id && a.cost_center_id == cost_center_id)
            .cloned()
            .collect();
        items.sort_by_key(|b| std::cmp::Reverse(b.allocation_date));
        Ok(items)
    }

    async fn get_profitability_report(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
        period_start: Option<DateTime<Utc>>,
        period_end: Option<DateTime<Utc>>,
    ) -> Result<ProfitabilityReport, ApiError> {
        let inner = self.inner.lock();

        let center = inner
            .centers
            .get(&cost_center_id)
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .ok_or_else(|| {
                ApiError::NotFound(format!("Cost center {} not found", cost_center_id))
            })?;

        let allocations: Vec<&CostCenterAllocation> = inner
            .allocations
            .values()
            .filter(|a| a.tenant_id == tenant_id && a.cost_center_id == cost_center_id)
            .filter(|a| {
                if let Some(start) = period_start {
                    a.allocation_date >= start
                } else {
                    true
                }
            })
            .filter(|a| {
                if let Some(end) = period_end {
                    a.allocation_date <= end
                } else {
                    true
                }
            })
            .collect();

        let mut total_income = Decimal::ZERO;
        let mut total_expense = Decimal::ZERO;

        for alloc in &allocations {
            match alloc.source_type.as_str() {
                "invoice" | "sales" => {
                    total_income += alloc.amount;
                }
                _ => {
                    total_expense += alloc.amount;
                }
            }
        }

        let net_profit = total_income - total_expense;

        Ok(ProfitabilityReport {
            cost_center_id: center.id,
            cost_center_code: center.code.clone(),
            cost_center_name: center.name.clone(),
            center_type: center.center_type.clone(),
            total_income,
            total_expense,
            net_profit,
            allocation_count: allocations.len() as i64,
            period_start,
            period_end,
        })
    }

    // ---- Budget Operations ----

    async fn create_budget(
        &self,
        budget: CreateBudget,
        tenant_id: i64,
    ) -> Result<Budget, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_budget_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now();

        let b = Budget {
            id,
            tenant_id,
            cost_center_id: budget.cost_center_id,
            period: budget.period,
            period_start: budget.period_start,
            period_end: budget.period_end,
            budgeted_amount: budget.budgeted_amount,
            actual_amount: Decimal::ZERO,
            variance: Decimal::ZERO,
            notes: budget.notes,
            created_at: now,
            updated_at: None,
        };

        inner.budgets.insert(id, b.clone());
        Ok(b)
    }

    async fn find_budget_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Budget>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .budgets
            .get(&id)
            .filter(|b| b.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_budgets_by_cost_center(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Budget>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<Budget> = inner
            .budgets
            .values()
            .filter(|b| b.tenant_id == tenant_id && b.cost_center_id == cost_center_id)
            .cloned()
            .collect();
        items.sort_by_key(|b| b.period_start);
        Ok(items)
    }

    async fn update_budget(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateBudget,
    ) -> Result<Budget, ApiError> {
        let mut inner = self.inner.lock();

        let budget = inner
            .budgets
            .get_mut(&id)
            .filter(|b| b.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Budget {} not found", id)))?;

        if let Some(period) = update.period {
            budget.period = period;
        }
        if let Some(start) = update.period_start {
            budget.period_start = start;
        }
        if let Some(end) = update.period_end {
            budget.period_end = end;
        }
        if let Some(amount) = update.budgeted_amount {
            budget.budgeted_amount = amount;
        }
        if let Some(notes) = update.notes {
            budget.notes = notes;
        }
        budget.updated_at = Some(chrono::Utc::now());

        Ok(budget.clone())
    }

    async fn delete_budget(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let existed = inner
            .budgets
            .remove(&id)
            .filter(|b| b.tenant_id == tenant_id)
            .is_some();
        if !existed {
            return Err(ApiError::NotFound(format!("Budget {} not found", id)));
        }
        Ok(())
    }

    async fn get_variance_report(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
        period_start: Option<DateTime<Utc>>,
        period_end: Option<DateTime<Utc>>,
    ) -> Result<VarianceReport, ApiError> {
        let inner = self.inner.lock();

        let center = inner
            .centers
            .get(&cost_center_id)
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .ok_or_else(|| {
                ApiError::NotFound(format!("Cost center {} not found", cost_center_id))
            })?;

        let budgets: Vec<&Budget> = inner
            .budgets
            .values()
            .filter(|b| b.tenant_id == tenant_id && b.cost_center_id == cost_center_id)
            .filter(|b| {
                if let Some(start) = period_start {
                    b.period_start >= start
                } else {
                    true
                }
            })
            .filter(|b| {
                if let Some(end) = period_end {
                    b.period_end <= end
                } else {
                    true
                }
            })
            .collect();

        let allocations: Vec<&CostCenterAllocation> = inner
            .allocations
            .values()
            .filter(|a| a.tenant_id == tenant_id && a.cost_center_id == cost_center_id)
            .filter(|a| {
                if let Some(start) = period_start {
                    a.allocation_date >= start
                } else {
                    true
                }
            })
            .filter(|a| {
                if let Some(end) = period_end {
                    a.allocation_date <= end
                } else {
                    true
                }
            })
            .collect();

        let mut total_budgeted = Decimal::ZERO;
        let mut total_actual = Decimal::ZERO;
        let mut lines = Vec::new();

        for budget in budgets {
            let actual: Decimal = allocations
                .iter()
                .filter(|a| {
                    a.allocation_date >= budget.period_start
                        && a.allocation_date <= budget.period_end
                })
                .map(|a| a.amount)
                .sum();
            let variance = budget.budgeted_amount - actual;
            let variance_percentage = if budget.budgeted_amount > Decimal::ZERO {
                (variance / budget.budgeted_amount) * Decimal::new(100, 0)
            } else {
                Decimal::ZERO
            };

            total_budgeted += budget.budgeted_amount;
            total_actual += actual;

            lines.push(VarianceReportLine {
                cost_center_id: center.id,
                cost_center_code: center.code.clone(),
                cost_center_name: center.name.clone(),
                period: budget.period.clone(),
                period_start: budget.period_start,
                period_end: budget.period_end,
                budgeted_amount: budget.budgeted_amount,
                actual_amount: actual,
                variance,
                variance_percentage,
            });
        }

        let total_variance = total_budgeted - total_actual;

        Ok(VarianceReport {
            tenant_id,
            total_budgeted,
            total_actual,
            total_variance,
            lines,
        })
    }

    // ---- Allocation Rule Operations ----

    async fn create_allocation_rule(
        &self,
        rule: CreateAllocationRule,
        tenant_id: i64,
    ) -> Result<AllocationRule, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_rule_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now();

        let r = AllocationRule {
            id,
            tenant_id,
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
            created_at: now,
            updated_at: None,
        };

        inner.rules.insert(id, r.clone());
        Ok(r)
    }

    async fn find_allocation_rule_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<AllocationRule>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .rules
            .get(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_allocation_rules_by_source(
        &self,
        source_type: &str,
        tenant_id: i64,
    ) -> Result<Vec<AllocationRule>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<AllocationRule> = inner
            .rules
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.source_type == source_type && r.is_active)
            .cloned()
            .collect();
        items.sort_by_key(|r| (r.priority, r.id));
        Ok(items)
    }

    async fn find_allocation_rules_by_cost_center(
        &self,
        cost_center_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<AllocationRule>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<AllocationRule> = inner
            .rules
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.cost_center_id == cost_center_id)
            .cloned()
            .collect();
        items.sort_by_key(|r| (r.priority, r.id));
        Ok(items)
    }

    async fn update_allocation_rule(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateAllocationRule,
    ) -> Result<AllocationRule, ApiError> {
        let mut inner = self.inner.lock();

        let rule = inner
            .rules
            .get_mut(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Allocation rule {} not found", id)))?;

        if let Some(name) = update.name {
            rule.name = name;
        }
        if let Some(source_type) = update.source_type {
            rule.source_type = source_type;
        }
        if let Some(start) = update.account_range_start {
            rule.account_range_start = start;
        }
        if let Some(end) = update.account_range_end {
            rule.account_range_end = end;
        }
        if let Some(cost_center_id) = update.cost_center_id {
            rule.cost_center_id = cost_center_id;
        }
        if let Some(rule_type) = update.rule_type {
            rule.rule_type = rule_type;
        }
        if let Some(pct) = update.percentage {
            rule.percentage = pct;
        }
        if let Some(fixed) = update.fixed_amount {
            rule.fixed_amount = fixed;
        }
        if let Some(is_active) = update.is_active {
            rule.is_active = is_active;
        }
        if let Some(priority) = update.priority {
            rule.priority = priority;
        }
        rule.updated_at = Some(chrono::Utc::now());

        Ok(rule.clone())
    }

    async fn delete_allocation_rule(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let existed = inner
            .rules
            .remove(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .is_some();
        if !existed {
            return Err(ApiError::NotFound(format!(
                "Allocation rule {} not found",
                id
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cost_center_crud() {
        let repo = InMemoryCostCenterRepository::new();

        // Create
        let create = CreateCostCenter {
            code: "CC-001".to_string(),
            name: "Production".to_string(),
            description: Some("Main production".to_string()),
            center_type: CostCenterType::Cost,
            parent_id: None,
            is_active: true,
        };
        let center = repo.create(create, 1).await.unwrap();
        assert_eq!(center.id, 1);
        assert_eq!(center.code, "CC-001");

        // Find by ID
        let found = repo.find_by_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.id, center.id);

        // Not found for different tenant
        let not_found = repo.find_by_id(1, 999).await.unwrap();
        assert!(not_found.is_none());

        // Update
        let update = UpdateCostCenter {
            name: Some("Updated Production".to_string()),
            ..Default::default()
        };
        let updated = repo.update(1, 1, update).await.unwrap();
        assert_eq!(updated.name, "Updated Production");

        // Soft delete
        repo.soft_delete(1, 1, 1).await.unwrap();
        let gone = repo.find_by_id(1, 1).await.unwrap();
        assert!(gone.is_none());

        // Restore
        let restored = repo.restore(1, 1).await.unwrap();
        assert_eq!(restored.name, "Updated Production");

        // Find by type
        let costs = repo.find_by_type(1, CostCenterType::Cost).await.unwrap();
        assert_eq!(costs.len(), 1);
    }

    #[tokio::test]
    async fn test_allocations_and_profitability() {
        let repo = InMemoryCostCenterRepository::new();

        let create = CreateCostCenter {
            code: "PC-001".to_string(),
            name: "Sales East".to_string(),
            description: None,
            center_type: CostCenterType::Profit,
            parent_id: None,
            is_active: true,
        };
        let center = repo.create(create, 1).await.unwrap();

        // Create income allocation
        let income_alloc = CreateAllocation {
            source_type: "invoice".to_string(),
            source_id: 1,
            cost_center_id: center.id,
            amount: Decimal::new(5000, 0),
            percentage: Decimal::new(100, 0),
            allocation_date: None,
            description: None,
        };
        repo.create_allocation(income_alloc, 1).await.unwrap();

        // Create expense allocation
        let expense_alloc = CreateAllocation {
            source_type: "payroll".to_string(),
            source_id: 2,
            cost_center_id: center.id,
            amount: Decimal::new(2000, 0),
            percentage: Decimal::new(100, 0),
            allocation_date: None,
            description: None,
        };
        repo.create_allocation(expense_alloc, 1).await.unwrap();

        // Get allocations
        let allocs = repo.get_allocations(center.id, 1).await.unwrap();
        assert_eq!(allocs.len(), 2);

        // Profitability report
        let report = repo
            .get_profitability_report(center.id, 1, None, None)
            .await
            .unwrap();
        assert_eq!(report.total_income, Decimal::new(5000, 0));
        assert_eq!(report.total_expense, Decimal::new(2000, 0));
        assert_eq!(report.net_profit, Decimal::new(3000, 0));
        assert_eq!(report.allocation_count, 2);
    }

    #[tokio::test]
    async fn test_budget_crud() {
        let repo = InMemoryCostCenterRepository::new();

        let create = CreateCostCenter {
            code: "CC-001".to_string(),
            name: "Production".to_string(),
            description: None,
            center_type: CostCenterType::Cost,
            parent_id: None,
            is_active: true,
        };
        let center = repo.create(create, 1).await.unwrap();

        let budget_create = CreateBudget {
            cost_center_id: center.id,
            period: crate::domain::cost_center::model::BudgetPeriod::Monthly,
            period_start: Utc::now(),
            period_end: Utc::now(),
            budgeted_amount: Decimal::new(10000, 0),
            notes: Some("January budget".to_string()),
        };
        let budget = repo.create_budget(budget_create, 1).await.unwrap();
        assert_eq!(budget.budgeted_amount, Decimal::new(10000, 0));

        let found = repo.find_budget_by_id(budget.id, 1).await.unwrap().unwrap();
        assert_eq!(found.id, budget.id);

        let budgets = repo
            .find_budgets_by_cost_center(center.id, 1)
            .await
            .unwrap();
        assert_eq!(budgets.len(), 1);

        let update = UpdateBudget {
            budgeted_amount: Some(Decimal::new(12000, 0)),
            ..Default::default()
        };
        let updated = repo.update_budget(budget.id, 1, update).await.unwrap();
        assert_eq!(updated.budgeted_amount, Decimal::new(12000, 0));

        repo.delete_budget(budget.id, 1).await.unwrap();
        let gone = repo.find_budget_by_id(budget.id, 1).await.unwrap();
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn test_variance_report() {
        let repo = InMemoryCostCenterRepository::new();

        let create = CreateCostCenter {
            code: "CC-001".to_string(),
            name: "Production".to_string(),
            description: None,
            center_type: CostCenterType::Cost,
            parent_id: None,
            is_active: true,
        };
        let center = repo.create(create, 1).await.unwrap();

        let now = Utc::now();
        let budget_create = CreateBudget {
            cost_center_id: center.id,
            period: crate::domain::cost_center::model::BudgetPeriod::Monthly,
            period_start: now,
            period_end: now + chrono::Duration::days(30),
            budgeted_amount: Decimal::new(10000, 0),
            notes: None,
        };
        repo.create_budget(budget_create, 1).await.unwrap();

        let alloc = CreateAllocation {
            source_type: "payroll".to_string(),
            source_id: 1,
            cost_center_id: center.id,
            amount: Decimal::new(3000, 0),
            percentage: Decimal::new(100, 0),
            allocation_date: Some(now + chrono::Duration::days(5)),
            description: None,
        };
        repo.create_allocation(alloc, 1).await.unwrap();

        let report = repo
            .get_variance_report(
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
        assert_eq!(report.lines[0].variance, Decimal::new(7000, 0));
    }

    #[tokio::test]
    async fn test_allocation_rule_crud() {
        let repo = InMemoryCostCenterRepository::new();

        let create = CreateCostCenter {
            code: "CC-001".to_string(),
            name: "Production".to_string(),
            description: None,
            center_type: CostCenterType::Cost,
            parent_id: None,
            is_active: true,
        };
        let center = repo.create(create, 1).await.unwrap();

        let rule_create = CreateAllocationRule {
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
        };
        let rule = repo.create_allocation_rule(rule_create, 1).await.unwrap();
        assert_eq!(rule.name, "Payroll Split");
        assert_eq!(rule.percentage, Some(Decimal::new(50, 0)));

        let found = repo
            .find_allocation_rule_by_id(rule.id, 1)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id, rule.id);

        let by_source = repo
            .find_allocation_rules_by_source("payroll", 1)
            .await
            .unwrap();
        assert_eq!(by_source.len(), 1);

        let by_center = repo
            .find_allocation_rules_by_cost_center(center.id, 1)
            .await
            .unwrap();
        assert_eq!(by_center.len(), 1);

        let update = UpdateAllocationRule {
            percentage: Some(Some(Decimal::new(75, 0))),
            ..Default::default()
        };
        let updated = repo
            .update_allocation_rule(rule.id, 1, update)
            .await
            .unwrap();
        assert_eq!(updated.percentage, Some(Decimal::new(75, 0)));

        repo.delete_allocation_rule(rule.id, 1).await.unwrap();
        let gone = repo.find_allocation_rule_by_id(rule.id, 1).await.unwrap();
        assert!(gone.is_none());
    }
}
