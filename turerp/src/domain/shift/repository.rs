//! Shift Planning repository traits and in-memory implementations

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::common::SoftDeletable;
use crate::domain::shift::model::{
    AttendanceRecord, AttendanceStatus, ClockInRequest, ClockOutRequest, CreateShift,
    CreateShiftAssignment, Shift, ShiftAssignment,
};
use crate::error::ApiError;

/// Repository trait for Shift operations
#[async_trait]
pub trait ShiftRepository: Send + Sync {
    async fn create(&self, shift: CreateShift) -> Result<Shift, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Shift>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Shift>, ApiError>;
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Shift>, ApiError>;
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        shift: crate::domain::shift::model::UpdateShift,
    ) -> Result<Shift, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Shift, ApiError>;
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Shift>, ApiError>;
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for ShiftAssignment operations
#[async_trait]
pub trait ShiftAssignmentRepository: Send + Sync {
    async fn create(&self, assignment: CreateShiftAssignment) -> Result<ShiftAssignment, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<ShiftAssignment>, ApiError>;
    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<ShiftAssignment>, ApiError>;
    async fn find_by_shift(&self, shift_id: i64) -> Result<Vec<ShiftAssignment>, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn restore(&self, id: i64) -> Result<ShiftAssignment, ApiError>;
    async fn find_deleted(&self) -> Result<Vec<ShiftAssignment>, ApiError>;
    async fn destroy(&self, id: i64) -> Result<(), ApiError>;
}

