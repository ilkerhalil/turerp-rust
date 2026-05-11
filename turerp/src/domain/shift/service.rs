//! Shift Planning service for business logic

use chrono::Utc;
use rust_decimal::Decimal;

use crate::common::pagination::PaginatedResult;
use crate::domain::shift::model::{
    AttendanceRecordResponse, AttendanceStatus, ClockInRequest, ClockOutRequest, CreateShift,
    CreateShiftAssignment, OvertimeCalculation, ShiftAssignment, ShiftReport, ShiftReportQuery,
    ShiftResponse, UpdateShift,
};
use crate::domain::shift::repository::{
    BoxAttendanceRecordRepository, BoxShiftAssignmentRepository, BoxShiftRepository,
};
use crate::error::ApiError;

/// Shift service
#[derive(Clone)]
pub struct ShiftService {
    shift_repo: BoxShiftRepository,
    assignment_repo: BoxShiftAssignmentRepository,
    attendance_repo: BoxAttendanceRecordRepository,
}

impl ShiftService {
    pub fn new(
        shift_repo: BoxShiftRepository,
        assignment_repo: BoxShiftAssignmentRepository,
        attendance_repo: BoxAttendanceRecordRepository,
    ) -> Self {
        Self {
            shift_repo,
            assignment_repo,
            attendance_repo,
        }
    }

