//! Background job scheduler domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    CreateJob, CreateJobSchedule, Job, JobCounts, JobPriority, JobSchedule, JobStatus, JobType,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresJobRepository;
pub use repository::{BoxJobRepository, InMemoryJobRepository, JobRepository};
pub use service::JobService;
