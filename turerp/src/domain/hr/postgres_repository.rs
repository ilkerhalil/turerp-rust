//! PostgreSQL HR repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::hr::model::{
    Attendance, AttendanceStatus, CreateAttendance, CreateEmployee, CreateLeaveRequest, Employee,
    EmployeeStatus, LeaveRequest, LeaveRequestStatus, LeaveType, Payroll, PayrollStatus,
};
use crate::domain::hr::repository::{
    AttendanceRepository, BoxAttendanceRepository, BoxEmployeeRepository,
    BoxLeaveRequestRepository, BoxLeaveTypeRepository, BoxPayrollRepository, EmployeeRepository,
    LeaveRequestRepository, LeaveTypeRepository, PayrollRepository,
};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types

// ---------------------------------------------------------------------------
// Employee row and repository
// ---------------------------------------------------------------------------

/// Database row representation for Employee
#[derive(Debug, FromRow)]
struct EmployeeRow {
    id: i64,
    tenant_id: i64,
    user_id: Option<i64>,
    employee_number: String,
    first_name: String,
    last_name: String,
    email: String,
    phone: Option<String>,
    department: Option<String>,
    position: Option<String>,
    hire_date: DateTime<Utc>,
    termination_date: Option<DateTime<Utc>>,
    status: String,
    salary: Decimal,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    total_count: Option<i64>,
}

impl From<EmployeeRow> for Employee {
    fn from(row: EmployeeRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid employee status '{}' in database: {}, defaulting to Active",
                row.status,
                e
            );
            EmployeeStatus::Active
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            user_id: row.user_id,
            employee_number: row.employee_number,
            first_name: row.first_name,
            last_name: row.last_name,
            email: row.email,
            phone: row.phone,
            department: row.department,
            position: row.position,
            hire_date: row.hire_date,
            termination_date: row.termination_date,
            status,
            salary: row.salary,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL employee repository
pub struct PostgresEmployeeRepository {
    pool: Arc<PgPool>,
}

impl PostgresEmployeeRepository {
    /// Create a new PostgreSQL employee repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxEmployeeRepository {
        Arc::new(self) as BoxEmployeeRepository
    }
}

