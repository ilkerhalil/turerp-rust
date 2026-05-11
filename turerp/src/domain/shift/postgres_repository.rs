//! PostgreSQL Shift Planning repository implementation

use async_trait::async_trait;
use chrono::{DateTime, NaiveTime, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::shift::model::{
    AttendanceRecord, AttendanceRecordResponse, AttendanceStatus, ClockInRequest, ClockOutRequest,
    CreateShift, CreateShiftAssignment, Shift, ShiftAssignment, ShiftResponse, ShiftType,
    UpdateShift,
};
use crate::domain::shift::repository::{
    AttendanceRecordRepository, BoxAttendanceRecordRepository, BoxShiftAssignmentRepository,
    BoxShiftRepository, ShiftAssignmentRepository, ShiftRepository,
};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Shift row and repository
// ---------------------------------------------------------------------------

#[derive(Debug, FromRow)]
struct ShiftRow {
    id: i64,
    tenant_id: i64,
    name: String,
    shift_type: String,
    start_time: NaiveTime,
    end_time: NaiveTime,
    break_duration_minutes: i32,
    expected_hours: Decimal,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
    total_count: Option<i64>,
}

impl From<ShiftRow> for Shift {
    fn from(row: ShiftRow) -> Self {
        let shift_type = row.shift_type.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid shift type '{}' in database: {}, defaulting to Custom",
                row.shift_type,
                e
            );
            ShiftType::Custom
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            shift_type,
            start_time: row.start_time,
            end_time: row.end_time,
            break_duration_minutes: row.break_duration_minutes,
            expected_hours: row.expected_hours,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

pub struct PostgresShiftRepository {
    pool: Arc<PgPool>,
}

impl PostgresShiftRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxShiftRepository {
        Arc::new(self) as BoxShiftRepository
    }
}

#[async_trait]
impl ShiftRepository for PostgresShiftRepository {
    async fn create(&self, create: CreateShift) -> Result<Shift, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let shift_type_str = create.shift_type.to_string();

        let row: ShiftRow = sqlx::query_as(
            r#"
            INSERT INTO shifts (tenant_id, name, shift_type, start_time, end_time,
                              break_duration_minutes, expected_hours, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, true, NOW(), NOW())
            RETURNING id, tenant_id, name, shift_type, start_time, end_time,
                      break_duration_minutes, expected_hours, is_active, created_at, updated_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.name)
        .bind(&shift_type_str)
        .bind(create.start_time)
        .bind(create.end_time)
        .bind(create.break_duration_minutes)
        .bind(create.expected_hours)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Shift"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Shift>, ApiError> {
        let result: Option<ShiftRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, shift_type, start_time, end_time,
                   break_duration_minutes, expected_hours, is_active, created_at, updated_at,
                   deleted_at, deleted_by
            FROM shifts
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find shift by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Shift>, ApiError> {
        let rows: Vec<ShiftRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, shift_type, start_time, end_time,
                   break_duration_minutes, expected_hours, is_active, created_at, updated_at,
                   deleted_at, deleted_by
            FROM shifts
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Shift"))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Shift>, ApiError> {
        let offset = (page.saturating_sub(1)) * per_page;
        let rows: Vec<ShiftRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, shift_type, start_time, end_time,
                   break_duration_minutes, expected_hours, is_active, created_at, updated_at,
                   deleted_at, deleted_by,
                   COUNT(*) OVER() as total_count
            FROM shifts
            WHERE tenant_id = $1 AND deleted_at IS NULL
            ORDER BY id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Shift"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<Shift> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateShift,
    ) -> Result<Shift, ApiError> {
        let row: ShiftRow = sqlx::query_as(
            r#"
            UPDATE shifts
            SET name = COALESCE($1, name),
                shift_type = COALESCE($2, shift_type),
                start_time = COALESCE($3, start_time),
                end_time = COALESCE($4, end_time),
                break_duration_minutes = COALESCE($5, break_duration_minutes),
                expected_hours = COALESCE($6, expected_hours),
                is_active = COALESCE($7, is_active),
                updated_at = NOW()
            WHERE id = $8 AND tenant_id = $9 AND deleted_at IS NULL
            RETURNING id, tenant_id, name, shift_type, start_time, end_time,
                      break_duration_minutes, expected_hours, is_active, created_at, updated_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(update.name)
        .bind(update.shift_type.map(|s| s.to_string()))
        .bind(update.start_time)
        .bind(update.end_time)
        .bind(update.break_duration_minutes)
        .bind(update.expected_hours)
        .bind(update.is_active)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Shift"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM shifts
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete shift: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Shift not found".to_string()));
        }
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE shifts
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to soft delete shift: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Shift not found".to_string()));
        }
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Shift, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE shifts
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore shift: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Shift not found or not deleted".to_string(),
            ));
        }

        self.find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Shift not found".to_string()))
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Shift>, ApiError> {
        let rows: Vec<ShiftRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, shift_type, start_time, end_time,
                   break_duration_minutes, expected_hours, is_active, created_at, updated_at,
                   deleted_at, deleted_by
            FROM shifts
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find deleted shifts: {}", e)))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM shifts
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy shift: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Shift not found".to_string()));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ShiftAssignment row and repository
// ---------------------------------------------------------------------------

#[derive(Debug, FromRow)]
struct ShiftAssignmentRow {
    id: i64,
    shift_id: i64,
    employee_id: i64,
    start_date: DateTime<Utc>,
    end_date: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
}

impl From<ShiftAssignmentRow> for ShiftAssignment {
    fn from(row: ShiftAssignmentRow) -> Self {
        Self {
            id: row.id,
            shift_id: row.shift_id,
            employee_id: row.employee_id,
            start_date: row.start_date,
            end_date: row.end_date,
            created_at: row.created_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

pub struct PostgresShiftAssignmentRepository {
    pool: Arc<PgPool>,
}

impl PostgresShiftAssignmentRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxShiftAssignmentRepository {
        Arc::new(self) as BoxShiftAssignmentRepository
    }
}

#[async_trait]
impl ShiftAssignmentRepository for PostgresShiftAssignmentRepository {
    async fn create(&self, create: CreateShiftAssignment) -> Result<ShiftAssignment, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let row: ShiftAssignmentRow = sqlx::query_as(
            r#"
            INSERT INTO shift_assignments (shift_id, employee_id, start_date, end_date, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            RETURNING id, shift_id, employee_id, start_date, end_date, created_at,
                      deleted_at, deleted_by
            "#,
        )
        .bind(create.shift_id)
        .bind(create.employee_id)
        .bind(create.start_date)
        .bind(create.end_date)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ShiftAssignment"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<ShiftAssignment>, ApiError> {
        let result: Option<ShiftAssignmentRow> = sqlx::query_as(
            r#"
            SELECT id, shift_id, employee_id, start_date, end_date, created_at,
                   deleted_at, deleted_by
            FROM shift_assignments
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find shift assignment by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<ShiftAssignment>, ApiError> {
        let rows: Vec<ShiftAssignmentRow> = sqlx::query_as(
            r#"
            SELECT id, shift_id, employee_id, start_date, end_date, created_at,
                   deleted_at, deleted_by
            FROM shift_assignments
            WHERE employee_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(employee_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find shift assignments by employee: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_shift(&self, shift_id: i64) -> Result<Vec<ShiftAssignment>, ApiError> {
        let rows: Vec<ShiftAssignmentRow> = sqlx::query_as(
            r#"
            SELECT id, shift_id, employee_id, start_date, end_date, created_at,
                   deleted_at, deleted_by
            FROM shift_assignments
            WHERE shift_id = $1 AND deleted_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(shift_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find shift assignments by shift: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM shift_assignments
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete shift assignment: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Shift assignment not found".to_string()));
        }
        Ok(())
    }

    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE shift_assignments
            SET deleted_at = NOW(), deleted_by = $2
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to soft delete shift assignment: {}", e))
        })?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Shift assignment not found".to_string()));
        }
        Ok(())
    }

    async fn restore(&self, id: i64) -> Result<ShiftAssignment, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE shift_assignments
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore shift assignment: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Shift assignment not found or not deleted".to_string(),
            ));
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Shift assignment not found".to_string()))
    }

    async fn find_deleted(&self) -> Result<Vec<ShiftAssignment>, ApiError> {
        let rows: Vec<ShiftAssignmentRow> = sqlx::query_as(
            r#"
            SELECT id, shift_id, employee_id, start_date, end_date, created_at,
                   deleted_at, deleted_by
            FROM shift_assignments
            WHERE deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find deleted shift assignments: {}", e))
        })?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM shift_assignments
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy shift assignment: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Shift assignment not found".to_string()));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// AttendanceRecord row and repository
// ---------------------------------------------------------------------------

#[derive(Debug, FromRow)]
struct AttendanceRecordRow {
    id: i64,
    employee_id: i64,
    shift_id: i64,
    date: DateTime<Utc>,
    clock_in: Option<DateTime<Utc>>,
    clock_out: Option<DateTime<Utc>>,
    hours_worked: Decimal,
    overtime_hours: Decimal,
    status: String,
    notes: Option<String>,
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<i64>,
}

impl From<AttendanceRecordRow> for AttendanceRecord {
    fn from(row: AttendanceRecordRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid attendance status '{}' in database: {}, defaulting to Present",
                row.status,
                e
            );
            AttendanceStatus::Present
        });

        Self {
            id: row.id,
            employee_id: row.employee_id,
            shift_id: row.shift_id,
            date: row.date,
            clock_in: row.clock_in,
            clock_out: row.clock_out,
            hours_worked: row.hours_worked,
            overtime_hours: row.overtime_hours,
            status,
            notes: row.notes,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

pub struct PostgresAttendanceRecordRepository {
    pool: Arc<PgPool>,
}

impl PostgresAttendanceRecordRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxAttendanceRecordRepository {
        Arc::new(self) as BoxAttendanceRecordRepository
    }
}

#[async_trait]
impl AttendanceRecordRepository for PostgresAttendanceRecordRepository {
    async fn clock_in(&self, req: ClockInRequest) -> Result<AttendanceRecord, ApiError> {
        let date = req
            .timestamp
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("00:00:00 is always valid")
            .and_local_timezone(Utc)
            .single()
            .expect("UTC has no ambiguous times");
        let status_str = AttendanceStatus::Present.to_string();

        let row: AttendanceRecordRow = sqlx::query_as(
            r#"
            INSERT INTO attendance_records (employee_id, shift_id, date, clock_in, clock_out,
                                            hours_worked, overtime_hours, status, notes)
            VALUES ($1, $2, $3, $4, NULL, 0, 0, $5, $6)
            RETURNING id, employee_id, shift_id, date, clock_in, clock_out,
                      hours_worked, overtime_hours, status, notes, deleted_at, deleted_by
            "#,
        )
        .bind(req.employee_id)
        .bind(req.shift_id)
        .bind(date)
        .bind(req.timestamp)
        .bind(&status_str)
        .bind(&req.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "AttendanceRecord"))?;

        Ok(row.into())
    }

    async fn clock_out(&self, req: ClockOutRequest) -> Result<AttendanceRecord, ApiError> {
        let record: Option<AttendanceRecordRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, shift_id, date, clock_in, clock_out,
                   hours_worked, overtime_hours, status, notes, deleted_at, deleted_by
            FROM attendance_records
            WHERE employee_id = $1 AND date::date = $2::date
              AND clock_out IS NULL AND deleted_at IS NULL
            ORDER BY clock_in DESC
            LIMIT 1
            "#,
        )
        .bind(req.employee_id)
        .bind(req.timestamp)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find open attendance record: {}", e)))?;

        let id = record.as_ref().map(|r| r.id).ok_or_else(|| {
            ApiError::NotFound("No open attendance record found for employee today".to_string())
        })?;

        let clock_in = record
            .as_ref()
            .and_then(|r| r.clock_in)
            .unwrap_or(req.timestamp);
        let duration = req.timestamp.signed_duration_since(clock_in);
        let hours = Decimal::from(duration.num_seconds()) / Decimal::from(3600);
        let hours_worked = hours.max(Decimal::ZERO);

        let row: AttendanceRecordRow = sqlx::query_as(
            r#"
            UPDATE attendance_records
            SET clock_out = $1,
                hours_worked = $2,
                notes = COALESCE($3, notes)
            WHERE id = $4
            RETURNING id, employee_id, shift_id, date, clock_in, clock_out,
                      hours_worked, overtime_hours, status, notes, deleted_at, deleted_by
            "#,
        )
        .bind(req.timestamp)
        .bind(hours_worked)
        .bind(req.notes.as_ref().map(|n| format!("; {}", n)))
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "AttendanceRecord"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<AttendanceRecord>, ApiError> {
        let result: Option<AttendanceRecordRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, shift_id, date, clock_in, clock_out,
                   hours_worked, overtime_hours, status, notes, deleted_at, deleted_by
            FROM attendance_records
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find attendance record by id: {}", e))
        })?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<AttendanceRecord>, ApiError> {
        let rows: Vec<AttendanceRecordRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, shift_id, date, clock_in, clock_out,
                   hours_worked, overtime_hours, status, notes, deleted_at, deleted_by
            FROM attendance_records
            WHERE employee_id = $1 AND deleted_at IS NULL
            ORDER BY date DESC
            "#,
        )
        .bind(employee_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find attendance records by employee: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_employee_and_date(
        &self,
        employee_id: i64,
        date: DateTime<Utc>,
    ) -> Result<Option<AttendanceRecord>, ApiError> {
        let result: Option<AttendanceRecordRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, shift_id, date, clock_in, clock_out,
                   hours_worked, overtime_hours, status, notes, deleted_at, deleted_by
            FROM attendance_records
            WHERE employee_id = $1 AND date::date = $2::date AND deleted_at IS NULL
            "#,
        )
        .bind(employee_id)
        .bind(date)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find attendance record by date: {}", e))
        })?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_period(
        &self,
        employee_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AttendanceRecord>, ApiError> {
        let rows: Vec<AttendanceRecordRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, shift_id, date, clock_in, clock_out,
                   hours_worked, overtime_hours, status, notes, deleted_at, deleted_by
            FROM attendance_records
            WHERE employee_id = $1 AND date >= $2 AND date <= $3 AND deleted_at IS NULL
            ORDER BY date DESC
            "#,
        )
        .bind(employee_id)
        .bind(start)
        .bind(end)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to find attendance records by period: {}",
                e
            ))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM attendance_records
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete attendance record: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Attendance record not found".to_string(),
            ));
        }
        Ok(())
    }

    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE attendance_records
            SET deleted_at = NOW(), deleted_by = $2
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to soft delete attendance record: {}", e))
        })?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Attendance record not found".to_string(),
            ));
        }
        Ok(())
    }

    async fn restore(&self, id: i64) -> Result<AttendanceRecord, ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE attendance_records
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to restore attendance record: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Attendance record not found or not deleted".to_string(),
            ));
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Attendance record not found".to_string()))
    }

    async fn find_deleted(&self) -> Result<Vec<AttendanceRecord>, ApiError> {
        let rows: Vec<AttendanceRecordRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, shift_id, date, clock_in, clock_out,
                   hours_worked, overtime_hours, status, notes, deleted_at, deleted_by
            FROM attendance_records
            WHERE deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find deleted attendance records: {}", e))
        })?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM attendance_records
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to destroy attendance record: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(
                "Attendance record not found".to_string(),
            ));
        }
        Ok(())
    }
}
