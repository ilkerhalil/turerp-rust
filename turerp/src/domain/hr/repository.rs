//! HR repository

use async_trait::async_trait;
use chrono::Utc;
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::domain::hr::model::{
    Attendance, AttendanceStatus, CreateAttendance, CreateEmployee, CreateLeaveRequest, Employee,
    EmployeeStatus, LeaveRequest, LeaveRequestStatus, LeaveType, Payroll, PayrollStatus,
};
use crate::error::ApiError;

/// Repository trait for Employee operations
#[async_trait]
pub trait EmployeeRepository: Send + Sync {
    async fn create(&self, employee: CreateEmployee) -> Result<Employee, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Employee>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Employee>, ApiError>;
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Employee>, ApiError>;
    async fn find_by_user(&self, user_id: i64) -> Result<Option<Employee>, ApiError>;
    async fn update_status(&self, id: i64, status: EmployeeStatus) -> Result<Employee, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for Attendance operations
#[async_trait]
pub trait AttendanceRepository: Send + Sync {
    async fn create(&self, attendance: CreateAttendance) -> Result<Attendance, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Attendance>, ApiError>;
    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<Attendance>, ApiError>;
    async fn find_by_date(&self, date: chrono::DateTime<Utc>) -> Result<Vec<Attendance>, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for LeaveRequest operations
#[async_trait]
pub trait LeaveRequestRepository: Send + Sync {
    async fn create(&self, request: CreateLeaveRequest) -> Result<LeaveRequest, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<LeaveRequest>, ApiError>;
    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<LeaveRequest>, ApiError>;
    async fn update_status(
        &self,
        id: i64,
        status: LeaveRequestStatus,
        approver_id: Option<i64>,
    ) -> Result<LeaveRequest, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for LeaveType operations
#[async_trait]
pub trait LeaveTypeRepository: Send + Sync {
    async fn create(&self, leave_type: LeaveType) -> Result<LeaveType, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<LeaveType>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<LeaveType>, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for Payroll operations
#[async_trait]
pub trait PayrollRepository: Send + Sync {
    async fn create(&self, payroll: Payroll) -> Result<Payroll, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Payroll>, ApiError>;
    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<Payroll>, ApiError>;
    async fn find_by_period(
        &self,
        tenant_id: i64,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<Vec<Payroll>, ApiError>;
    async fn update_status(&self, id: i64, status: PayrollStatus) -> Result<Payroll, ApiError>;
    async fn mark_paid(&self, id: i64) -> Result<Payroll, ApiError>;
}

/// Type aliases
pub type BoxEmployeeRepository = Arc<dyn EmployeeRepository>;
pub type BoxAttendanceRepository = Arc<dyn AttendanceRepository>;
pub type BoxLeaveRequestRepository = Arc<dyn LeaveRequestRepository>;
pub type BoxLeaveTypeRepository = Arc<dyn LeaveTypeRepository>;
pub type BoxPayrollRepository = Arc<dyn PayrollRepository>;

/// Inner state for InMemoryEmployeeRepository
struct InMemoryEmployeeInner {
    employees: std::collections::HashMap<i64, Employee>,
    next_id: i64,
}

/// In-memory employee repository
pub struct InMemoryEmployeeRepository {
    inner: Mutex<InMemoryEmployeeInner>,
}

impl InMemoryEmployeeRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryEmployeeInner {
                employees: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryEmployeeRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmployeeRepository for InMemoryEmployeeRepository {
    async fn create(&self, create: CreateEmployee) -> Result<Employee, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let now = chrono::Utc::now();

        let employee = Employee {
            id,
            tenant_id: create.tenant_id,
            user_id: create.user_id,
            employee_number: create.employee_number,
            first_name: create.first_name,
            last_name: create.last_name,
            email: create.email,
            phone: create.phone,
            department: create.department,
            position: create.position,
            hire_date: create.hire_date,
            termination_date: None,
            status: EmployeeStatus::Active,
            salary: create.salary,
            created_at: now,
            updated_at: now,
        };

        inner.employees.insert(id, employee.clone());
        Ok(employee)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Employee>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.employees.get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Employee>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .employees
            .values()
            .filter(|e| e.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Employee>, ApiError> {
        let inner = self.inner.lock();
        let total = inner
            .employees
            .values()
            .filter(|e| e.tenant_id == tenant_id)
            .count() as u64;

        let items: Vec<Employee> = inner
            .employees
            .values()
            .filter(|e| e.tenant_id == tenant_id)
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .cloned()
            .collect();

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_user(&self, user_id: i64) -> Result<Option<Employee>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .employees
            .values()
            .find(|e| e.user_id == Some(user_id))
            .cloned())
    }

    async fn update_status(&self, id: i64, status: EmployeeStatus) -> Result<Employee, ApiError> {
        let mut inner = self.inner.lock();
        let employee = inner
            .employees
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Employee {} not found", id)))?;
        employee.status = status;
        employee.updated_at = chrono::Utc::now();
        Ok(employee.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.employees.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryAttendanceRepository
struct InMemoryAttendanceInner {
    records: std::collections::HashMap<i64, Attendance>,
    next_id: i64,
}

/// In-memory attendance repository
pub struct InMemoryAttendanceRepository {
    inner: Mutex<InMemoryAttendanceInner>,
}

impl InMemoryAttendanceRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryAttendanceInner {
                records: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryAttendanceRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AttendanceRepository for InMemoryAttendanceRepository {
    async fn create(&self, create: CreateAttendance) -> Result<Attendance, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let hours_worked = create.calculate_hours();
        let status = if create.check_in.is_none() && create.check_out.is_none() {
            AttendanceStatus::Absent
        } else if hours_worked < Decimal::new(4, 0) {
            AttendanceStatus::Late
        } else {
            AttendanceStatus::Present
        };

        let record = Attendance {
            id,
            employee_id: create.employee_id,
            date: create.date,
            check_in: create.check_in,
            check_out: create.check_out,
            hours_worked,
            status,
            notes: create.notes,
        };

        inner.records.insert(id, record.clone());
        Ok(record)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Attendance>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.records.get(&id).cloned())
    }

    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<Attendance>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .filter(|r| r.employee_id == employee_id)
            .cloned()
            .collect())
    }

    async fn find_by_date(&self, date: chrono::DateTime<Utc>) -> Result<Vec<Attendance>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .filter(|r| r.date.date_naive() == date.date_naive())
            .cloned()
            .collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.records.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryLeaveRequestRepository
struct InMemoryLeaveRequestInner {
    requests: std::collections::HashMap<i64, LeaveRequest>,
    next_id: i64,
}

/// In-memory leave request repository
pub struct InMemoryLeaveRequestRepository {
    inner: Mutex<InMemoryLeaveRequestInner>,
}

impl InMemoryLeaveRequestRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryLeaveRequestInner {
                requests: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryLeaveRequestRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LeaveRequestRepository for InMemoryLeaveRequestRepository {
    async fn create(&self, create: CreateLeaveRequest) -> Result<LeaveRequest, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let request = LeaveRequest {
            id,
            employee_id: create.employee_id,
            leave_type_id: create.leave_type_id,
            status: LeaveRequestStatus::Pending,
            start_date: create.start_date,
            end_date: create.end_date,
            total_days: create.calculate_total_days(),
            reason: create.reason,
            approved_by: None,
            approved_at: None,
            created_at: chrono::Utc::now(),
        };

        inner.requests.insert(id, request.clone());
        Ok(request)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<LeaveRequest>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.requests.get(&id).cloned())
    }

    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<LeaveRequest>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .requests
            .values()
            .filter(|r| r.employee_id == employee_id)
            .cloned()
            .collect())
    }