/// Repository trait for AttendanceRecord operations
#[async_trait]
pub trait AttendanceRecordRepository: Send + Sync {
    async fn clock_in(&self, req: ClockInRequest) -> Result<AttendanceRecord, ApiError>;
    async fn clock_out(&self, req: ClockOutRequest) -> Result<AttendanceRecord, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<AttendanceRecord>, ApiError>;
    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<AttendanceRecord>, ApiError>;
    async fn find_by_employee_and_date(
        &self,
        employee_id: i64,
        date: DateTime<Utc>,
    ) -> Result<Option<AttendanceRecord>, ApiError>;
    async fn find_by_period(
        &self,
        employee_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AttendanceRecord>, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<(), ApiError>;
    async fn restore(&self, id: i64) -> Result<AttendanceRecord, ApiError>;
    async fn find_deleted(&self) -> Result<Vec<AttendanceRecord>, ApiError>;
    async fn destroy(&self, id: i64) -> Result<(), ApiError>;
}

/// Type aliases
pub type BoxShiftRepository = Arc<dyn ShiftRepository>;
pub type BoxShiftAssignmentRepository = Arc<dyn ShiftAssignmentRepository>;
pub type BoxAttendanceRecordRepository = Arc<dyn AttendanceRecordRepository>;

// ---------------------------------------------------------------------------
// In-memory Shift repository
// ---------------------------------------------------------------------------

struct InMemoryShiftInner {
    shifts: std::collections::HashMap<i64, Shift>,
    next_id: i64,
}

pub struct InMemoryShiftRepository {
    inner: Mutex<InMemoryShiftInner>,
}

impl InMemoryShiftRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryShiftInner {
                shifts: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryShiftRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ShiftRepository for InMemoryShiftRepository {
    async fn create(&self, create: CreateShift) -> Result<Shift, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let now = Utc::now();

        let shift = Shift {
            id,
            tenant_id: create.tenant_id,
            name: create.name,
            shift_type: create.shift_type,
            start_time: create.start_time,
            end_time: create.end_time,
            break_duration_minutes: create.break_duration_minutes,
            expected_hours: create.expected_hours,
            is_active: true,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        inner.shifts.insert(id, shift.clone());
        Ok(shift)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Shift>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .shifts
            .get(&id)
            .filter(|s| s.tenant_id == tenant_id && !s.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Shift>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .shifts
            .values()
            .filter(|s| s.tenant_id == tenant_id && !s.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Shift>, ApiError> {
        let inner = self.inner.lock();
        let total = inner
            .shifts
            .values()
            .filter(|s| s.tenant_id == tenant_id && !s.is_deleted())
            .count() as u64;

        let items: Vec<Shift> = inner
            .shifts
            .values()
            .filter(|s| s.tenant_id == tenant_id && !s.is_deleted())
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .cloned()
            .collect();

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: crate::domain::shift::model::UpdateShift,
    ) -> Result<Shift, ApiError> {
        let mut inner = self.inner.lock();
        let shift = inner
            .shifts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Shift {} not found", id)))?;
        if shift.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Shift {} not found", id)));
        }

        if let Some(name) = update.name {
            shift.name = name;
        }
        if let Some(shift_type) = update.shift_type {
            shift.shift_type = shift_type;
        }
        if let Some(start_time) = update.start_time {
            shift.start_time = start_time;
        }
        if let Some(end_time) = update.end_time {
            shift.end_time = end_time;
        }
        if let Some(break_duration) = update.break_duration_minutes {
            shift.break_duration_minutes = break_duration;
        }
        if let Some(expected_hours) = update.expected_hours {
            shift.expected_hours = expected_hours;
        }
        if let Some(is_active) = update.is_active {
            shift.is_active = is_active;
        }
        shift.updated_at = Utc::now();
        Ok(shift.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let shift = inner
            .shifts
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Shift {} not found", id)))?;
        if shift.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Shift {} not found", id)));
        }
        inner.shifts.remove(&id);
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let shift = inner
            .shifts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Shift {} not found", id)))?;
        if shift.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Shift {} not found", id)));
        }
        shift.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Shift, ApiError> {
        let mut inner = self.inner.lock();
        let shift = inner
            .shifts
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Shift {} not found", id)))?;
        if shift.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Shift {} not found", id)));
        }
        shift.restore();
        Ok(shift.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Shift>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .shifts
            .values()
            .filter(|s| s.tenant_id == tenant_id && s.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let shift = inner
            .shifts
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Shift {} not found", id)))?;
        if shift.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Shift {} not found", id)));
        }
        inner.shifts.remove(&id);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// In-memory ShiftAssignment repository
// ---------------------------------------------------------------------------

struct InMemoryShiftAssignmentInner {
    assignments: std::collections::HashMap<i64, ShiftAssignment>,
    next_id: i64,
}

pub struct InMemoryShiftAssignmentRepository {
    inner: Mutex<InMemoryShiftAssignmentInner>,
}

impl InMemoryShiftAssignmentRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryShiftAssignmentInner {
                assignments: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryShiftAssignmentRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ShiftAssignmentRepository for InMemoryShiftAssignmentRepository {
    async fn create(&self, create: CreateShiftAssignment) -> Result<ShiftAssignment, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let assignment = ShiftAssignment {
            id,
            shift_id: create.shift_id,
            employee_id: create.employee_id,
            start_date: create.start_date,
            end_date: create.end_date,
            created_at: Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };

        inner.assignments.insert(id, assignment.clone());
        Ok(assignment)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<ShiftAssignment>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .assignments
            .get(&id)
            .filter(|a| !a.is_deleted())
            .cloned())
    }

    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<ShiftAssignment>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .assignments
            .values()
            .filter(|a| a.employee_id == employee_id && !a.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_shift(&self, shift_id: i64) -> Result<Vec<ShiftAssignment>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .assignments
            .values()
            .filter(|a| a.shift_id == shift_id && !a.is_deleted())
            .cloned()
            .collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.assignments.remove(&id);
        Ok(())
    }

    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let assignment = inner
            .assignments
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Shift assignment {} not found", id)))?;
        assignment.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64) -> Result<ShiftAssignment, ApiError> {
        let mut inner = self.inner.lock();
        let assignment = inner
            .assignments
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Shift assignment {} not found", id)))?;
        assignment.restore();
        Ok(assignment.clone())
    }

    async fn find_deleted(&self) -> Result<Vec<ShiftAssignment>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .assignments
            .values()
            .filter(|a| a.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner
            .assignments
            .remove(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Shift assignment {} not found", id)))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// In-memory AttendanceRecord repository
// ---------------------------------------------------------------------------

struct InMemoryAttendanceRecordInner {
    records: std::collections::HashMap<i64, AttendanceRecord>,
    next_id: i64,
}

pub struct InMemoryAttendanceRecordRepository {
    inner: Mutex<InMemoryAttendanceRecordInner>,
}

impl InMemoryAttendanceRecordRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryAttendanceRecordInner {
                records: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryAttendanceRecordRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AttendanceRecordRepository for InMemoryAttendanceRecordRepository {
    async fn clock_in(&self, req: ClockInRequest) -> Result<AttendanceRecord, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let record = AttendanceRecord {
            id,
            employee_id: req.employee_id,
            shift_id: req.shift_id,
            date: req
                .timestamp
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .expect("00:00:00 is always valid")
                .and_local_timezone(Utc)
                .single()
                .expect("UTC has no ambiguous times"),
            clock_in: Some(req.timestamp),
            clock_out: None,
            hours_worked: Decimal::ZERO,
            overtime_hours: Decimal::ZERO,
            status: AttendanceStatus::Present,
            notes: req.notes,
            deleted_at: None,
            deleted_by: None,
        };

        inner.records.insert(id, record.clone());
        Ok(record)
    }

    async fn clock_out(&self, req: ClockOutRequest) -> Result<AttendanceRecord, ApiError> {
        let mut inner = self.inner.lock();
        let mut found = None;
        for (id, record) in inner.records.iter_mut() {
            if record.employee_id == req.employee_id
                && record.clock_out.is_none()
                && !record.is_deleted()
            {
                record.clock_out = Some(req.timestamp);
                let clock_in_time = record.clock_in.unwrap_or(req.timestamp);
                let duration = req.timestamp.signed_duration_since(clock_in_time);
                let hours = Decimal::from(duration.num_seconds()) / Decimal::from(3600);
                record.hours_worked = hours.max(Decimal::ZERO);

                if let Some(ref mut notes) = record.notes {
                    if let Some(n) = req.notes {
                        notes.push_str(&format!("; {}", n));
                    }
                } else {
                    record.notes = req.notes;
                }
                found = Some(*id);
                break;
            }
        }

        let id = found.ok_or_else(|| {
            ApiError::NotFound("No open attendance record found for employee".to_string())
        })?;
        Ok(inner
            .records
            .get(&id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound("Attendance record disappeared".to_string()))?)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<AttendanceRecord>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.records.get(&id).filter(|r| !r.is_deleted()).cloned())
    }

    async fn find_by_employee(&self, employee_id: i64) -> Result<Vec<AttendanceRecord>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .filter(|r| r.employee_id == employee_id && !r.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_employee_and_date(
        &self,
        employee_id: i64,
        date: DateTime<Utc>,
    ) -> Result<Option<AttendanceRecord>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .find(|r| {
                r.employee_id == employee_id
                    && r.date.date_naive() == date.date_naive()
                    && !r.is_deleted()
            })
            .cloned())
    }

    async fn find_by_period(
        &self,
        employee_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AttendanceRecord>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .filter(|r| {
                r.employee_id == employee_id && r.date >= start && r.date <= end && !r.is_deleted()
            })
            .cloned()
            .collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner.records.remove(&id);
        Ok(())
    }

    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let record = inner
            .records
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Attendance record {} not found", id)))?;
        record.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64) -> Result<AttendanceRecord, ApiError> {
        let mut inner = self.inner.lock();
        let record = inner
            .records
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Attendance record {} not found", id)))?;
        record.restore();
        Ok(record.clone())
    }

    async fn find_deleted(&self) -> Result<Vec<AttendanceRecord>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .filter(|r| r.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner
            .records
            .remove(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Attendance record {} not found", id)))?;
        Ok(())
    }
}
