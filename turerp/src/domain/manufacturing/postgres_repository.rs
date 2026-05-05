//! PostgreSQL manufacturing repository implementation

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::manufacturing::model::{
    BillOfMaterials, BillOfMaterialsLine, CreateBillOfMaterials, CreateBillOfMaterialsLine,
    CreateInspection, CreateNonConformanceReport, CreateRouting, CreateRoutingOperation,
    CreateWorkOrder, CreateWorkOrderMaterial, CreateWorkOrderOperation, Inspection,
    InspectionStatus, NcrStatus, NcrType, NonConformanceReport, Routing, RoutingOperation,
    UpdateInspection, UpdateNonConformanceReport, WorkOrder, WorkOrderMaterial, WorkOrderOperation,
    WorkOrderPriority, WorkOrderStatus,
};
use crate::domain::manufacturing::repository::{
    BillOfMaterialsRepository, BoxBillOfMaterialsRepository, BoxInspectionRepository,
    BoxNcrRepository, BoxRoutingRepository, BoxWorkOrderRepository, InspectionRepository,
    NcrRepository, RoutingRepository, WorkOrderRepository,
};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

/// Convert WorkOrderStatus to its database string representation
fn work_order_status_to_str(status: &WorkOrderStatus) -> &'static str {
    match status {
        WorkOrderStatus::Draft => "Draft",
        WorkOrderStatus::Scheduled => "Scheduled",
        WorkOrderStatus::InProgress => "InProgress",
        WorkOrderStatus::OnHold => "OnHold",
        WorkOrderStatus::Completed => "Completed",
        WorkOrderStatus::Cancelled => "Cancelled",
    }
}

/// Parse a WorkOrderStatus from its database string representation
fn parse_work_order_status(s: &str) -> Result<WorkOrderStatus, String> {
    match s {
        "Draft" => Ok(WorkOrderStatus::Draft),
        "Scheduled" => Ok(WorkOrderStatus::Scheduled),
        "InProgress" => Ok(WorkOrderStatus::InProgress),
        "OnHold" => Ok(WorkOrderStatus::OnHold),
        "Completed" => Ok(WorkOrderStatus::Completed),
        "Cancelled" => Ok(WorkOrderStatus::Cancelled),
        _ => Err(format!("Invalid work order status: {}", s)),
    }
}

/// Convert WorkOrderPriority to its database string representation
fn work_order_priority_to_str(priority: &WorkOrderPriority) -> &'static str {
    match priority {
        WorkOrderPriority::Low => "Low",
        WorkOrderPriority::Normal => "Normal",
        WorkOrderPriority::High => "High",
        WorkOrderPriority::Urgent => "Urgent",
    }
}

/// Parse a WorkOrderPriority from its database string representation
fn parse_work_order_priority(s: &str) -> Result<WorkOrderPriority, String> {
    match s {
        "Low" => Ok(WorkOrderPriority::Low),
        "Normal" => Ok(WorkOrderPriority::Normal),
        "High" => Ok(WorkOrderPriority::High),
        "Urgent" => Ok(WorkOrderPriority::Urgent),
        _ => Err(format!("Invalid work order priority: {}", s)),
    }
}

// ==================== WORK ORDER ====================

/// Database row representation for WorkOrder
#[derive(Debug, FromRow)]
struct WorkOrderRow {
    id: i64,
    tenant_id: i64,
    name: String,
    product_id: i64,
    quantity: Decimal,
    bom_id: Option<i64>,
    routing_id: Option<i64>,
    status: String,
    priority: String,
    planned_start: Option<chrono::DateTime<chrono::Utc>>,
    planned_end: Option<chrono::DateTime<chrono::Utc>>,
    actual_start: Option<chrono::DateTime<chrono::Utc>>,
    actual_end: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    total_count: Option<i64>,
}

impl From<WorkOrderRow> for WorkOrder {
    fn from(row: WorkOrderRow) -> Self {
        let status = parse_work_order_status(&row.status).unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid work order status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            WorkOrderStatus::Draft
        });
        let priority = parse_work_order_priority(&row.priority).unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid work order priority '{}' in database: {}, defaulting to Normal",
                row.priority,
                e
            );
            WorkOrderPriority::Normal
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            product_id: row.product_id,
            quantity: row.quantity,
            bom_id: row.bom_id,
            routing_id: row.routing_id,
            status,
            priority,
            planned_start: row.planned_start,
            planned_end: row.planned_end,
            actual_start: row.actual_start,
            actual_end: row.actual_end,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: None,
            deleted_by: None,
        }
    }
}

