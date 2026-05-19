//! User domain module

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{CreateUser, Role, UpdateUser, User, UserResponse};
pub use postgres_repository::PostgresUserRepository;
pub use repository::{BoxUserRepository, InMemoryUserRepository, RepositoryError, UserRepository};
pub use service::{UserPermissions, UserService};
