//! Quality control domain module

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    CreateInspection, CreateNonConformanceReport, Inspection, InspectionStatus, NcrStatus, NcrType,
    NonConformanceReport, UpdateInspection, UpdateNonConformanceReport,
};
pub use postgres_repository::{PostgresInspectionRepository, PostgresNcrRepository};
pub use repository::{
    BoxInspectionRepository, BoxNcrRepository, InMemoryInspectionRepository, InMemoryNcrRepository,
    InspectionRepository, NcrRepository,
};
pub use service::QualityControlService;
