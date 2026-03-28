//! User domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{CreateUser, Role, UpdateUser, User, UserResponse};
pub use repository::{BoxUserRepository, InMemoryUserRepository, RepositoryError, UserRepository};
pub use service::UserService;
