//! HR domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Employee entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    pub id: i64,
    pub tenant_id: i64,
    pub user_id: Option<i64>,
    pub employee_number: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub department: Option<String>,
    pub position: Option<String>,
    pub hire_date: DateTime<Utc>,
    pub termination_date: Option<DateTime<Utc>>,
    pub status: EmployeeStatus,
    pub salary: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Employee status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmployeeStatus {
    Active,
    OnLeave,
    Terminated,
    Suspended,
}

/// Attendance record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendance {
    pub id: i64,
    pub employee_id: i64,
    pub date: DateTime<Utc>,
    pub check_in: Option<DateTime<Utc>>,
    pub check_out: Option<DateTime<Utc>>,
    pub hours_worked: f64,
    pub status: AttendanceStatus,
    pub notes: Option<String>,
}

/// Attendance status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttendanceStatus {
    Present,
    Absent,
    Late,
    OnLeave,
    Holiday,
}

/// Leave type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveType {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub max_days_per_year: f64,
    pub requires_approval: bool,
}

/// Leave request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRequest {
    pub id: i64,
    pub employee_id: i64,
    pub leave_type_id: i64,
    pub status: LeaveRequestStatus,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub total_days: f64,
    pub reason: Option<String>,
    pub approved_by: Option<i64>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Leave request status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LeaveRequestStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

/// Payroll record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payroll {
    pub id: i64,
    pub tenant_id: i64,
    pub employee_id: i64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub basic_salary: f64,
    pub overtime_hours: f64,
    pub overtime_pay: f64,
    pub bonuses: f64,
    pub deductions: f64,
    pub net_salary: f64,
    pub status: PayrollStatus,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Payroll status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PayrollStatus {
    Draft,
    Calculated,
    Approved,
    Paid,
}

/// Create employee request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEmployee {
    pub tenant_id: i64,
    pub user_id: Option<i64>,
    pub employee_number: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub department: Option<String>,
    pub position: Option<String>,
    pub hire_date: DateTime<Utc>,
    pub salary: f64,
}

impl CreateEmployee {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.employee_number.trim().is_empty() {
            errors.push("Employee number is required".to_string());
        }
        if self.first_name.trim().is_empty() {
            errors.push("First name is required".to_string());
        }
        if self.last_name.trim().is_empty() {
            errors.push("Last name is required".to_string());
        }
        if self.email.trim().is_empty() {
            errors.push("Email is required".to_string());
        }
        if self.salary < 0.0 {
            errors.push("Salary cannot be negative".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create attendance request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAttendance {
    pub employee_id: i64,
    pub date: DateTime<Utc>,
    pub check_in: Option<DateTime<Utc>>,
    pub check_out: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

impl CreateAttendance {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.check_in.is_some()
            && self.check_out.is_some()
            && self.check_out.unwrap() <= self.check_in.unwrap()
        {
            errors.push("Check out must be after check in".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn calculate_hours(&self) -> f64 {
        match (self.check_in, self.check_out) {
            (Some(in_time), Some(out_time)) => {
                let duration = out_time.signed_duration_since(in_time);
                duration.num_seconds() as f64 / 3600.0
            }
            _ => 0.0,
        }
    }
}

/// Create leave request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLeaveRequest {
    pub employee_id: i64,
    pub leave_type_id: i64,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub reason: Option<String>,
}

impl CreateLeaveRequest {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.end_date < self.start_date {
            errors.push("End date must be after start date".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn calculate_total_days(&self) -> f64 {
        let duration = self.end_date.signed_duration_since(self.start_date);
        duration.num_days() as f64 + 1.0
    }
}

/// Employee response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeResponse {
    pub id: i64,
    pub employee_number: String,
    pub first_name: String,
    pub last_name: String,
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub department: Option<String>,
    pub position: Option<String>,
    pub hire_date: DateTime<Utc>,
    pub status: EmployeeStatus,
    pub salary: f64,
}

impl From<Employee> for EmployeeResponse {
    fn from(e: Employee) -> Self {
        let first_name = e.first_name.clone();
        let last_name = e.last_name.clone();
        Self {
            id: e.id,
            employee_number: e.employee_number,
            first_name: first_name.clone(),
            last_name: last_name.clone(),
            full_name: format!("{} {}", first_name, last_name),
            email: e.email,
            phone: e.phone,
            department: e.department,
            position: e.position,
            hire_date: e.hire_date,
            status: e.status,
            salary: e.salary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_employee_validation() {
        let valid = CreateEmployee {
            tenant_id: 1,
            user_id: None,
            employee_number: "EMP001".to_string(),
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: "john@example.com".to_string(),
            phone: Some("+1234567890".to_string()),
            department: Some("IT".to_string()),
            position: Some("Developer".to_string()),
            hire_date: Utc::now(),
            salary: 5000.0,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateEmployee {
            tenant_id: 1,
            user_id: None,
            employee_number: "".to_string(),
            first_name: "".to_string(),
            last_name: "Doe".to_string(),
            email: "john@example.com".to_string(),
            phone: None,
            department: None,
            position: None,
            hire_date: Utc::now(),
            salary: 5000.0,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_attendance_hours() {
        let attendance = CreateAttendance {
            employee_id: 1,
            date: Utc::now(),
            check_in: Some(Utc::now()),
            check_out: Some(Utc::now() + chrono::Duration::hours(8)),
            notes: None,
        };
        let hours = attendance.calculate_hours();
        assert!(hours >= 7.9 && hours <= 8.1);
    }
}
