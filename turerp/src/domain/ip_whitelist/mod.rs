//! IP whitelist domain module
//!
//! Provides tenant-scoped IP access control with support for CIDR notation.

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    CreateIpWhitelistEntry, IpWhitelistCheckResult, IpWhitelistEntry, IpWhitelistEntryResponse,
    UpdateIpWhitelistEntry,
};
pub use postgres_repository::PostgresIpWhitelistRepository;
pub use repository::{
    BoxIpWhitelistRepository, InMemoryIpWhitelistRepository, IpWhitelistRepository,
};
pub use service::IpWhitelistService;
