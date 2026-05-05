//! MFA (Multi-Factor Authentication) domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::{
    BackupCodesResponse, DisableMfaRequest, EnableMfaRequest, MfaChallenge, MfaMethod,
    MfaRequiredResponse, MfaSettings, MfaSetupResponse, MfaStatusResponse, VerifyMfaRequest,
    VerifyTotpRequest,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresMfaRepository;
pub use repository::{BoxMfaRepository, InMemoryMfaRepository, MfaRepository};
pub use service::MfaService;
