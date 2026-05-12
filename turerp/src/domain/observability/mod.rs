//! Observability domain module

pub mod model;
pub mod repository;
pub mod service;

#[cfg(feature = "postgres")]
pub mod postgres_repository;

pub use model::{
    Alert, AlertRule, AlertSeverity, AlertState, HealthCheckResult, HealthStatus, SliDefinition,
    SliMeasurement, SloCompliance, SloDefinition, SloStatus, SloTarget, SparklineDataPoint,
    SystemHealthSummary,
};
pub use repository::{
    BoxObservabilityRepository, InMemoryObservabilityRepository, ObservabilityRepository,
};
pub use service::ObservabilityService;
