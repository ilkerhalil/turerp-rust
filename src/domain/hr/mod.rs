//! HR domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    Attendance, AttendanceStatus, CreateAttendance, CreateEmployee, CreateLeaveRequest, Employee,
    EmployeeResponse, EmployeeStatus, LeaveRequest, LeaveRequestStatus, LeaveType, Payroll,
    PayrollStatus,
};
pub use repository::{
    AttendanceRepository, BoxAttendanceRepository, BoxEmployeeRepository,
    BoxLeaveRequestRepository, BoxLeaveTypeRepository, BoxPayrollRepository, EmployeeRepository,
    InMemoryAttendanceRepository, InMemoryEmployeeRepository, InMemoryLeaveRequestRepository,
    InMemoryLeaveTypeRepository, InMemoryPayrollRepository, LeaveRequestRepository,
    LeaveTypeRepository, PayrollRepository,
};
pub use service::HrService;
