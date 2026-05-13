//! LDAP / Active Directory synchronization domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    CreateLdapConfig, LdapConfig, LdapConfigResponse, LdapSyncResult, LdapUser,
    TestLdapConnectionRequest, UpdateLdapConfig,
};
pub use repository::{BoxLdapConfigRepository, InMemoryLdapConfigRepository, LdapConfigRepository};
pub use service::{Ldap3Client, LdapClient, LdapSyncService};
