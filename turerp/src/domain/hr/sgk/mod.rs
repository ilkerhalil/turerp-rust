//! SGK (Social Security Institution) payroll integration module

pub mod calculator;
pub mod ebildirge;
pub mod model;
pub mod repository;
pub mod service;

pub use ebildirge::{EBildirgeGenerator, EmployerInfo};
pub use model::*;
pub use repository::*;
pub use service::SgkPayrollService;