    // Shift operations
    pub async fn create_shift(&self, create: CreateShift) -> Result<ShiftResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let shift = self.shift_repo.create(create).await?;
        Ok(ShiftResponse::from(shift))
    }

    pub async fn get_shift(&self, id: i64, tenant_id: i64) -> Result<ShiftResponse, ApiError> {
        let shift = self
            .shift_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Shift {} not found", id)))?;
        Ok(ShiftResponse::from(shift))
    }

    pub async fn get_shifts_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<ShiftResponse>, ApiError> {
        let shifts = self.shift_repo.find_by_tenant(tenant_id).await?;
        Ok(shifts.into_iter().map(ShiftResponse::from).collect())
    }

    pub async fn get_shifts_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<ShiftResponse>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        let result = self
            .shift_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await?;
        Ok(PaginatedResult::new(
            result.items.into_iter().map(ShiftResponse::from).collect(),
            result.page,
            result.per_page,
            result.total,
        ))
    }

    pub async fn update_shift(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateShift,
    ) -> Result<ShiftResponse, ApiError> {
        let shift = self.shift_repo.update(id, tenant_id, update).await?;
        Ok(ShiftResponse::from(shift))
    }

    pub async fn delete_shift(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.shift_repo.delete(id, tenant_id).await
    }

    // Shift soft-delete operations
    pub async fn soft_delete_shift(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.shift_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    pub async fn restore_shift(&self, id: i64, tenant_id: i64) -> Result<ShiftResponse, ApiError> {
        let shift = self.shift_repo.restore(id, tenant_id).await?;
        Ok(ShiftResponse::from(shift))
    }

    pub async fn list_deleted_shifts(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<ShiftResponse>, ApiError> {
        let shifts = self.shift_repo.find_deleted(tenant_id).await?;
        Ok(shifts.into_iter().map(ShiftResponse::from).collect())
    }

    pub async fn destroy_shift(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.shift_repo.destroy(id, tenant_id).await
    }

    // Shift assignment operations
    pub async fn assign_employee(
        &self,
        assignment: CreateShiftAssignment,
    ) -> Result<ShiftAssignment, ApiError> {
        self.assignment_repo.create(assignment).await
    }

    pub async fn get_assignments_by_employee(
        &self,
        employee_id: i64,
    ) -> Result<Vec<ShiftAssignment>, ApiError> {
        self.assignment_repo.find_by_employee(employee_id).await
    }

    pub async fn get_assignments_by_shift(
        &self,
        shift_id: i64,
    ) -> Result<Vec<ShiftAssignment>, ApiError> {
        self.assignment_repo.find_by_shift(shift_id).await
    }

    pub async fn remove_assignment(&self, id: i64) -> Result<(), ApiError> {
        self.assignment_repo.delete(id).await
    }

    pub async fn soft_delete_assignment(&self, id: i64, deleted_by: i64) -> Result<(), ApiError> {
        self.assignment_repo.soft_delete(id, deleted_by).await
    }

    pub async fn restore_assignment(&self, id: i64) -> Result<ShiftAssignment, ApiError> {
        self.assignment_repo.restore(id).await
    }

    pub async fn list_deleted_assignments(&self) -> Result<Vec<ShiftAssignment>, ApiError> {
        self.assignment_repo.find_deleted().await
    }

    pub async fn destroy_assignment(&self, id: i64) -> Result<(), ApiError> {
        self.assignment_repo.destroy(id).await
    }

    // Attendance tracking operations
    pub async fn clock_in(
        &self,
        req: ClockInRequest,
    ) -> Result<AttendanceRecordResponse, ApiError> {
        let record = self.attendance_repo.clock_in(req).await?;
        Ok(AttendanceRecordResponse::from(record))
    }

    pub async fn clock_out(
        &self,
        req: ClockOutRequest,
    ) -> Result<AttendanceRecordResponse, ApiError> {
        let record = self.attendance_repo.clock_out(req).await?;
        Ok(AttendanceRecordResponse::from(record))
    }

    pub async fn get_attendance_by_employee(
        &self,
        employee_id: i64,
    ) -> Result<Vec<AttendanceRecordResponse>, ApiError> {
        let records = self.attendance_repo.find_by_employee(employee_id).await?;
        Ok(records
            .into_iter()
            .map(AttendanceRecordResponse::from)
            .collect())
    }

    pub async fn get_attendance_by_period(
        &self,
        employee_id: i64,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<Vec<AttendanceRecordResponse>, ApiError> {
        let records = self
            .attendance_repo
            .find_by_period(employee_id, start, end)
            .await?;
        Ok(records
            .into_iter()
            .map(AttendanceRecordResponse::from)
            .collect())
    }

    pub async fn delete_attendance(&self, id: i64) -> Result<(), ApiError> {
        self.attendance_repo.delete(id).await
    }

    pub async fn soft_delete_attendance(&self, id: i64, deleted_by: i64) -> Result<(), ApiError> {
        self.attendance_repo.soft_delete(id, deleted_by).await
    }

    pub async fn restore_attendance(&self, id: i64) -> Result<AttendanceRecordResponse, ApiError> {
        let record = self.attendance_repo.restore(id).await?;
        Ok(AttendanceRecordResponse::from(record))
    }

    pub async fn list_deleted_attendance(&self) -> Result<Vec<AttendanceRecordResponse>, ApiError> {
        let records = self.attendance_repo.find_deleted().await?;
        Ok(records
            .into_iter()
            .map(AttendanceRecordResponse::from)
            .collect())
    }

    pub async fn destroy_attendance(&self, id: i64) -> Result<(), ApiError> {
        self.attendance_repo.destroy(id).await
    }

    // Overtime calculation
    pub async fn calculate_overtime(
        &self,
        employee_id: i64,
        period_start: chrono::DateTime<Utc>,
        period_end: chrono::DateTime<Utc>,
        expected_hours_per_day: Decimal,
        overtime_rate: Decimal,
    ) -> Result<OvertimeCalculation, ApiError> {
        let records = self
            .attendance_repo
            .find_by_period(employee_id, period_start, period_end)
            .await?;

        let mut regular_hours = Decimal::ZERO;
        let mut overtime_hours = Decimal::ZERO;

        for record in records {
            if record.status == AttendanceStatus::Present || record.status == AttendanceStatus::Late
            {
                let daily_regular = record.hours_worked.min(expected_hours_per_day);
                let daily_overtime =
                    (record.hours_worked - expected_hours_per_day).max(Decimal::ZERO);
                regular_hours += daily_regular;
                overtime_hours += daily_overtime;
            }
        }

        let overtime_pay = overtime_hours * overtime_rate;

        Ok(OvertimeCalculation {
            employee_id,
            period_start,
            period_end,
            regular_hours,
            overtime_hours,
            overtime_rate,
            overtime_pay,
        })
    }

    // Shift report generation
    pub async fn generate_shift_report(
        &self,
        _tenant_id: i64,
        query: ShiftReportQuery,
    ) -> Result<Vec<ShiftReport>, ApiError> {
        let records = if let Some(employee_id) = query.employee_id {
            self.attendance_repo
                .find_by_period(employee_id, query.period_start, query.period_end)
                .await?
        } else {
            return Err(ApiError::BadRequest(
                "Employee ID is required for shift report".to_string(),
            ));
        };

        let mut total_hours = Decimal::ZERO;
        let mut regular_hours = Decimal::ZERO;
        let mut absent_days = 0i32;
        let mut late_days = 0i32;
        let mut early_leave_days = 0i32;

        for record in &records {
            total_hours += record.hours_worked;
            match record.status {
                AttendanceStatus::Present => regular_hours += record.hours_worked,
                AttendanceStatus::Absent => absent_days += 1,
                AttendanceStatus::Late => late_days += 1,
                AttendanceStatus::EarlyLeave => early_leave_days += 1,
                _ => {}
            }
        }

        let expected_hours = Decimal::new(8, 0) * Decimal::from(records.len() as i64);
        let overtime_hours = (total_hours - expected_hours).max(Decimal::ZERO);

        let report = ShiftReport {
            employee_id: query.employee_id.unwrap_or(0),
            employee_name: "".to_string(),
            period_start: query.period_start,
            period_end: query.period_end,
            total_hours,
            regular_hours,
            overtime_hours,
            absent_days,
            late_days,
            early_leave_days,
        };

        Ok(vec![report])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::shift::repository::{
        InMemoryAttendanceRecordRepository, InMemoryShiftAssignmentRepository,
        InMemoryShiftRepository,
    };
    use chrono::NaiveTime;
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_service() -> ShiftService {
        let shift_repo = Arc::new(InMemoryShiftRepository::new()) as BoxShiftRepository;
        let assignment_repo =
            Arc::new(InMemoryShiftAssignmentRepository::new()) as BoxShiftAssignmentRepository;
        let attendance_repo =
            Arc::new(InMemoryAttendanceRecordRepository::new()) as BoxAttendanceRecordRepository;
        ShiftService::new(shift_repo, assignment_repo, attendance_repo)
    }

    #[tokio::test]
    async fn test_create_shift() {
        let service = create_service();
        let create = CreateShift {
            tenant_id: 1,
            name: "Morning Shift".to_string(),
            shift_type: crate::domain::shift::model::ShiftType::Morning,
            start_time: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            break_duration_minutes: 60,
            expected_hours: dec!(8),
        };
        let result = service.create_shift(create).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Morning Shift");
    }

    #[tokio::test]
    async fn test_clock_in_out() {
        let service = create_service();
        let base: chrono::DateTime<chrono::Utc> = "2024-01-15T09:00:00Z".parse().unwrap();
        let clock_in = ClockInRequest {
            employee_id: 1,
            shift_id: 1,
            timestamp: base,
            notes: None,
        };
        let result = service.clock_in(clock_in).await;
        assert!(result.is_ok());

        let clock_out = ClockOutRequest {
            employee_id: 1,
            timestamp: base + chrono::Duration::hours(8),
            notes: None,
        };
        let result = service.clock_out(clock_out).await;
        assert!(result.is_ok(), "clock_out failed: {:?}", result);
        let record = result.unwrap();
        assert!(record.hours_worked >= dec!(7.9) && record.hours_worked <= dec!(8.1));
    }

    #[tokio::test]
    async fn test_assign_employee() {
        let service = create_service();
        let assignment = CreateShiftAssignment {
            shift_id: 1,
            employee_id: 1,
            start_date: Utc::now(),
            end_date: None,
        };
        let result = service.assign_employee(assignment).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().employee_id, 1);
    }
}
