//! API Key domain module

pub mod model;
pub mod repository;
pub mod service;

pub use model::{
    extract_prefix, generate_api_key, hash_api_key, ApiKey, ApiKeyCreationResult, ApiKeyResponse,
    ApiKeyScope, CreateApiKey, UpdateApiKey,
};
#[cfg(feature = "postgres")]
pub mod postgres_repository;

#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresApiKeyRepository;
pub use repository::{ApiKeyRepository, BoxApiKeyRepository, InMemoryApiKeyRepository};
pub use service::ApiKeyService;
