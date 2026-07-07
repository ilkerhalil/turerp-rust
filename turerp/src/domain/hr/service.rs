//! HR service for business logic
use chrono::Utc;
use rust_decimal::Decimal;

use crate::common::pagination::PaginatedResult;
use crate::domain::company::service::ensure_company_owned;
use crate::domain::company::BoxCompanyRepository;
use crate::domain::hr::model::{
    Attendance, CreateAttendance, CreateEmployee, CreateLeaveRequest, Employee, EmployeeResponse,
    EmployeeStatus, LeaveRequest, LeaveRequestStatus, LeaveType, Payroll, PayrollStatus,
};
use crate::domain::hr::repository::{
    BoxAttendanceRepository, BoxEmployeeRepository, BoxLeaveRequestRepository,
    BoxLeaveTypeRepository, BoxPayrollRepository,
};
use crate::domain::hr::sgk::calculator::{
    default_income_tax_brackets_2026, default_sgk_config_2026, PayrollCalculator,
};
use crate::error::ApiError;

/// HR service
#[derive(Clone)]
pub struct HrService {
    employee_repo: BoxEmployeeRepository,
    attendance_repo: BoxAttendanceRepository,
    leave_request_repo: BoxLeaveRequestRepository,
    leave_type_repo: BoxLeaveTypeRepository,
    payroll_repo: BoxPayrollRepository,
    company_repo: BoxCompanyRepository,
}

impl HrService {
    pub fn new(
        employee_repo: BoxEmployeeRepository,
        attendance_repo: BoxAttendanceRepository,
        leave_request_repo: BoxLeaveRequestRepository,
        leave_type_repo: BoxLeaveTypeRepository,
        payroll_repo: BoxPayrollRepository,
        company_repo: BoxCompanyRepository,
    ) -> Self {
        Self {
            employee_repo,
            attendance_repo,
            leave_request_repo,
            leave_type_repo,
            payroll_repo,
            company_repo,
        }
    }

    pub fn employee_repo(&self) -> &BoxEmployeeRepository {
        &self.employee_repo
    }

    pub fn payroll_repo(&self) -> &BoxPayrollRepository {
        &self.payroll_repo
    }

