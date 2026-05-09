//! Company (multi-company within tenant) module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{Company, CompanyResponse, CreateCompany, UpdateCompany};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresCompanyRepository;
pub use repository::{BoxCompanyRepository, CompanyRepository, InMemoryCompanyRepository};
pub use service::CompanyService;
