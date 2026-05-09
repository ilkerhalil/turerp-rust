//! Cost Center service — business logic for cost center management

use chrono::{DateTime, Utc};

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::cost_center::model::{
    BulkRestoreFailed, BulkRestoreResponse, CostCenter, CostCenterAllocation, CostCenterResponse,
    CostCenterType, CreateAllocation, CreateCostCenter, ProfitabilityReport, UpdateCostCenter,
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
}
