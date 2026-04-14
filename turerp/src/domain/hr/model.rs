//! HR domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
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
    pub salary: Decimal,
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

impl std::fmt::Display for EmployeeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmployeeStatus::Active => write!(f, "Active"),
            EmployeeStatus::OnLeave => write!(f, "OnLeave"),
            EmployeeStatus::Terminated => write!(f, "Terminated"),
            EmployeeStatus::Suspended => write!(f, "Suspended"),
        }
    }
}

impl std::str::FromStr for EmployeeStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Active" => Ok(EmployeeStatus::Active),
            "OnLeave" => Ok(EmployeeStatus::OnLeave),
            "Terminated" => Ok(EmployeeStatus::Terminated),
            "Suspended" => Ok(EmployeeStatus::Suspended),
            _ => Err(format!("Invalid employee status: {}", s)),
        }
    }
}

/// Attendance record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendance {
    pub id: i64,
    pub employee_id: i64,
    pub date: DateTime<Utc>,
    pub check_in: Option<DateTime<Utc>>,
    pub check_out: Option<DateTime<Utc>>,
    pub hours_worked: Decimal,
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

impl std::fmt::Display for AttendanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttendanceStatus::Present => write!(f, "Present"),
            AttendanceStatus::Absent => write!(f, "Absent"),
            AttendanceStatus::Late => write!(f, "Late"),
            AttendanceStatus::OnLeave => write!(f, "OnLeave"),
            AttendanceStatus::Holiday => write!(f, "Holiday"),
        }
    }
}

impl std::str::FromStr for AttendanceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Present" => Ok(AttendanceStatus::Present),
            "Absent" => Ok(AttendanceStatus::Absent),
            "Late" => Ok(AttendanceStatus::Late),
            "OnLeave" => Ok(AttendanceStatus::OnLeave),
            "Holiday" => Ok(AttendanceStatus::Holiday),
            _ => Err(format!("Invalid attendance status: {}", s)),
        }
    }
}

/// Leave type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveType {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub max_days_per_year: Decimal,
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
    pub total_days: Decimal,
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

impl std::fmt::Display for LeaveRequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeaveRequestStatus::Pending => write!(f, "Pending"),
            LeaveRequestStatus::Approved => write!(f, "Approved"),
            LeaveRequestStatus::Rejected => write!(f, "Rejected"),
            LeaveRequestStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl std::str::FromStr for LeaveRequestStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pending" => Ok(LeaveRequestStatus::Pending),
            "Approved" => Ok(LeaveRequestStatus::Approved),
            "Rejected" => Ok(LeaveRequestStatus::Rejected),
            "Cancelled" => Ok(LeaveRequestStatus::Cancelled),
            _ => Err(format!("Invalid leave request status: {}", s)),
        }
    }
}

/// Payroll record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payroll {
    pub id: i64,
    pub tenant_id: i64,
    pub employee_id: i64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub basic_salary: Decimal,
    pub overtime_hours: Decimal,
    pub overtime_pay: Decimal,
    pub bonuses: Decimal,
    pub deductions: Decimal,
    pub net_salary: Decimal,
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

impl std::fmt::Display for PayrollStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PayrollStatus::Draft => write!(f, "Draft"),
            PayrollStatus::Calculated => write!(f, "Calculated"),
            PayrollStatus::Approved => write!(f, "Approved"),
            PayrollStatus::Paid => write!(f, "Paid"),
        }
    }
}

impl std::str::FromStr for PayrollStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(PayrollStatus::Draft),
            "Calculated" => Ok(PayrollStatus::Calculated),
            "Approved" => Ok(PayrollStatus::Approved),
            "Paid" => Ok(PayrollStatus::Paid),
            _ => Err(format!("Invalid payroll status: {}", s)),
        }
    }
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
    pub salary: Decimal,
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
        if self.salary < Decimal::ZERO {
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
        if let (Some(check_in), Some(check_out)) = (self.check_in, self.check_out) {
            if check_out <= check_in {
                errors.push("Check out must be after check in".to_string());
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn calculate_hours(&self) -> Decimal {
        match (self.check_in, self.check_out) {
            (Some(in_time), Some(out_time)) => {
                let duration = out_time.signed_duration_since(in_time);
                Decimal::from(duration.num_seconds()) / Decimal::from(3600)
            }
            _ => Decimal::ZERO,
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

    pub fn calculate_total_days(&self) -> Decimal {
        let duration = self.end_date.signed_duration_since(self.start_date);
        Decimal::from(duration.num_days()) + Decimal::ONE
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
    pub salary: Decimal,
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
            salary: Decimal::new(500000, 2), // 5000.00
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
            salary: Decimal::new(500000, 2), // 5000.00
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_create_attendance_hours() {
        use rust_decimal_macros::dec;

        let check_in = Utc::now();
        let attendance = CreateAttendance {
            employee_id: 1,
            date: check_in,
            check_in: Some(check_in),
            check_out: Some(check_in + chrono::Duration::hours(8)),
            notes: None,
        };
        let hours = attendance.calculate_hours();
        // 8 hours with small tolerance for test timing
        assert!(hours >= dec!(7.9) && hours <= dec!(8.1));
    }
}
