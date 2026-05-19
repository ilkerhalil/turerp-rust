//! HR domain module

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;
pub mod sgk;

// Re-exports
pub use model::{
    Attendance, AttendanceStatus, CreateAttendance, CreateEmployee, CreateLeaveRequest, Employee,
    EmployeeResponse, EmployeeStatus, LeaveRequest, LeaveRequestStatus, LeaveType, Payroll,
    PayrollStatus,
};
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
pub use sgk::model::{
    CreateEmployeeBonus, CreateSgkConfig, CreateSgkEmployeeRegistration, EmployeeBonus,
    IncomeTaxBracket, MaritalStatus, SgkConfig, SgkEmployeeRegistration, SgkPayrollLineItem,
    SgkPayrollSummary, UpdateSgkConfig,
};
pub use sgk::repository::{
    BoxEmployeeBonusRepository, BoxSgkConfigRepository, BoxSgkEmployeeRegistrationRepository,
    EmployeeBonusRepository, InMemoryEmployeeBonusRepository, InMemorySgkConfigRepository,
    InMemorySgkEmployeeRegistrationRepository, SgkConfigRepository,
    SgkEmployeeRegistrationRepository,
};
