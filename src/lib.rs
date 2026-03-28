//! Turerp ERP - Multi-tenant SaaS ERP system
//!
//! This is the core library for the Turerp ERP system built with Rust,
//! Actix-web, and Sea-orm.

pub mod api;
pub mod config;
pub mod domain;
pub mod error;
pub mod middleware;
pub mod utils;

// Re-export commonly used types
pub use config::Config;
pub use domain::{
    auth::{AuthService, LoginRequest, RefreshTokenRequest, RegisterRequest},
    cari::{CariResponse, CariService, CariStatus, CariType, CreateCari, UpdateCari},
    tenant::{CreateTenant, Tenant, UpdateTenant},
    user::{CreateUser, Role, UpdateUser, User, UserResponse, UserService},
};
pub use error::{ApiError, ApiResult, ErrorResponse};

/// Application state
pub mod app {
    use actix_web::web;

    use crate::domain::auth::AuthService;
    use crate::domain::user::repository::{BoxUserRepository, InMemoryUserRepository};
    use crate::domain::user::service::UserService;
    use crate::utils::jwt::JwtService;
    use std::sync::Arc;

    /// Application state data
    #[derive(Clone)]
    pub struct AppState {
        pub auth_service: web::Data<AuthService>,
        pub user_service: web::Data<UserService>,
    }

    /// Create application with in-memory storage (for development/testing)
    pub fn create_app_state() -> AppState {
        // Create in-memory repository
        let repo = Arc::new(InMemoryUserRepository::new()) as BoxUserRepository;

        // Create user service
        let user_service = UserService::new(repo);

        // Create JWT service
        let jwt_service = JwtService::new(
            "dev-secret-key-change-in-production-12345".to_string(),
            3600,
            604800,
        );

        // Create auth service
        let auth_service = AuthService::new(user_service.clone(), jwt_service);

        // Wrap in actix Data
        let user_service = web::Data::new(user_service);
        let auth_service = web::Data::new(auth_service);

        AppState {
            auth_service,
            user_service,
        }
    }
}

/// Setup logging for the application
pub fn setup_logging() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "turerp=debug,actix_web=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lib_exists() {
        assert_eq!(42, 42);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.server.port, 8000);
    }
}