/// Database row representation for WorkOrderOperation
#[derive(Debug, FromRow)]
struct WorkOrderOperationRow {
    id: i64,
    work_order_id: i64,
    operation_sequence: i32,
    operation_name: String,
    work_center_id: Option<i64>,
    planned_hours: Decimal,
    actual_hours: Decimal,
    status: String,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<WorkOrderOperationRow> for WorkOrderOperation {
    fn from(row: WorkOrderOperationRow) -> Self {
        Self {
            id: row.id,
            work_order_id: row.work_order_id,
            operation_sequence: row.operation_sequence,
            operation_name: row.operation_name,
            work_center_id: row.work_center_id,
            planned_hours: row.planned_hours,
            actual_hours: row.actual_hours,
            status: row.status,
            started_at: row.started_at,
            completed_at: row.completed_at,
        }
    }
}

/// Database row representation for WorkOrderMaterial
#[derive(Debug, FromRow)]
struct WorkOrderMaterialRow {
    id: i64,
    work_order_id: i64,
    product_id: i64,
    quantity_required: Decimal,
    quantity_issued: Decimal,
    is_issued: bool,
}

impl From<WorkOrderMaterialRow> for WorkOrderMaterial {
    fn from(row: WorkOrderMaterialRow) -> Self {
        Self {
            id: row.id,
            work_order_id: row.work_order_id,
            product_id: row.product_id,
            quantity_required: row.quantity_required,
            quantity_issued: row.quantity_issued,
            is_issued: row.is_issued,
        }
    }
}

/// PostgreSQL work order repository
pub struct PostgresWorkOrderRepository {
    pool: Arc<PgPool>,
}

impl PostgresWorkOrderRepository {
    /// Create a new PostgreSQL work order repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxWorkOrderRepository {
        Arc::new(self) as BoxWorkOrderRepository
    }
}

