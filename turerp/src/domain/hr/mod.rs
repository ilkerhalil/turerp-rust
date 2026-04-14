//! HR domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    Attendance, AttendanceStatus, CreateAttendance, CreateEmployee, CreateLeaveRequest, Employee,
    EmployeeResponse, EmployeeStatus, LeaveRequest, LeaveRequestStatus, LeaveType, Payroll,
    PayrollStatus,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::{
    PostgresAttendanceRepository, PostgresEmployeeRepository, PostgresLeaveRequestRepository,
    PostgresLeaveTypeRepository, PostgresPayrollRepository,
};
pub use repository::{
    AttendanceRepository, BoxAttendanceRepository, BoxEmployeeRepository,
    BoxLeaveRequestRepository, BoxLeaveTypeRepository, BoxPayrollRepository, EmployeeRepository,
    InMemoryAttendanceRepository, InMemoryEmployeeRepository, InMemoryLeaveRequestRepository,
    InMemoryLeaveTypeRepository, InMemoryPayrollRepository, LeaveRequestRepository,
    LeaveTypeRepository, PayrollRepository,
};
pub use service::HrService;
