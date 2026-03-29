//! Turerp ERP - Multi-tenant SaaS ERP system
//!
//! This is the core library for the Turerp ERP system built with Rust,
//! Actix-web, and SQLx.

pub mod api;
pub mod config;
#[cfg(feature = "postgres")]
pub mod db;
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
    use std::sync::Arc;

    use crate::config::Config;
    use crate::domain::auth::AuthService;
    use crate::domain::user::repository::BoxUserRepository;
    use crate::domain::user::service::UserService;
    use crate::utils::jwt::JwtService;

    #[cfg(not(feature = "postgres"))]
    use crate::domain::user::repository::InMemoryUserRepository;

    #[cfg(feature = "postgres")]
    use crate::db;
    #[cfg(feature = "postgres")]
    use crate::domain::user::postgres_repository::PostgresUserRepository;
    #[cfg(feature = "postgres")]
    use sqlx::PgPool;

    /// Application state data
    #[derive(Clone)]
    pub struct AppState {
        pub auth_service: web::Data<AuthService>,
        pub user_service: web::Data<UserService>,
        pub jwt_service: web::Data<JwtService>,
        #[cfg(feature = "postgres")]
        pub db_pool: web::Data<Arc<PgPool>>,
    }

    /// Create application state with in-memory storage (for development/testing)
    #[cfg(not(feature = "postgres"))]
    pub fn create_app_state(config: &Config) -> AppState {
        let repo = Arc::new(InMemoryUserRepository::new()) as BoxUserRepository;

        // Create user service
        let user_service = UserService::new(repo);

        // Create JWT service from config
        let jwt_service = JwtService::new(
            config.jwt.secret.clone(),
            config.jwt.access_token_expiration,
            config.jwt.refresh_token_expiration,
        );

        // Create auth service
        let auth_service = AuthService::new(user_service.clone(), jwt_service.clone());

        // Wrap in actix Data
        let user_service = web::Data::new(user_service);
        let jwt_service = web::Data::new(jwt_service);
        let auth_service = web::Data::new(auth_service);

        AppState {
            auth_service,
            user_service,
            jwt_service,
        }
    }

    /// Create application state with PostgreSQL storage (for production)
    #[cfg(feature = "postgres")]
    pub async fn create_app_state(config: &Config) -> AppState {
        // Create connection pool
        let pool = Arc::new(
            db::create_pool(&config.database)
                .await
                .expect("Failed to create database pool"),
        );

        // Run migrations
        db::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        // Create PostgreSQL repository
        let repo = Arc::new(PostgresUserRepository::new(pool.clone())) as BoxUserRepository;

        // Create user service
        let user_service = UserService::new(repo);

        // Create JWT service from config
        let jwt_service = JwtService::new(
            config.jwt.secret.clone(),
            config.jwt.access_token_expiration,
            config.jwt.refresh_token_expiration,
        );

        // Create auth service
        let auth_service = AuthService::new(user_service.clone(), jwt_service.clone());

        // Wrap in actix Data
        let user_service = web::Data::new(user_service);
        let jwt_service = web::Data::new(jwt_service);
        let auth_service = web::Data::new(auth_service);
        let db_pool = web::Data::new(pool);

        AppState {
            auth_service,
            user_service,
            jwt_service,
            db_pool,
        }
    }

    /// Create application state with in-memory storage
    #[cfg(not(feature = "postgres"))]
    pub fn create_app_state_in_memory(config: &Config) -> AppState {
        create_app_state(config)
    }

    /// Create application state with in-memory storage (postgres mode - for testing)
    #[cfg(feature = "postgres")]
    pub fn create_app_state_in_memory(config: &Config) -> AppState {
        use crate::domain::user::repository::InMemoryUserRepository;

        let repo = Arc::new(InMemoryUserRepository::new()) as BoxUserRepository;

        // Create user service
        let user_service = UserService::new(repo);

        // Create JWT service from config
        let jwt_service = JwtService::new(
            config.jwt.secret.clone(),
            config.jwt.access_token_expiration,
            config.jwt.refresh_token_expiration,
        );

        // Create auth service
        let auth_service = AuthService::new(user_service.clone(), jwt_service.clone());

        // Wrap in actix Data
        let user_service = web::Data::new(user_service);
        let jwt_service = web::Data::new(jwt_service);
        let auth_service = web::Data::new(auth_service);

        // For in-memory testing with postgres feature, create a mock pool
        // Note: This should only be used for testing - health checks will fail
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let pool = rt.block_on(async {
            // Use connect_lazy to avoid immediate connection
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy("postgres://localhost/dummy")
                .expect("Failed to create lazy pool")
        });
        let db_pool = web::Data::new(Arc::new(pool));

        AppState {
            auth_service,
            user_service,
            jwt_service,
            db_pool,
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
        assert!(config.is_development());
    }

    #[test]
    fn test_app_state_creation() {
        let config = Config::default();
        let state = app::create_app_state_in_memory(&config);
        // Verify services are created
        assert!(std::sync::Arc::strong_count(&state.auth_service) > 0);
        assert!(std::sync::Arc::strong_count(&state.user_service) > 0);
        assert!(std::sync::Arc::strong_count(&state.jwt_service) > 0);
    }
}