#[async_trait]
impl EmployeeRepository for PostgresEmployeeRepository {
    async fn create(&self, create: CreateEmployee) -> Result<Employee, ApiError> {
        let status = EmployeeStatus::Active.to_string();
        let termination_date: Option<DateTime<Utc>> = None;

        let row: EmployeeRow = sqlx::query_as(
            r#"
            INSERT INTO employees (tenant_id, user_id, employee_number, first_name, last_name,
                                   email, phone, department, position, hire_date, termination_date,
                                   status, salary, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW(), NOW())
            RETURNING id, tenant_id, user_id, employee_number, first_name, last_name,
                      email, phone, department, position, hire_date, termination_date,
                      status, salary, created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(create.user_id)
        .bind(&create.employee_number)
        .bind(&create.first_name)
        .bind(&create.last_name)
        .bind(&create.email)
        .bind(&create.phone)
        .bind(&create.department)
        .bind(&create.position)
        .bind(create.hire_date)
        .bind(termination_date)
        .bind(&status)
        .bind(create.salary)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Employee"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Employee>, ApiError> {
        let result: Option<EmployeeRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, user_id, employee_number, first_name, last_name,
                   email, phone, department, position, hire_date, termination_date,
                   status, salary, created_at, updated_at
            FROM employees
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find employee by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Employee>, ApiError> {
        let rows: Vec<EmployeeRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, user_id, employee_number, first_name, last_name,
                   email, phone, department, position, hire_date, termination_date,
                   status, salary, created_at, updated_at
            FROM employees
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Employee"))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Employee>, ApiError> {
        let offset = (page.saturating_sub(1)) * per_page;
        let rows: Vec<EmployeeRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, user_id, employee_number, first_name, last_name,
                   email, phone, department, position, hire_date, termination_date,
                   status, salary, created_at, updated_at, COUNT(*) OVER() as total_count
            FROM employees
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
        .map_err(|e| map_sqlx_error(e, "Employee"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<Employee> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_user(&self, user_id: i64) -> Result<Option<Employee>, ApiError> {
        let result: Option<EmployeeRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, user_id, employee_number, first_name, last_name,
                   email, phone, department, position, hire_date, termination_date,
                   status, salary, created_at, updated_at
            FROM employees
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find employee by user: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn update_status(&self, id: i64, status: EmployeeStatus) -> Result<Employee, ApiError> {
        let status_str = status.to_string();
        let termination_date: Option<DateTime<Utc>> = if status == EmployeeStatus::Terminated {
            Some(chrono::Utc::now())
        } else {
            None
        };

        let row: EmployeeRow = sqlx::query_as(
            r#"
            UPDATE employees
            SET status = $1,
                termination_date = COALESCE($2, termination_date),
                updated_at = NOW()
            WHERE id = $3
            RETURNING id, tenant_id, user_id, employee_number, first_name, last_name,
                      email, phone, department, position, hire_date, termination_date,
                      status, salary, created_at, updated_at
            "#,
        )
        .bind(&status_str)
        .bind(termination_date)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Employee"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM employees
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete employee: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Employee not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Attendance row and repository
// ---------------------------------------------------------------------------

/// Database row representation for Attendance
#[derive(Debug, FromRow)]
struct AttendanceRow {
    id: i64,
    employee_id: i64,
    date: DateTime<Utc>,
    check_in: Option<DateTime<Utc>>,
    check_out: Option<DateTime<Utc>>,
    hours_worked: Decimal,
    status: String,
    notes: Option<String>,
}

impl From<AttendanceRow> for Attendance {
    fn from(row: AttendanceRow) -> Self {
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
            date: row.date,
            check_in: row.check_in,
            check_out: row.check_out,
            hours_worked: row.hours_worked,
            status,
            notes: row.notes,
        }
    }
}

/// PostgreSQL attendance repository
pub struct PostgresAttendanceRepository {
    pool: Arc<PgPool>,
}

impl PostgresAttendanceRepository {
    /// Create a new PostgreSQL attendance repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxAttendanceRepository {
        Arc::new(self) as BoxAttendanceRepository
    }
}

#[async_trait]
impl AttendanceRepository for PostgresAttendanceRepository {
    async fn create(&self, create: CreateAttendance) -> Result<Attendance, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let hours_worked = create.calculate_hours();
        let status = if create.check_in.is_none() && create.check_out.is_none() {
            AttendanceStatus::Absent
        } else if hours_worked < Decimal::new(4, 0) {
            AttendanceStatus::Late
        } else {
            AttendanceStatus::Present
        };
        let status_str = status.to_string();

        let row: AttendanceRow = sqlx::query_as(
            r#"
            INSERT INTO attendance (employee_id, date, check_in, check_out, hours_worked, status, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, employee_id, date, check_in, check_out, hours_worked, status, notes
            "#,
        )
        .bind(create.employee_id)
        .bind(create.date)
        .bind(create.check_in)
        .bind(create.check_out)
        .bind(hours_worked)
        .bind(&status_str)
        .bind(&create.notes)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Attendance"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Attendance>, ApiError> {
        let result: Option<AttendanceRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, date, check_in, check_out, hours_worked, status, notes
            FROM attendance
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find attendance by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<Attendance>, ApiError> {
        let rows: Vec<AttendanceRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, date, check_in, check_out, hours_worked, status, notes
            FROM attendance
            WHERE employee_id = $1
            ORDER BY date DESC
            "#,
        )
        .bind(employee_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find attendance by employee: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_date(&self, date: DateTime<Utc>) -> Result<Vec<Attendance>, ApiError> {
        let rows: Vec<AttendanceRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, date, check_in, check_out, hours_worked, status, notes
            FROM attendance
            WHERE date::date = $1::date
            ORDER BY employee_id
            "#,
        )
        .bind(date)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find attendance by date: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM attendance
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete attendance: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Attendance not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Leave type row and repository
// ---------------------------------------------------------------------------

/// Database row representation for LeaveType
#[derive(Debug, FromRow)]
struct LeaveTypeRow {
    id: i64,
    tenant_id: i64,
    name: String,
    description: Option<String>,
    max_days_per_year: Decimal,
    requires_approval: bool,
}

impl From<LeaveTypeRow> for LeaveType {
    fn from(row: LeaveTypeRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            description: row.description,
            max_days_per_year: row.max_days_per_year,
            requires_approval: row.requires_approval,
        }
    }
}

/// PostgreSQL leave type repository
pub struct PostgresLeaveTypeRepository {
    pool: Arc<PgPool>,
}

impl PostgresLeaveTypeRepository {
    /// Create a new PostgreSQL leave type repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxLeaveTypeRepository {
        Arc::new(self) as BoxLeaveTypeRepository
    }
}

#[async_trait]
impl LeaveTypeRepository for PostgresLeaveTypeRepository {
    async fn create(&self, leave_type: LeaveType) -> Result<LeaveType, ApiError> {
        let row: LeaveTypeRow = sqlx::query_as(
            r#"
            INSERT INTO leave_types (tenant_id, name, description, max_days_per_year, requires_approval)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, tenant_id, name, description, max_days_per_year, requires_approval
            "#,
        )
        .bind(leave_type.tenant_id)
        .bind(&leave_type.name)
        .bind(&leave_type.description)
        .bind(leave_type.max_days_per_year)
        .bind(leave_type.requires_approval)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "LeaveType"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<LeaveType>, ApiError> {
        let result: Option<LeaveTypeRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, max_days_per_year, requires_approval
            FROM leave_types
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find leave type by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<LeaveType>, ApiError> {
        let rows: Vec<LeaveTypeRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, description, max_days_per_year, requires_approval
            FROM leave_types
            WHERE tenant_id = $1
            ORDER BY name
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find leave types by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM leave_types
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete leave type: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Leave type not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Leave request row and repository
// ---------------------------------------------------------------------------

/// Database row representation for LeaveRequest
#[derive(Debug, FromRow)]
struct LeaveRequestRow {
    id: i64,
    employee_id: i64,
    leave_type_id: i64,
    status: String,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    total_days: Decimal,
    reason: Option<String>,
    approved_by: Option<i64>,
    approved_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<LeaveRequestRow> for LeaveRequest {
    fn from(row: LeaveRequestRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid leave request status '{}' in database: {}, defaulting to Pending",
                row.status,
                e
            );
            LeaveRequestStatus::Pending
        });

        Self {
            id: row.id,
            employee_id: row.employee_id,
            leave_type_id: row.leave_type_id,
            status,
            start_date: row.start_date,
            end_date: row.end_date,
            total_days: row.total_days,
            reason: row.reason,
            approved_by: row.approved_by,
            approved_at: row.approved_at,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL leave request repository
pub struct PostgresLeaveRequestRepository {
    pool: Arc<PgPool>,
}

impl PostgresLeaveRequestRepository {
    /// Create a new PostgreSQL leave request repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxLeaveRequestRepository {
        Arc::new(self) as BoxLeaveRequestRepository
    }
}

#[async_trait]
impl LeaveRequestRepository for PostgresLeaveRequestRepository {
    async fn create(&self, create: CreateLeaveRequest) -> Result<LeaveRequest, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let total_days = create.calculate_total_days();
        let status = LeaveRequestStatus::Pending.to_string();

        let row: LeaveRequestRow = sqlx::query_as(
            r#"
            INSERT INTO leave_requests (employee_id, leave_type_id, status, start_date, end_date,
                                        total_days, reason, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING id, employee_id, leave_type_id, status, start_date, end_date,
                      total_days, reason, approved_by, approved_at, created_at
            "#,
        )
        .bind(create.employee_id)
        .bind(create.leave_type_id)
        .bind(&status)
        .bind(create.start_date)
        .bind(create.end_date)
        .bind(total_days)
        .bind(&create.reason)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "LeaveRequest"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<LeaveRequest>, ApiError> {
        let result: Option<LeaveRequestRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, leave_type_id, status, start_date, end_date,
                   total_days, reason, approved_by, approved_at, created_at
            FROM leave_requests
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find leave request by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<LeaveRequest>, ApiError> {
        let rows: Vec<LeaveRequestRow> = sqlx::query_as(
            r#"
            SELECT id, employee_id, leave_type_id, status, start_date, end_date,
                   total_days, reason, approved_by, approved_at, created_at
            FROM leave_requests
            WHERE employee_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(employee_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find leave requests by employee: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(
        &self,
        id: i64,
        status: LeaveRequestStatus,
        approver_id: Option<i64>,
    ) -> Result<LeaveRequest, ApiError> {
        let status_str = status.to_string();

        let row: LeaveRequestRow = sqlx::query_as(
            r#"
            UPDATE leave_requests
            SET status = $1,
                approved_by = COALESCE($2, approved_by),
                approved_at = CASE WHEN $2 IS NOT NULL THEN NOW() ELSE approved_at END
            WHERE id = $3
            RETURNING id, employee_id, leave_type_id, status, start_date, end_date,
                      total_days, reason, approved_by, approved_at, created_at
            "#,
        )
        .bind(&status_str)
        .bind(approver_id)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "LeaveRequest"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM leave_requests
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete leave request: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Leave request not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Payroll row and repository
// ---------------------------------------------------------------------------

/// Database row representation for Payroll
#[derive(Debug, FromRow)]
struct PayrollRow {
    id: i64,
    tenant_id: i64,
    employee_id: i64,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    basic_salary: Decimal,
    overtime_hours: Decimal,
    overtime_pay: Decimal,
    bonuses: Decimal,
    deductions: Decimal,
    net_salary: Decimal,
    status: String,
    paid_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<PayrollRow> for Payroll {
    fn from(row: PayrollRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid payroll status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            PayrollStatus::Draft
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            employee_id: row.employee_id,
            period_start: row.period_start,
            period_end: row.period_end,
            basic_salary: row.basic_salary,
            overtime_hours: row.overtime_hours,
            overtime_pay: row.overtime_pay,
            bonuses: row.bonuses,
            deductions: row.deductions,
            net_salary: row.net_salary,
            status,
            paid_at: row.paid_at,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL payroll repository
pub struct PostgresPayrollRepository {
    pool: Arc<PgPool>,
}

impl PostgresPayrollRepository {
    /// Create a new PostgreSQL payroll repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxPayrollRepository {
        Arc::new(self) as BoxPayrollRepository
    }
}

#[async_trait]
impl PayrollRepository for PostgresPayrollRepository {
    async fn create(&self, payroll: Payroll) -> Result<Payroll, ApiError> {
        let status_str = payroll.status.to_string();

        let row: PayrollRow = sqlx::query_as(
            r#"
            INSERT INTO payrolls (tenant_id, employee_id, period_start, period_end,
                                  basic_salary, overtime_hours, overtime_pay, bonuses,
                                  deductions, net_salary, status, paid_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())
            RETURNING id, tenant_id, employee_id, period_start, period_end,
                      basic_salary, overtime_hours, overtime_pay, bonuses,
                      deductions, net_salary, status, paid_at, created_at
            "#,
        )
        .bind(payroll.tenant_id)
        .bind(payroll.employee_id)
        .bind(payroll.period_start)
        .bind(payroll.period_end)
        .bind(payroll.basic_salary)
        .bind(payroll.overtime_hours)
        .bind(payroll.overtime_pay)
        .bind(payroll.bonuses)
        .bind(payroll.deductions)
        .bind(payroll.net_salary)
        .bind(&status_str)
        .bind(payroll.paid_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Payroll"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Payroll>, ApiError> {
        let result: Option<PayrollRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, employee_id, period_start, period_end,
                   basic_salary, overtime_hours, overtime_pay, bonuses,
                   deductions, net_salary, status, paid_at, created_at
            FROM payrolls
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find payroll by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<Payroll>, ApiError> {
        let rows: Vec<PayrollRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, employee_id, period_start, period_end,
                   basic_salary, overtime_hours, overtime_pay, bonuses,
                   deductions, net_salary, status, paid_at, created_at
            FROM payrolls
            WHERE employee_id = $1
            ORDER BY period_start DESC
            "#,
        )
        .bind(employee_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find payroll by employee: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_period(
        &self,
        tenant_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Payroll>, ApiError> {
        let rows: Vec<PayrollRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, employee_id, period_start, period_end,
                   basic_salary, overtime_hours, overtime_pay, bonuses,
                   deductions, net_salary, status, paid_at, created_at
            FROM payrolls
            WHERE tenant_id = $1 AND period_start >= $2 AND period_end <= $3
            ORDER BY period_start DESC
            "#,
        )
        .bind(tenant_id)
        .bind(start)
        .bind(end)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find payroll by period: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(&self, id: i64, status: PayrollStatus) -> Result<Payroll, ApiError> {
        let status_str = status.to_string();

        let row: PayrollRow = sqlx::query_as(
            r#"
            UPDATE payrolls
            SET status = $1
            WHERE id = $2
            RETURNING id, tenant_id, employee_id, period_start, period_end,
                      basic_salary, overtime_hours, overtime_pay, bonuses,
                      deductions, net_salary, status, paid_at, created_at
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Payroll"))?;

        Ok(row.into())
    }

    async fn mark_paid(&self, id: i64) -> Result<Payroll, ApiError> {
        let row: PayrollRow = sqlx::query_as(
            r#"
            UPDATE payrolls
            SET status = 'Paid',
                paid_at = NOW()
            WHERE id = $1
            RETURNING id, tenant_id, employee_id, period_start, period_end,
                      basic_salary, overtime_hours, overtime_pay, bonuses,
                      deductions, net_salary, status, paid_at, created_at
            "#,
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Payroll"))?;

        Ok(row.into())
    }
}
