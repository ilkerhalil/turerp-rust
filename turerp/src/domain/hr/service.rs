//! HR service for business logic
use chrono::Utc;
use rust_decimal::Decimal;

use crate::common::pagination::PaginatedResult;
use crate::domain::hr::model::{
    Attendance, CreateAttendance, CreateEmployee, CreateLeaveRequest, Employee, EmployeeResponse,
    EmployeeStatus, LeaveRequest, LeaveRequestStatus, LeaveType, Payroll, PayrollStatus,
};
use crate::domain::hr::repository::{
    BoxAttendanceRepository, BoxEmployeeRepository, BoxLeaveRequestRepository,
    BoxLeaveTypeRepository, BoxPayrollRepository,
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
}

impl HrService {
    pub fn new(
        employee_repo: BoxEmployeeRepository,
        attendance_repo: BoxAttendanceRepository,
        leave_request_repo: BoxLeaveRequestRepository,
        leave_type_repo: BoxLeaveTypeRepository,
        payroll_repo: BoxPayrollRepository,
    ) -> Self {
        Self {
            employee_repo,
            attendance_repo,
            leave_request_repo,
            leave_type_repo,
            payroll_repo,
        }
    }

    // Employee operations
    pub async fn create_employee(
        &self,
        create: CreateEmployee,
    ) -> Result<EmployeeResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let employee = self.employee_repo.create(create).await?;
        Ok(EmployeeResponse::from(employee))
    }

    pub async fn get_employee(&self, id: i64) -> Result<EmployeeResponse, ApiError> {
        let employee = self
            .employee_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Employee {} not found", id)))?;
        Ok(EmployeeResponse::from(employee))
    }

    pub async fn get_employees_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<EmployeeResponse>, ApiError> {
        let employees = self.employee_repo.find_by_tenant(tenant_id).await?;
        Ok(employees.into_iter().map(EmployeeResponse::from).collect())
    }

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

    pub async fn update_employee_status(
        &self,
        id: i64,
        status: EmployeeStatus,
    ) -> Result<Employee, ApiError> {
        self.employee_repo.update_status(id, status).await
    }

    pub async fn terminate_employee(&self, id: i64) -> Result<Employee, ApiError> {
        self.employee_repo
            .update_status(id, EmployeeStatus::Terminated)
            .await
    }

    // Attendance operations
    pub async fn record_attendance(
        &self,
        create: CreateAttendance,
    ) -> Result<Attendance, ApiError> {
        self.attendance_repo.create(create).await
    }

    pub async fn get_attendance_by_employee(
        &self,
        employee_id: i64,
    ) -> Result<Vec<Attendance>, ApiError> {
        self.attendance_repo.find_by_employee(employee_id).await
    }

    pub async fn get_attendance_by_date(
        &self,
        date: chrono::DateTime<Utc>,
    ) -> Result<Vec<Attendance>, ApiError> {
        self.attendance_repo.find_by_date(date).await
    }

    // Leave operations
    pub async fn create_leave_request(
        &self,
        create: CreateLeaveRequest,
    ) -> Result<LeaveRequest, ApiError> {
        self.leave_request_repo.create(create).await
    }

    pub async fn get_leave_requests_by_employee(
        &self,
        employee_id: i64,
    ) -> Result<Vec<LeaveRequest>, ApiError> {
        self.leave_request_repo.find_by_employee(employee_id).await
    }

    pub async fn approve_leave_request(
        &self,
        id: i64,
        approver_id: i64,
    ) -> Result<LeaveRequest, ApiError> {
        self.leave_request_repo
            .update_status(id, LeaveRequestStatus::Approved, Some(approver_id))
            .await
    }

    pub async fn reject_leave_request(
        &self,
        id: i64,
        approver_id: i64,
    ) -> Result<LeaveRequest, ApiError> {
        self.leave_request_repo
            .update_status(id, LeaveRequestStatus::Rejected, Some(approver_id))
            .await
    }

    // Leave type operations
    pub async fn get_leave_types(&self, tenant_id: i64) -> Result<Vec<LeaveType>, ApiError> {
        self.leave_type_repo.find_by_tenant(tenant_id).await
    }

    // Payroll operations
    pub async fn calculate_payroll(
        &self,
        employee_id: i64,
        period_start: chrono::DateTime<Utc>,
        period_end: chrono::DateTime<Utc>,
    ) -> Result<Payroll, ApiError> {
        let employee = self
            .employee_repo
            .find_by_id(employee_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Employee not found".to_string()))?;

        // Get attendance for the period to calculate overtime
        let attendance = self.attendance_repo.find_by_employee(employee_id).await?;
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
        let deductions = gross * Decimal::new(20, 2); // Simplified tax calculation (0.20)
        let net = gross - deductions;

        let payroll = Payroll {
            id: 0,
            tenant_id: employee.tenant_id,
            employee_id,
            period_start,
            period_end,
            basic_salary: employee.salary,
            overtime_hours,
            overtime_pay,
            bonuses: Decimal::ZERO,
            deductions,
            net_salary: net,
            status: PayrollStatus::Calculated,
            paid_at: None,
            created_at: chrono::Utc::now(),
        };

        self.payroll_repo.create(payroll).await
    }

    pub async fn get_payroll_by_employee(
        &self,
        employee_id: i64,
    ) -> Result<Vec<Payroll>, ApiError> {
        self.payroll_repo.find_by_employee(employee_id).await
    }

    pub async fn mark_payroll_paid(&self, id: i64) -> Result<Payroll, ApiError> {
        self.payroll_repo.mark_paid(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::hr::repository::{
        InMemoryAttendanceRepository, InMemoryEmployeeRepository, InMemoryLeaveRequestRepository,
        InMemoryLeaveTypeRepository, InMemoryPayrollRepository,
    };
    use std::sync::Arc;

    fn create_service() -> HrService {
        let employee_repo = Arc::new(InMemoryEmployeeRepository::new()) as BoxEmployeeRepository;
        let attendance_repo =
            Arc::new(InMemoryAttendanceRepository::new()) as BoxAttendanceRepository;
        let leave_request_repo =
            Arc::new(InMemoryLeaveRequestRepository::new()) as BoxLeaveRequestRepository;
        let leave_type_repo =
            Arc::new(InMemoryLeaveTypeRepository::new()) as BoxLeaveTypeRepository;
        let payroll_repo = Arc::new(InMemoryPayrollRepository::new()) as BoxPayrollRepository;
        HrService::new(
            employee_repo,
            attendance_repo,
            leave_request_repo,
            leave_type_repo,
            payroll_repo,
        )
    }

    #[tokio::test]
    async fn test_create_employee() {
        let service = create_service();
        let create = CreateEmployee {
            tenant_id: 1,
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
        };
        let result = service.create_employee(create).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().first_name, "John");
    }

    #[tokio::test]
    async fn test_record_attendance() {
        let service = create_service();
        let create = CreateAttendance {
            employee_id: 1,
            date: chrono::Utc::now(),
            check_in: Some(chrono::Utc::now()),
            check_out: Some(chrono::Utc::now() + chrono::Duration::hours(8)),
            notes: None,
        };
        let result = service.record_attendance(create).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_leave_types() {
        let service = create_service();
        let result = service.get_leave_types(1).await.unwrap();
        assert!(!result.is_empty());
    }
}