#[async_trait]
impl WorkOrderRepository for PostgresWorkOrderRepository {
    async fn create(&self, create: CreateWorkOrder) -> Result<WorkOrder, ApiError> {
        let status_str = work_order_status_to_str(&WorkOrderStatus::Draft);
        let priority_str = work_order_priority_to_str(&create.priority);

        let row: WorkOrderRow = sqlx::query_as(
            r#"
            INSERT INTO work_orders (tenant_id, name, product_id, quantity, bom_id, routing_id,
                                     status, priority, planned_start, planned_end,
                                     actual_start, actual_end, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL, NULL, NOW(), NOW())
            RETURNING id, tenant_id, name, product_id, quantity, bom_id, routing_id,
                      status, priority, planned_start, planned_end,
                      actual_start, actual_end, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.name)
        .bind(create.product_id)
        .bind(create.quantity)
        .bind(create.bom_id)
        .bind(create.routing_id)
        .bind(status_str)
        .bind(priority_str)
        .bind(create.planned_start)
        .bind(create.planned_end)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkOrder"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<WorkOrder>, ApiError> {
        let result: Option<WorkOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,
                   status, priority, planned_start, planned_end,
                   actual_start, actual_end, created_at, updated_at
            FROM work_orders
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find work order by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<WorkOrder>, ApiError> {
        let rows: Vec<WorkOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,
                   status, priority, planned_start, planned_end,
                   actual_start, actual_end, created_at, updated_at
            FROM work_orders
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find work orders by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<WorkOrder>, ApiError> {
        let offset = page.saturating_sub(1) * per_page;

        let rows: Vec<WorkOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,
                   status, priority, planned_start, planned_end,
                   actual_start, actual_end, created_at, updated_at,
                   COUNT(*) OVER() as total_count
            FROM work_orders
            WHERE tenant_id = $1
            ORDER BY id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find work orders by tenant paginated: {}",
                e
            ))
        })?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<WorkOrder> = rows.into_iter().map(|r| r.into()).collect();

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<WorkOrder>, ApiError> {
        let rows: Vec<WorkOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,
                   status, priority, planned_start, planned_end,
                   actual_start, actual_end, created_at, updated_at
            FROM work_orders
            WHERE product_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(product_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find work orders by product: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_status(
        &self,
        tenant_id: i64,
        status: WorkOrderStatus,
    ) -> Result<Vec<WorkOrder>, ApiError> {
        let status_str = work_order_status_to_str(&status);

        let rows: Vec<WorkOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,
                   status, priority, planned_start, planned_end,
                   actual_start, actual_end, created_at, updated_at
            FROM work_orders
            WHERE tenant_id = $1 AND status = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(status_str)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find work orders by status: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(&self, id: i64, status: WorkOrderStatus) -> Result<WorkOrder, ApiError> {
        let status_str = work_order_status_to_str(&status);

        let row: WorkOrderRow = sqlx::query_as(
            r#"
            UPDATE work_orders
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, tenant_id, name, product_id, quantity, bom_id, routing_id,
                      status, priority, planned_start, planned_end,
                      actual_start, actual_end, created_at, updated_at
            "#,
        )
        .bind(status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkOrder"))?;

        Ok(row.into())
    }

    async fn add_operation(
        &self,
        op: CreateWorkOrderOperation,
    ) -> Result<WorkOrderOperation, ApiError> {
        let row: WorkOrderOperationRow = sqlx::query_as(
            r#"
            INSERT INTO work_order_operations (work_order_id, operation_sequence, operation_name,
                                               work_center_id, planned_hours, actual_hours, status,
                                               started_at, completed_at)
            VALUES ($1, $2, $3, $4, $5, 0, 'Pending', NULL, NULL)
            RETURNING id, work_order_id, operation_sequence, operation_name, work_center_id,
                      planned_hours, actual_hours, status, started_at, completed_at
            "#,
        )
        .bind(op.work_order_id)
        .bind(op.operation_sequence)
        .bind(&op.operation_name)
        .bind(op.work_center_id)
        .bind(op.planned_hours)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkOrderOperation"))?;

        Ok(row.into())
    }

    async fn get_operations(
        &self,
        work_order_id: i64,
    ) -> Result<Vec<WorkOrderOperation>, ApiError> {
        let rows: Vec<WorkOrderOperationRow> = sqlx::query_as(
            r#"
            SELECT id, work_order_id, operation_sequence, operation_name, work_center_id,
                   planned_hours, actual_hours, status, started_at, completed_at
            FROM work_order_operations
            WHERE work_order_id = $1
            ORDER BY operation_sequence
            "#,
        )
        .bind(work_order_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get work order operations: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn add_material(
        &self,
        mat: CreateWorkOrderMaterial,
    ) -> Result<WorkOrderMaterial, ApiError> {
        let row: WorkOrderMaterialRow = sqlx::query_as(
            r#"
            INSERT INTO work_order_materials (work_order_id, product_id, quantity_required,
                                              quantity_issued, is_issued)
            VALUES ($1, $2, $3, 0, false)
            RETURNING id, work_order_id, product_id, quantity_required, quantity_issued, is_issued
            "#,
        )
        .bind(mat.work_order_id)
        .bind(mat.product_id)
        .bind(mat.quantity_required)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkOrderMaterial"))?;

        Ok(row.into())
    }

    async fn get_materials(&self, work_order_id: i64) -> Result<Vec<WorkOrderMaterial>, ApiError> {
        let rows: Vec<WorkOrderMaterialRow> = sqlx::query_as(
            r#"
            SELECT id, work_order_id, product_id, quantity_required, quantity_issued, is_issued
            FROM work_order_materials
            WHERE work_order_id = $1
            "#,
        )
        .bind(work_order_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get work order materials: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE work_orders
            SET deleted_at = NOW(), deleted_by = $3, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete work order: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("WorkOrder not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<WorkOrder, ApiError> {
        let row: WorkOrderRow = sqlx::query_as(
            r#"
            UPDATE work_orders
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING id, tenant_id, name, product_id, quantity, bom_id, routing_id,
                      status, priority, planned_start, planned_end,
                      actual_start, actual_end, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "WorkOrder"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<WorkOrder>, ApiError> {
        let rows: Vec<WorkOrderRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, product_id, quantity, bom_id, routing_id,
                   status, priority, planned_start, planned_end,
                   actual_start, actual_end, created_at, updated_at
            FROM work_orders
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted work orders: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM work_orders
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy work order: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("WorkOrder not found".to_string()));
        }

        Ok(())
    }
}

// ==================== BILL OF MATERIALS ====================

/// Database row representation for BillOfMaterials
#[derive(Debug, FromRow)]
struct BillOfMaterialsRow {
    id: i64,
    tenant_id: i64,
    product_id: i64,
    version: String,
    is_active: bool,
    is_primary: bool,
    valid_from: Option<chrono::DateTime<chrono::Utc>>,
    valid_to: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<BillOfMaterialsRow> for BillOfMaterials {
    fn from(row: BillOfMaterialsRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            product_id: row.product_id,
            version: row.version,
            is_active: row.is_active,
            is_primary: row.is_primary,
            valid_from: row.valid_from,
            valid_to: row.valid_to,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: None,
            deleted_by: None,
        }
    }
}

/// Database row representation for BillOfMaterialsLine
#[derive(Debug, FromRow)]
struct BillOfMaterialsLineRow {
    id: i64,
    bom_id: i64,
    component_product_id: i64,
    quantity: Decimal,
    unit_id: Option<i64>,
    scrap_percentage: Decimal,
    is_optional: bool,
    notes: Option<String>,
}

impl From<BillOfMaterialsLineRow> for BillOfMaterialsLine {
    fn from(row: BillOfMaterialsLineRow) -> Self {
        Self {
            id: row.id,
            bom_id: row.bom_id,
            component_product_id: row.component_product_id,
            quantity: row.quantity,
            unit_id: row.unit_id,
            scrap_percentage: row.scrap_percentage,
            is_optional: row.is_optional,
            notes: row.notes,
        }
    }
}

/// PostgreSQL bill of materials repository
pub struct PostgresBillOfMaterialsRepository {
    pool: Arc<PgPool>,
}

impl PostgresBillOfMaterialsRepository {
    /// Create a new PostgreSQL bill of materials repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxBillOfMaterialsRepository {
        Arc::new(self) as BoxBillOfMaterialsRepository
    }
}

#[async_trait]
impl BillOfMaterialsRepository for PostgresBillOfMaterialsRepository {
    async fn create(&self, create: CreateBillOfMaterials) -> Result<BillOfMaterials, ApiError> {
        let row: BillOfMaterialsRow = sqlx::query_as(
            r#"
            INSERT INTO bills_of_materials (tenant_id, product_id, version, is_active, is_primary,
                                            valid_from, valid_to, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            RETURNING id, tenant_id, product_id, version, is_active, is_primary,
                      valid_from, valid_to, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(create.product_id)
        .bind(&create.version)
        .bind(create.is_active)
        .bind(create.is_primary)
        .bind(create.valid_from)
        .bind(create.valid_to)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "BillOfMaterials"))?;

        Ok(row.into())
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<BillOfMaterials>, ApiError> {
        let result: Option<BillOfMaterialsRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, product_id, version, is_active, is_primary,
                   valid_from, valid_to, created_at, updated_at
            FROM bills_of_materials
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find bill of materials by id: {}", e))
        })?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<BillOfMaterials>, ApiError> {
        let rows: Vec<BillOfMaterialsRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, product_id, version, is_active, is_primary,
                   valid_from, valid_to, created_at, updated_at
            FROM bills_of_materials
            WHERE product_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(product_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find bills of materials by product: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_primary_by_product(
        &self,
        product_id: i64,
    ) -> Result<Option<BillOfMaterials>, ApiError> {
        let result: Option<BillOfMaterialsRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, product_id, version, is_active, is_primary,
                   valid_from, valid_to, created_at, updated_at
            FROM bills_of_materials
            WHERE product_id = $1 AND is_primary = true AND is_active = true
            LIMIT 1
            "#,
        )
        .bind(product_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find primary bill of materials by product: {}",
                e
            ))
        })?;

        Ok(result.map(|r| r.into()))
    }

    async fn add_line(
        &self,
        line: CreateBillOfMaterialsLine,
    ) -> Result<BillOfMaterialsLine, ApiError> {
        let row: BillOfMaterialsLineRow = sqlx::query_as(
            r#"
            INSERT INTO bom_lines (bom_id, component_product_id, quantity, unit_id,
                                  scrap_percentage, is_optional, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, bom_id, component_product_id, quantity, unit_id,
                      scrap_percentage, is_optional, notes
            "#,
        )
        .bind(line.bom_id)
        .bind(line.component_product_id)
        .bind(line.quantity)
        .bind(line.unit_id)
        .bind(line.scrap_percentage)
        .bind(line.is_optional)
        .bind(&line.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "BillOfMaterialsLine"))?;

        Ok(row.into())
    }

    async fn get_lines(&self, bom_id: i64) -> Result<Vec<BillOfMaterialsLine>, ApiError> {
        let rows: Vec<BillOfMaterialsLineRow> = sqlx::query_as(
            r#"
            SELECT id, bom_id, component_product_id, quantity, unit_id,
                   scrap_percentage, is_optional, notes
            FROM bom_lines
            WHERE bom_id = $1
            ORDER BY id
            "#,
        )
        .bind(bom_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get BOM lines: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE bills_of_materials
            SET deleted_at = NOW(), deleted_by = $3, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to soft delete bill of materials: {}", e))
        })?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("BillOfMaterials not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<BillOfMaterials, ApiError> {
        let row: BillOfMaterialsRow = sqlx::query_as(
            r#"
            UPDATE bills_of_materials
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING id, tenant_id, product_id, version, is_active, is_primary,
                      valid_from, valid_to, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "BillOfMaterials"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<BillOfMaterials>, ApiError> {
        let rows: Vec<BillOfMaterialsRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, product_id, version, is_active, is_primary,
                   valid_from, valid_to, created_at, updated_at
            FROM bills_of_materials
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find deleted bills of materials: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM bills_of_materials
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy bill of materials: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("BillOfMaterials not found".to_string()));
        }

        Ok(())
    }
}

// ==================== ROUTING ====================

/// Database row representation for Routing
#[derive(Debug, FromRow)]
struct RoutingRow {
    id: i64,
    tenant_id: i64,
    product_id: i64,
    version: String,
    is_active: bool,
    is_primary: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<RoutingRow> for Routing {
    fn from(row: RoutingRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            product_id: row.product_id,
            version: row.version,
            is_active: row.is_active,
            is_primary: row.is_primary,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: None,
            deleted_by: None,
        }
    }
}

/// Database row representation for RoutingOperation
#[derive(Debug, FromRow)]
struct RoutingOperationRow {
    id: i64,
    routing_id: i64,
    sequence: i32,
    operation_name: String,
    work_center_id: Option<i64>,
    setup_hours: Decimal,
    run_hours: Decimal,
    description: Option<String>,
}

impl From<RoutingOperationRow> for RoutingOperation {
    fn from(row: RoutingOperationRow) -> Self {
        Self {
            id: row.id,
            routing_id: row.routing_id,
            sequence: row.sequence,
            operation_name: row.operation_name,
            work_center_id: row.work_center_id,
            setup_hours: row.setup_hours,
            run_hours: row.run_hours,
            description: row.description,
        }
    }
}

/// PostgreSQL routing repository
pub struct PostgresRoutingRepository {
    pool: Arc<PgPool>,
}

impl PostgresRoutingRepository {
    /// Create a new PostgreSQL routing repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxRoutingRepository {
        Arc::new(self) as BoxRoutingRepository
    }
}

#[async_trait]
impl RoutingRepository for PostgresRoutingRepository {
    async fn create(&self, create: CreateRouting) -> Result<Routing, ApiError> {
        let row: RoutingRow = sqlx::query_as(
            r#"
            INSERT INTO routings (tenant_id, product_id, version, is_active, is_primary,
                                  created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
            RETURNING id, tenant_id, product_id, version, is_active, is_primary,
                      created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(create.product_id)
        .bind(&create.version)
        .bind(create.is_active)
        .bind(create.is_primary)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Routing"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Routing>, ApiError> {
        let result: Option<RoutingRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, product_id, version, is_active, is_primary,
                   created_at, updated_at
            FROM routings
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find routing by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<Routing>, ApiError> {
        let rows: Vec<RoutingRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, product_id, version, is_active, is_primary,
                   created_at, updated_at
            FROM routings
            WHERE product_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(product_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find routings by product: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_primary_by_product(&self, product_id: i64) -> Result<Option<Routing>, ApiError> {
        let result: Option<RoutingRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, product_id, version, is_active, is_primary,
                   created_at, updated_at
            FROM routings
            WHERE product_id = $1 AND is_primary = true AND is_active = true
            LIMIT 1
            "#,
        )
        .bind(product_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find primary routing by product: {}", e))
        })?;

        Ok(result.map(|r| r.into()))
    }

    async fn add_operation(
        &self,
        create: CreateRoutingOperation,
    ) -> Result<RoutingOperation, ApiError> {
        let row: RoutingOperationRow = sqlx::query_as(
            r#"
            INSERT INTO routing_operations (routing_id, sequence, operation_name, work_center_id,
                                            setup_hours, run_hours, description)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, routing_id, sequence, operation_name, work_center_id,
                      setup_hours, run_hours, description
            "#,
        )
        .bind(create.routing_id)
        .bind(create.sequence)
        .bind(&create.operation_name)
        .bind(create.work_center_id)
        .bind(create.setup_hours)
        .bind(create.run_hours)
        .bind(&create.description)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "RoutingOperation"))?;

        Ok(row.into())
    }

    async fn get_operations(&self, routing_id: i64) -> Result<Vec<RoutingOperation>, ApiError> {
        let rows: Vec<RoutingOperationRow> = sqlx::query_as(
            r#"
            SELECT id, routing_id, sequence, operation_name, work_center_id,
                   setup_hours, run_hours, description
            FROM routing_operations
            WHERE routing_id = $1
            ORDER BY sequence
            "#,
        )
        .bind(routing_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get routing operations: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE routings
            SET deleted_at = NOW(), deleted_by = $3, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete routing: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Routing not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Routing, ApiError> {
        let row: RoutingRow = sqlx::query_as(
            r#"
            UPDATE routings
            SET deleted_at = NULL, deleted_by = NULL, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING id, tenant_id, product_id, version, is_active, is_primary,
                      created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Routing"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Routing>, ApiError> {
        let rows: Vec<RoutingRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, product_id, version, is_active, is_primary,
                   created_at, updated_at
            FROM routings
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted routings: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM routings
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy routing: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Routing not found".to_string()));
        }

        Ok(())
    }
}

// ==================== INSPECTION ====================

/// Database row representation for Inspection
#[derive(Debug, FromRow)]
struct InspectionRow {
    id: i64,
    tenant_id: i64,
    work_order_id: Option<i64>,
    product_id: i64,
    inspection_type: String,
    quantity_inspected: Decimal,
    quantity_passed: Decimal,
    quantity_failed: Decimal,
    status: String,
    inspector_id: Option<i64>,
    inspected_at: Option<chrono::DateTime<chrono::Utc>>,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

/// Convert InspectionStatus to/from database string
fn inspection_status_to_str(status: &InspectionStatus) -> &'static str {
    match status {
        InspectionStatus::Pending => "Pending",
        InspectionStatus::InProgress => "InProgress",
        InspectionStatus::Passed => "Passed",
        InspectionStatus::Failed => "Failed",
        InspectionStatus::Rework => "Rework",
    }
}

fn parse_inspection_status(s: &str) -> Result<InspectionStatus, String> {
    match s {
        "Pending" => Ok(InspectionStatus::Pending),
        "InProgress" => Ok(InspectionStatus::InProgress),
        "Passed" => Ok(InspectionStatus::Passed),
        "Failed" => Ok(InspectionStatus::Failed),
        "Rework" => Ok(InspectionStatus::Rework),
        _ => Err(format!("Invalid inspection status: {}", s)),
    }
}

impl From<InspectionRow> for Inspection {
    fn from(row: InspectionRow) -> Self {
        let status = parse_inspection_status(&row.status).unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid inspection status '{}' in database: {}, defaulting to Pending",
                row.status,
                e
            );
            InspectionStatus::Pending
        });
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            work_order_id: row.work_order_id,
            product_id: row.product_id,
            inspection_type: row.inspection_type,
            quantity_inspected: row.quantity_inspected,
            quantity_passed: row.quantity_passed,
            quantity_failed: row.quantity_failed,
            status,
            inspector_id: row.inspector_id,
            inspected_at: row.inspected_at,
            notes: row.notes,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL inspection repository
pub struct PostgresInspectionRepository {
    pool: Arc<PgPool>,
}

impl PostgresInspectionRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxInspectionRepository {
        Arc::new(self) as BoxInspectionRepository
    }
}

#[async_trait]
impl InspectionRepository for PostgresInspectionRepository {
    async fn create(&self, create: CreateInspection) -> Result<Inspection, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let status_str = inspection_status_to_str(&create.status);
        let inspected_at = if create.status == InspectionStatus::Passed
            || create.status == InspectionStatus::Failed
        {
            Some(chrono::Utc::now())
        } else {
            None
        };

        let row: InspectionRow = sqlx::query_as(
            r#"
            INSERT INTO inspections (tenant_id, work_order_id, product_id, inspection_type,
                                     quantity_inspected, quantity_passed, quantity_failed,
                                     status, inspector_id, inspected_at, notes, created_at,
                                     deleted_at, deleted_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NULL, NULL)
            RETURNING id, tenant_id, work_order_id, product_id, inspection_type,
                      quantity_inspected, quantity_passed, quantity_failed,
                      status, inspector_id, inspected_at, notes, created_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(create.work_order_id)
        .bind(create.product_id)
        .bind(&create.inspection_type)
        .bind(create.quantity_inspected)
        .bind(create.quantity_passed)
        .bind(create.quantity_failed)
        .bind(status_str)
        .bind(create.inspector_id)
        .bind(inspected_at)
        .bind(&create.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Inspection"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Inspection>, ApiError> {
        let result: Option<InspectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, work_order_id, product_id, inspection_type,
                   quantity_inspected, quantity_passed, quantity_failed,
                   status, inspector_id, inspected_at, notes, created_at,
                   deleted_at, deleted_by
            FROM inspections
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find inspection by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Inspection>, ApiError> {
        let rows: Vec<InspectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, work_order_id, product_id, inspection_type,
                   quantity_inspected, quantity_passed, quantity_failed,
                   status, inspector_id, inspected_at, notes, created_at,
                   deleted_at, deleted_by
            FROM inspections
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find inspections by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_work_order(&self, work_order_id: i64) -> Result<Vec<Inspection>, ApiError> {
        let rows: Vec<InspectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, work_order_id, product_id, inspection_type,
                   quantity_inspected, quantity_passed, quantity_failed,
                   status, inspector_id, inspected_at, notes, created_at,
                   deleted_at, deleted_by
            FROM inspections
            WHERE work_order_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(work_order_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find inspections by work order: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update(&self, id: i64, update: UpdateInspection) -> Result<Inspection, ApiError> {
        let row: InspectionRow = sqlx::query_as(
            r#"
            UPDATE inspections
            SET status = COALESCE($2, status),
                quantity_passed = COALESCE($3, quantity_passed),
                quantity_failed = COALESCE($4, quantity_failed),
                inspector_id = COALESCE($5, inspector_id),
                notes = COALESCE($6, notes),
                inspected_at = CASE
                    WHEN $2 IN ('Passed', 'Failed') THEN NOW()
                    ELSE inspected_at
                END
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING id, tenant_id, work_order_id, product_id, inspection_type,
                      quantity_inspected, quantity_passed, quantity_failed,
                      status, inspector_id, inspected_at, notes, created_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(id)
        .bind(
            update
                .status
                .map(|s| inspection_status_to_str(&s).to_string()),
        )
        .bind(update.quantity_passed)
        .bind(update.quantity_failed)
        .bind(update.inspector_id)
        .bind(&update.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Inspection"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE inspections
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete inspection: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Inspection not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Inspection, ApiError> {
        let row: InspectionRow = sqlx::query_as(
            r#"
            UPDATE inspections
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING id, tenant_id, work_order_id, product_id, inspection_type,
                      quantity_inspected, quantity_passed, quantity_failed,
                      status, inspector_id, inspected_at, notes, created_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Inspection"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Inspection>, ApiError> {
        let rows: Vec<InspectionRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, work_order_id, product_id, inspection_type,
                   quantity_inspected, quantity_passed, quantity_failed,
                   status, inspector_id, inspected_at, notes, created_at,
                   deleted_at, deleted_by
            FROM inspections
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted inspections: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM inspections
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy inspection: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Inspection not found".to_string()));
        }

        Ok(())
    }
}

// ==================== NON-CONFORMANCE REPORT ====================

/// Database row representation for NonConformanceReport
#[derive(Debug, FromRow)]
struct NcrRow {
    id: i64,
    tenant_id: i64,
    inspection_id: Option<i64>,
    product_id: i64,
    ncr_type: String,
    description: String,
    root_cause: Option<String>,
    corrective_action: Option<String>,
    status: String,
    raised_by: i64,
    raised_at: chrono::DateTime<chrono::Utc>,
    closed_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

fn ncr_type_to_str(ncr_type: &NcrType) -> &'static str {
    match ncr_type {
        NcrType::Minor => "Minor",
        NcrType::Major => "Major",
        NcrType::Critical => "Critical",
    }
}

fn parse_ncr_type(s: &str) -> Result<NcrType, String> {
    match s {
        "Minor" => Ok(NcrType::Minor),
        "Major" => Ok(NcrType::Major),
        "Critical" => Ok(NcrType::Critical),
        _ => Err(format!("Invalid NCR type: {}", s)),
    }
}

fn ncr_status_to_str(status: &NcrStatus) -> &'static str {
    match status {
        NcrStatus::Open => "Open",
        NcrStatus::UnderReview => "UnderReview",
        NcrStatus::CorrectiveAction => "CorrectiveAction",
        NcrStatus::Closed => "Closed",
        NcrStatus::Rejected => "Rejected",
    }
}

fn parse_ncr_status(s: &str) -> Result<NcrStatus, String> {
    match s {
        "Open" => Ok(NcrStatus::Open),
        "UnderReview" => Ok(NcrStatus::UnderReview),
        "CorrectiveAction" => Ok(NcrStatus::CorrectiveAction),
        "Closed" => Ok(NcrStatus::Closed),
        "Rejected" => Ok(NcrStatus::Rejected),
        _ => Err(format!("Invalid NCR status: {}", s)),
    }
}

impl From<NcrRow> for NonConformanceReport {
    fn from(row: NcrRow) -> Self {
        let ncr_type = parse_ncr_type(&row.ncr_type).unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid NCR type '{}' in database: {}, defaulting to Minor",
                row.ncr_type,
                e
            );
            NcrType::Minor
        });
        let status = parse_ncr_status(&row.status).unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid NCR status '{}' in database: {}, defaulting to Open",
                row.status,
                e
            );
            NcrStatus::Open
        });
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            inspection_id: row.inspection_id,
            product_id: row.product_id,
            ncr_type,
            description: row.description,
            root_cause: row.root_cause,
            corrective_action: row.corrective_action,
            status,
            raised_by: row.raised_by,
            raised_at: row.raised_at,
            closed_at: row.closed_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// PostgreSQL NCR repository
pub struct PostgresNcrRepository {
    pool: Arc<PgPool>,
}

impl PostgresNcrRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxNcrRepository {
        Arc::new(self) as BoxNcrRepository
    }
}

#[async_trait]
impl NcrRepository for PostgresNcrRepository {
    async fn create(
        &self,
        create: CreateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let ncr_type_str = ncr_type_to_str(&create.ncr_type);

        let row: NcrRow = sqlx::query_as(
            r#"
            INSERT INTO non_conformance_reports (tenant_id, inspection_id, product_id, ncr_type,
                                                 description, root_cause, corrective_action,
                                                 status, raised_by, raised_at, closed_at,
                                                 deleted_at, deleted_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'Open', $8, NOW(), NULL, NULL, NULL)
            RETURNING id, tenant_id, inspection_id, product_id, ncr_type,
                      description, root_cause, corrective_action,
                      status, raised_by, raised_at, closed_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(create.inspection_id)
        .bind(create.product_id)
        .bind(ncr_type_str)
        .bind(&create.description)
        .bind(&create.root_cause)
        .bind(&create.corrective_action)
        .bind(create.raised_by)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "NonConformanceReport"))?;

        Ok(row.into())
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<NonConformanceReport>, ApiError> {
        let result: Option<NcrRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, inspection_id, product_id, ncr_type,
                   description, root_cause, corrective_action,
                   status, raised_by, raised_at, closed_at,
                   deleted_at, deleted_by
            FROM non_conformance_reports
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find NCR by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<NonConformanceReport>, ApiError> {
        let rows: Vec<NcrRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, inspection_id, product_id, ncr_type,
                   description, root_cause, corrective_action,
                   status, raised_by, raised_at, closed_at,
                   deleted_at, deleted_by
            FROM non_conformance_reports
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY raised_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find NCRs by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_inspection(
        &self,
        inspection_id: i64,
    ) -> Result<Vec<NonConformanceReport>, ApiError> {
        let rows: Vec<NcrRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, inspection_id, product_id, ncr_type,
                   description, root_cause, corrective_action,
                   status, raised_by, raised_at, closed_at,
                   deleted_at, deleted_by
            FROM non_conformance_reports
            WHERE inspection_id = $1 AND deleted_at IS NULL
            ORDER BY raised_at DESC
            "#,
        )
        .bind(inspection_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find NCRs by inspection: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update(
        &self,
        id: i64,
        update: UpdateNonConformanceReport,
    ) -> Result<NonConformanceReport, ApiError> {
        let row: NcrRow = sqlx::query_as(
            r#"
            UPDATE non_conformance_reports
            SET ncr_type = COALESCE($2, ncr_type),
                description = COALESCE($3, description),
                root_cause = COALESCE($4, root_cause),
                corrective_action = COALESCE($5, corrective_action),
                status = COALESCE($6, status),
                closed_at = CASE
                    WHEN $6 = 'Closed' THEN NOW()
                    ELSE closed_at
                END
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING id, tenant_id, inspection_id, product_id, ncr_type,
                      description, root_cause, corrective_action,
                      status, raised_by, raised_at, closed_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(id)
        .bind(update.ncr_type.map(|t| ncr_type_to_str(&t).to_string()))
        .bind(&update.description)
        .bind(&update.root_cause)
        .bind(&update.corrective_action)
        .bind(update.status.map(|s| ncr_status_to_str(&s).to_string()))
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "NonConformanceReport"))?;

        Ok(row.into())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE non_conformance_reports
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete NCR: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("NCR not found".to_string()));
        }

        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<NonConformanceReport, ApiError> {
        let row: NcrRow = sqlx::query_as(
            r#"
            UPDATE non_conformance_reports
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING id, tenant_id, inspection_id, product_id, ncr_type,
                      description, root_cause, corrective_action,
                      status, raised_by, raised_at, closed_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "NonConformanceReport"))?;

        Ok(row.into())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<NonConformanceReport>, ApiError> {
        let rows: Vec<NcrRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, inspection_id, product_id, ncr_type,
                   description, root_cause, corrective_action,
                   status, raised_by, raised_at, closed_at,
                   deleted_at, deleted_by
            FROM non_conformance_reports
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted NCRs: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM non_conformance_reports
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy NCR: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("NCR not found".to_string()));
        }

        Ok(())
    }
}