    async fn update_status(
        &self,
        id: i64,
        status: LeaveRequestStatus,
        approver_id: Option<i64>,
    ) -> Result<LeaveRequest, ApiError> {
        let mut inner = self.inner.lock();
        let request = inner
            .requests
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Leave request {} not found", id)))?;
        request.status = status;
        if let Some(aid) = approver_id {
            request.approved_by = Some(aid);
            request.approved_at = Some(chrono::Utc::now());
        }
        Ok(request.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.requests.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryLeaveTypeRepository
struct InMemoryLeaveTypeInner {
    types: std::collections::HashMap<i64, LeaveType>,
    next_id: i64,
}

/// In-memory leave type repository
pub struct InMemoryLeaveTypeRepository {
    inner: Mutex<InMemoryLeaveTypeInner>,
}

impl InMemoryLeaveTypeRepository {
    pub fn new() -> Self {
        let repo = Self {
            inner: Mutex::new(InMemoryLeaveTypeInner {
                types: std::collections::HashMap::new(),
                next_id: 1,
            }),
        };
        // Add default leave types
        let defaults = vec![
            (
                1,
                1,
                "Annual Leave",
                "Yearly vacation",
                Decimal::new(20, 0),
                true,
            ),
            (
                2,
                1,
                "Sick Leave",
                "Medical leave",
                Decimal::new(10, 0),
                false,
            ),
            (
                3,
                1,
                "Personal Leave",
                "Personal matters",
                Decimal::new(5, 0),
                true,
            ),
        ];
        let mut inner = repo.inner.lock();
        for (id, tid, name, desc, max, req) in defaults {
            inner.types.insert(
                id,
                LeaveType {
                    id,
                    tenant_id: tid,
                    name: name.to_string(),
                    description: Some(desc.to_string()),
                    max_days_per_year: max,
                    requires_approval: req,
                },
            );
        }
        inner.next_id = 4;
        drop(inner);
        repo
    }
}

impl Default for InMemoryLeaveTypeRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LeaveTypeRepository for InMemoryLeaveTypeRepository {
    async fn create(&self, leave_type: LeaveType) -> Result<LeaveType, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let mut lt = leave_type;
        lt.id = id;
        inner.types.insert(id, lt.clone());
        Ok(lt)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<LeaveType>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.types.get(&id).cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<LeaveType>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .types
            .values()
            .filter(|t| t.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.types.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryPayrollRepository
struct InMemoryPayrollInner {
    records: std::collections::HashMap<i64, Payroll>,
    next_id: i64,
}

/// In-memory payroll repository
pub struct InMemoryPayrollRepository {
    inner: Mutex<InMemoryPayrollInner>,
}

impl InMemoryPayrollRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryPayrollInner {
                records: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryPayrollRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PayrollRepository for InMemoryPayrollRepository {
    async fn create(&self, payroll: Payroll) -> Result<Payroll, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let mut p = payroll;
        p.id = id;
        inner.records.insert(id, p.clone());
        Ok(p)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Payroll>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.records.get(&id).cloned())
    }

    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<Payroll>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .filter(|p| p.employee_id == employee_id)
            .cloned()
            .collect())
    }

    async fn find_by_period(
        &self,
        tenant_id: i64,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<Vec<Payroll>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .filter(|p| p.tenant_id == tenant_id && p.period_start >= start && p.period_end <= end)
            .cloned()
            .collect())
    }

    async fn update_status(&self, id: i64, status: PayrollStatus) -> Result<Payroll, ApiError> {
        let mut inner = self.inner.lock();
        let payroll = inner
            .records
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Payroll {} not found", id)))?;
        payroll.status = status;
        Ok(payroll.clone())
    }

    async fn mark_paid(&self, id: i64) -> Result<Payroll, ApiError> {
        let mut inner = self.inner.lock();
        let payroll = inner
            .records
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Payroll {} not found", id)))?;
        payroll.status = PayrollStatus::Paid;
        payroll.paid_at = Some(chrono::Utc::now());
        Ok(payroll.clone())
    }
}
