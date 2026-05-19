//! Shift Planning domain module

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    AttendanceRecord, AttendanceRecordResponse, AttendanceStatus, ClockInRequest, ClockOutRequest,
    CreateShift, CreateShiftAssignment, OvertimeCalculation, Shift, ShiftAssignment, ShiftReport,
    ShiftReportQuery, ShiftResponse, ShiftType, UpdateShift,
};
pub use postgres_repository::{
    PostgresAttendanceRecordRepository, PostgresShiftAssignmentRepository, PostgresShiftRepository,
};
pub use repository::{
    AttendanceRecordRepository, BoxAttendanceRecordRepository, BoxShiftAssignmentRepository,
    BoxShiftRepository, InMemoryAttendanceRecordRepository, InMemoryShiftAssignmentRepository,
    InMemoryShiftRepository, ShiftAssignmentRepository, ShiftRepository,
};
pub use service::ShiftService;