    // Employee operations
    #[tracing::instrument(skip(self))]
    pub async fn create_employee(
        &self,
        create: CreateEmployee,
    ) -> Result<EmployeeResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        // Parent-ownership precheck: body company_id must belong to the caller's
        // tenant (legacy `1` sentinel skipped for backward compat).
        ensure_company_owned(&self.company_repo, create.company_id, create.tenant_id).await?;
        let employee = self.employee_repo.create(create).await?;
        Ok(EmployeeResponse::from(employee))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_employee(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<EmployeeResponse, ApiError> {
        let employee = self
            .employee_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Employee {} not found", id)))?;
        Ok(EmployeeResponse::from(employee))
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_employees_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<EmployeeResponse>, ApiError> {
        let employees = self.employee_repo.find_by_tenant(tenant_id).await?;
        Ok(employees.into_iter().map(EmployeeResponse::from).collect())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_employees_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<EmployeeResponse>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        let result = self
            .employee_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await?;
        Ok(PaginatedResult::new(
            result
                .items
                .into_iter()
                .map(EmployeeResponse::from)
                .collect(),
            result.page,
            result.per_page,
            result.total,
        ))
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_employee_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: EmployeeStatus,
    ) -> Result<Employee, ApiError> {
        self.employee_repo
            .update_status(id, tenant_id, status)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn terminate_employee(&self, id: i64, tenant_id: i64) -> Result<Employee, ApiError> {
        self.employee_repo
            .update_status(id, tenant_id, EmployeeStatus::Terminated)
            .await
    }

    // Employee soft-delete operations
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_employee(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.employee_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_employee(&self, id: i64, tenant_id: i64) -> Result<Employee, ApiError> {
        self.employee_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_employees(&self, tenant_id: i64) -> Result<Vec<Employee>, ApiError> {
        self.employee_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_employee(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.employee_repo.destroy(id, tenant_id).await
    }

    // Attendance operations
    #[tracing::instrument(skip(self))]
    pub async fn record_attendance(
        &self,
        create: CreateAttendance,
    ) -> Result<Attendance, ApiError> {
        // Parent-ownership precheck: the employee must belong to the caller's
        // tenant, else a tenant-A caller could record attendance against
        // tenant-B's employee (cross-tenant orphan write). The handler forces
        // `create.tenant_id` from the auth token, so it is the auth-derived
        // tenant here. Also yields a clean NotFound for a bogus employee_id
        // instead of an FK violation.
        self.employee_repo
            .find_by_id(create.employee_id, create.tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Employee not found".to_string()))?;
        self.attendance_repo.create(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_attendance_by_employee(
        &self,
        employee_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Attendance>, ApiError> {
        self.attendance_repo
            .find_by_employee(employee_id, tenant_id)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_attendance_by_date(
        &self,
        date: chrono::DateTime<Utc>,
        tenant_id: i64,
    ) -> Result<Vec<Attendance>, ApiError> {
        self.attendance_repo.find_by_date(date, tenant_id).await
    }

    // Attendance soft-delete operations
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_attendance(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.attendance_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_attendance(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Attendance, ApiError> {
        self.attendance_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_attendance(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Attendance>, ApiError> {
        self.attendance_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_attendance(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.attendance_repo.destroy(id, tenant_id).await
    }

    // Leave operations
    #[tracing::instrument(skip(self))]
    pub async fn create_leave_request(
        &self,
        create: CreateLeaveRequest,
    ) -> Result<LeaveRequest, ApiError> {
        // Parent-ownership precheck: the employee and leave type must belong to
        // the caller's tenant, else a tenant-A caller could file a leave
        // request referencing tenant-B's employee/leave-type (cross-tenant
        // orphan write). The handler forces `create.tenant_id` from the auth
        // token. Also yields a clean NotFound for a bogus id instead of an FK
        // violation.
        self.employee_repo
            .find_by_id(create.employee_id, create.tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Employee not found".to_string()))?;
        self.leave_type_repo
            .find_by_id(create.leave_type_id, create.tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Leave type not found".to_string()))?;
        self.leave_request_repo.create(create).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_leave_requests_by_employee(
        &self,
        employee_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<LeaveRequest>, ApiError> {
        self.leave_request_repo
            .find_by_employee(employee_id, tenant_id)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn approve_leave_request(
        &self,
        id: i64,
        tenant_id: i64,
        approver_id: i64,
    ) -> Result<LeaveRequest, ApiError> {
        self.leave_request_repo
            .update_status(
                id,
                tenant_id,
                LeaveRequestStatus::Approved,
                Some(approver_id),
            )
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn reject_leave_request(
        &self,
        id: i64,
        tenant_id: i64,
        approver_id: i64,
    ) -> Result<LeaveRequest, ApiError> {
        self.leave_request_repo
            .update_status(
                id,
                tenant_id,
                LeaveRequestStatus::Rejected,
                Some(approver_id),
            )
            .await
    }

    // Leave request soft-delete operations
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_leave_request(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.leave_request_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_leave_request(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<LeaveRequest, ApiError> {
        self.leave_request_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_leave_requests(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<LeaveRequest>, ApiError> {
        self.leave_request_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_leave_request(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.leave_request_repo.destroy(id, tenant_id).await
    }

    // Leave type operations
    #[tracing::instrument(skip(self))]
    pub async fn get_leave_types(&self, tenant_id: i64) -> Result<Vec<LeaveType>, ApiError> {
        self.leave_type_repo.find_by_tenant(tenant_id).await
    }

    // Leave type soft-delete operations
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_leave_type(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.leave_type_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_leave_type(&self, id: i64, tenant_id: i64) -> Result<LeaveType, ApiError> {
        self.leave_type_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_leave_types(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<LeaveType>, ApiError> {
        self.leave_type_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_leave_type(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.leave_type_repo.destroy(id, tenant_id).await
    }

    // Payroll operations
    #[tracing::instrument(skip(self))]
    pub async fn calculate_payroll(
        &self,
        tenant_id: i64,
        employee_id: i64,
        period_start: chrono::DateTime<Utc>,
        period_end: chrono::DateTime<Utc>,
    ) -> Result<Payroll, ApiError> {
        let employee = self
            .employee_repo
            .find_by_id(employee_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Employee not found".to_string()))?;

        // Get attendance for the period to calculate overtime
        let attendance = self
            .attendance_repo
            .find_by_employee(employee_id, tenant_id)
            .await?;
        let overtime_hours: Decimal = attendance
            .iter()
            .filter(|a| {
                a.date >= period_start
                    && a.date <= period_end
                    && a.hours_worked > Decimal::new(8, 0)
            })
            .map(|a| a.hours_worked - Decimal::new(8, 0))
            .sum();

        let overtime_pay = overtime_hours * (employee.salary / Decimal::new(200, 0)); // Hourly rate
        let gross = employee.salary + overtime_pay;
        // FIXME: sum period bonuses from EmployeeBonusRepository; requires wiring BoxEmployeeBonusRepository into HrService::new and all call sites
        let bonuses = Decimal::ZERO;

        let config = default_sgk_config_2026();
        let brackets = default_income_tax_brackets_2026();
        let calculator = PayrollCalculator::new(config, brackets);

        let marital_status = employee.marital_status.as_deref().unwrap_or("single");
        let line = calculator.gross_to_net(
            gross,
            bonuses,
            marital_status,
            employee.children_count,
            employee.spouse_working,
        );

        let deductions = line.sgk_premium_worker
            + line.unemployment_premium_worker
            + line.income_tax
            + line.stamp_tax;

        let payroll = Payroll {
            id: 0,
            tenant_id: employee.tenant_id,
            employee_id,
            period_start,
            period_end,
            basic_salary: employee.salary,
            overtime_hours,
            overtime_pay,
            bonuses,
            gross_salary: line.gross_salary,
            sgk_premium: line.sgk_premium_worker,
            unemployment_premium: line.unemployment_premium_worker,
            income_tax: line.income_tax,
            stamp_tax: line.stamp_tax,
            agi: line.agi,
            sgk_earnings_base: line.sgk_earnings_base,
            total_employer_cost: line.employer_cost,
            deductions,
            net_salary: line.net_salary,
            status: PayrollStatus::Calculated,
            paid_at: None,
            created_at: chrono::Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };

        self.payroll_repo.create(payroll).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_payroll_by_employee(
        &self,
        employee_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<Payroll>, ApiError> {
        self.payroll_repo
            .find_by_employee(employee_id, tenant_id)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn mark_payroll_paid(&self, id: i64, tenant_id: i64) -> Result<Payroll, ApiError> {
        self.payroll_repo.mark_paid(id, tenant_id).await
    }

    // Payroll soft-delete operations
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_payroll(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.payroll_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn restore_payroll(&self, id: i64, tenant_id: i64) -> Result<Payroll, ApiError> {
        self.payroll_repo.restore(id, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_payroll(&self, tenant_id: i64) -> Result<Vec<Payroll>, ApiError> {
        self.payroll_repo.find_deleted(tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn destroy_payroll(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.payroll_repo.destroy(id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::company::repository::InMemoryCompanyRepository;
    use crate::domain::company::service::LEGACY_COMPANY_ID;
    use crate::domain::company::CreateCompany;
    use crate::domain::hr::repository::{
        InMemoryAttendanceRepository, InMemoryEmployeeRepository, InMemoryLeaveRequestRepository,
        InMemoryLeaveTypeRepository, InMemoryPayrollRepository,
    };
    use std::sync::Arc;

    async fn create_service() -> HrService {
        let employee_repo = Arc::new(InMemoryEmployeeRepository::new()) as BoxEmployeeRepository;
        let attendance_repo =
            Arc::new(InMemoryAttendanceRepository::new()) as BoxAttendanceRepository;
        let leave_request_repo =
            Arc::new(InMemoryLeaveRequestRepository::new()) as BoxLeaveRequestRepository;
        let leave_type_repo =
            Arc::new(InMemoryLeaveTypeRepository::new()) as BoxLeaveTypeRepository;
        let payroll_repo = Arc::new(InMemoryPayrollRepository::new()) as BoxPayrollRepository;
        let company_repo = Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository;
        // Seed a company per tenant so the InMemory auto-id counter yields id=1
        // for tenant-1 (the LEGACY_COMPANY_ID sentinel, skipped by the precheck)
        // and id=2 for tenant-2 (a non-sentinel foreign company the reject test
        // targets).
        for tenant in [1, 2] {
            company_repo
                .create(CreateCompany {
                    code: format!("CO{}", tenant),
                    name: format!("Tenant {} Co", tenant),
                    tax_number: None,
                    address: None,
                    city: None,
                    country: None,
                    currency: "TRY".to_string(),
                    tenant_id: tenant,
                })
                .await
                .expect("seed company");
        }
        HrService::new(
            employee_repo,
            attendance_repo,
            leave_request_repo,
            leave_type_repo,
            payroll_repo,
            company_repo,
        )
    }

    /// Returns the tenant-2 company id (a non-sentinel foreign company) for the
    /// reject test, guarding that the seeded id is not the LEGACY sentinel.
    async fn foreign_company_id(service: &HrService) -> i64 {
        let id = service
            .company_repo
            .find_by_tenant(2)
            .await
            .expect("list tenant-2 companies")
            .into_iter()
            .map(|c| c.id)
            .next()
            .expect("tenant-2 company seeded");
        assert_ne!(id, LEGACY_COMPANY_ID);
        id
    }

    /// Seed an employee on `tenant_id` and return its id. The InMemory repo
    /// auto-assigns ids starting at 1, so the first employee on tenant 1 is
    /// id 1 and the first on tenant 2 is id 2 — matching the cross-tenant
    /// IDOR negative tests below.
    async fn seed_employee(service: &HrService, tenant_id: i64) -> i64 {
        let employee = service
            .create_employee(CreateEmployee {
                tenant_id,
                company_id: 1,
                user_id: None,
                employee_number: format!("EMP-{}", tenant_id),
                first_name: "Test".to_string(),
                last_name: "User".to_string(),
                email: format!("emp{}@test.com", tenant_id),
                phone: None,
                department: None,
                position: None,
                hire_date: chrono::Utc::now(),
                salary: Decimal::ZERO,
                tc_kimlik_no: format!("1000000000{}", tenant_id),
                children_count: 0,
            })
            .await
            .expect("seed employee");
        employee.id
    }

    fn attendance(employee_id: i64, tenant_id: i64) -> CreateAttendance {
        CreateAttendance {
            employee_id,
            date: chrono::Utc::now(),
            check_in: Some(chrono::Utc::now()),
            check_out: Some(chrono::Utc::now() + chrono::Duration::hours(8)),
            notes: None,
            tenant_id,
        }
    }

    fn leave_request(employee_id: i64, leave_type_id: i64, tenant_id: i64) -> CreateLeaveRequest {
        CreateLeaveRequest {
            employee_id,
            leave_type_id,
            start_date: chrono::Utc::now(),
            end_date: chrono::Utc::now() + chrono::Duration::days(3),
            reason: None,
            tenant_id,
        }
    }

    #[tokio::test]
    async fn test_create_employee() {
        let service = create_service().await;
        let create = CreateEmployee {
            tenant_id: 1,
            company_id: 1,
            user_id: None,
            employee_number: "EMP001".to_string(),
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: "john@example.com".to_string(),
            phone: Some("+1234567890".to_string()),
            department: Some("IT".to_string()),
            position: Some("Developer".to_string()),
            hire_date: chrono::Utc::now(),
            salary: Decimal::new(500000, 2), // 5000.00
            tc_kimlik_no: "12345678901".to_string(),
            children_count: 0,
        };
        let result = service.create_employee(create).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().first_name, "John");
    }

    #[tokio::test]
    async fn test_record_attendance() {
        let service = create_service().await;
        let employee_id = seed_employee(&service, 1).await;
        let result = service.record_attendance(attendance(employee_id, 1)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_attendance_rejects_foreign_employee() {
        let service = create_service().await;
        // employee_id 1 exists only on tenant 1; tenant 2 has no such row.
        let employee_id = seed_employee(&service, 1).await;
        let result = service.record_attendance(attendance(employee_id, 2)).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_create_leave_request() {
        let service = create_service().await;
        let employee_id = seed_employee(&service, 1).await;
        // Default leave types are seeded on tenant 1 (ids 1-3).
        let result = service
            .create_leave_request(leave_request(employee_id, 1, 1))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_leave_request_rejects_foreign_employee() {
        let service = create_service().await;
        let employee_id = seed_employee(&service, 1).await;
        // employee belongs to tenant 1; tenant 2 caller cannot reference it.
        let result = service
            .create_leave_request(leave_request(employee_id, 1, 2))
            .await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_create_leave_request_rejects_foreign_leave_type() {
        let service = create_service().await;
        let employee_id = seed_employee(&service, 2).await;
        // leave_type_id 1 is seeded on tenant 1 only; tenant 2 has no leave
        // types, so the precheck must reject before INSERT.
        let result = service
            .create_leave_request(leave_request(employee_id, 1, 2))
            .await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_get_leave_types() {
        let service = create_service().await;
        let result = service.get_leave_types(1).await.unwrap();
        assert!(!result.is_empty());
    }

    /// Rejects an employee stamped onto a foreign-tenant company.
    #[tokio::test]
    async fn test_create_employee_rejects_foreign_company() {
        let service = create_service().await;
        let foreign = foreign_company_id(&service).await;
        let result = service
            .create_employee(CreateEmployee {
                tenant_id: 1,
                company_id: foreign,
                user_id: None,
                employee_number: "EMP-FOR".to_string(),
                first_name: "Foreign".to_string(),
                last_name: "Stamp".to_string(),
                email: "foreign@test.com".to_string(),
                phone: None,
                department: None,
                position: None,
                hire_date: chrono::Utc::now(),
                salary: Decimal::ZERO,
                tc_kimlik_no: "10000000009".to_string(),
                children_count: 0,
            })
            .await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "expected NotFound for foreign company_id, got {:?}",
            result
        );
    }
}
