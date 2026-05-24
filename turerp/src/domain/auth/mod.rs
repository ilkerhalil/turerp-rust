//! Auth domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{LoginRequest, LoginResponse, LogoutRequest, RefreshTokenRequest, RegisterRequest};
pub use repository::{BoxRevokedTokenStore, InMemoryRevokedTokenStore, RevokedTokenStore};
pub use service::{create_auth_service, AuthService};
