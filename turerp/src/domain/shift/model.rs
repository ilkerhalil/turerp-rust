//! Shift Planning domain models

use chrono::{DateTime, NaiveTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Shift schedule definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Shift {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub shift_type: ShiftType,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub break_duration_minutes: i32,
    pub expected_hours: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for Shift {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

/// Shift type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum ShiftType {
    Morning,
    Evening,
    Night,
    Custom,
}

impl std::fmt::Display for ShiftType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShiftType::Morning => write!(f, "Morning"),
            ShiftType::Evening => write!(f, "Evening"),
            ShiftType::Night => write!(f, "Night"),
            ShiftType::Custom => write!(f, "Custom"),
        }
    }
}

impl std::str::FromStr for ShiftType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Morning" => Ok(ShiftType::Morning),
            "Evening" => Ok(ShiftType::Evening),
            "Night" => Ok(ShiftType::Night),
            "Custom" => Ok(ShiftType::Custom),
            _ => Err(format!("Invalid shift type: {}", s)),
        }
    }
}

/// Shift assignment for an employee
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShiftAssignment {
    pub id: i64,
    pub shift_id: i64,
    pub employee_id: i64,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for ShiftAssignment {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

/// Attendance record with clock in/out
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttendanceRecord {
    pub id: i64,
    pub employee_id: i64,
    pub shift_id: i64,
    pub date: DateTime<Utc>,
    pub clock_in: Option<DateTime<Utc>>,
    pub clock_out: Option<DateTime<Utc>>,
    pub hours_worked: Decimal,
    pub overtime_hours: Decimal,
    pub status: AttendanceStatus,
    pub notes: Option<String>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for AttendanceRecord {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

/// Attendance status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum AttendanceStatus {
    Present,
    Absent,
    Late,
    EarlyLeave,
    OnLeave,
    Holiday,
}

impl std::fmt::Display for AttendanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttendanceStatus::Present => write!(f, "Present"),
            AttendanceStatus::Absent => write!(f, "Absent"),
            AttendanceStatus::Late => write!(f, "Late"),
            AttendanceStatus::EarlyLeave => write!(f, "EarlyLeave"),
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
            "EarlyLeave" => Ok(AttendanceStatus::EarlyLeave),
            "OnLeave" => Ok(AttendanceStatus::OnLeave),
            "Holiday" => Ok(AttendanceStatus::Holiday),
            _ => Err(format!("Invalid attendance status: {}", s)),
        }
    }
}

/// Overtime calculation result for a period
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OvertimeCalculation {
    pub employee_id: i64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub regular_hours: Decimal,
    pub overtime_hours: Decimal,
    pub overtime_rate: Decimal,
    pub overtime_pay: Decimal,
}

/// Shift report per employee per period
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShiftReport {
    pub employee_id: i64,
    pub employee_name: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_hours: Decimal,
    pub regular_hours: Decimal,
    pub overtime_hours: Decimal,
    pub absent_days: i32,
    pub late_days: i32,
    pub early_leave_days: i32,
}

/// Create shift request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateShift {
    pub tenant_id: i64,
    pub name: String,
    pub shift_type: ShiftType,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub break_duration_minutes: i32,
    pub expected_hours: Decimal,
}

impl CreateShift {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("Shift name is required".to_string());
        }
        if self.expected_hours <= Decimal::ZERO {
            errors.push("Expected hours must be positive".to_string());
        }
        if self.break_duration_minutes < 0 {
            errors.push("Break duration cannot be negative".to_string());
        }
        if self.end_time <= self.start_time {
            errors.push("End time must be after start time".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update shift request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateShift {
    pub name: Option<String>,
    pub shift_type: Option<ShiftType>,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub break_duration_minutes: Option<i32>,
    pub expected_hours: Option<Decimal>,
    pub is_active: Option<bool>,
}

/// Create shift assignment request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateShiftAssignment {
    pub shift_id: i64,
    pub employee_id: i64,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
}

impl CreateShiftAssignment {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if let Some(end) = self.end_date {
            if end < self.start_date {
                errors.push("End date must be after start date".to_string());
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Clock in request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClockInRequest {
    pub employee_id: i64,
    pub shift_id: i64,
    pub timestamp: DateTime<Utc>,
    pub notes: Option<String>,
}

/// Clock out request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClockOutRequest {
    pub employee_id: i64,
    pub timestamp: DateTime<Utc>,
    pub notes: Option<String>,
}

/// Shift report query parameters
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShiftReportQuery {
    pub employee_id: Option<i64>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Shift response (without soft-delete fields)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ShiftResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub shift_type: ShiftType,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub break_duration_minutes: i32,
    pub expected_hours: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Shift> for ShiftResponse {
    fn from(s: Shift) -> Self {
        Self {
            id: s.id,
            tenant_id: s.tenant_id,
            name: s.name,
            shift_type: s.shift_type,
            start_time: s.start_time,
            end_time: s.end_time,
            break_duration_minutes: s.break_duration_minutes,
            expected_hours: s.expected_hours,
            is_active: s.is_active,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

/// Attendance record response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttendanceRecordResponse {
    pub id: i64,
    pub employee_id: i64,
    pub shift_id: i64,
    pub date: DateTime<Utc>,
    pub clock_in: Option<DateTime<Utc>>,
    pub clock_out: Option<DateTime<Utc>>,
    pub hours_worked: Decimal,
    pub overtime_hours: Decimal,
    pub status: AttendanceStatus,
    pub notes: Option<String>,
}

impl From<AttendanceRecord> for AttendanceRecordResponse {
    fn from(r: AttendanceRecord) -> Self {
        Self {
            id: r.id,
            employee_id: r.employee_id,
            shift_id: r.shift_id,
            date: r.date,
            clock_in: r.clock_in,
            clock_out: r.clock_out,
            hours_worked: r.hours_worked,
            overtime_hours: r.overtime_hours,
            status: r.status,
            notes: r.notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;
    use rust_decimal_macros::dec;

    #[test]
    fn test_create_shift_validation() {
        let valid = CreateShift {
            tenant_id: 1,
            name: "Morning Shift".to_string(),
            shift_type: ShiftType::Morning,
            start_time: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            break_duration_minutes: 60,
            expected_hours: dec!(8),
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateShift {
            tenant_id: 1,
            name: "".to_string(),
            shift_type: ShiftType::Custom,
            start_time: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            break_duration_minutes: -10,
            expected_hours: dec!(0),
        };
        let err = invalid.validate().unwrap_err();
        assert_eq!(err.len(), 4);
    }

    #[test]
    fn test_shift_type_display() {
        assert_eq!(ShiftType::Morning.to_string(), "Morning");
        assert_eq!(ShiftType::Night.to_string(), "Night");
    }

    #[test]
    fn test_attendance_status_parse() {
        assert_eq!(
            "EarlyLeave".parse::<AttendanceStatus>().unwrap(),
            AttendanceStatus::EarlyLeave
        );
        assert!("Invalid".parse::<AttendanceStatus>().is_err());
    }
}
