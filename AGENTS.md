# Turerp ERP - Developer Guide

## Project Overview
Multi-tenant SaaS ERP system built with Rust, Actix-web, and SQLx.

---

## Rust Best Practices

### 1. Error Handling

**Use thiserror for custom error types**

```rust
use thiserror::Error;
use actix_web::{ResponseError, HttpResponse};
use serde::Serialize;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Conflict: {0}")]
    Conflict(String),
}

// Implement Actix-web's ResponseError for automatic error responses
impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::NotFound(msg) => HttpResponse::NotFound().json(ErrorResponse { error: msg }),
            ApiError::Unauthorized(msg) => HttpResponse::Unauthorized().json(ErrorResponse { error: msg }),
            ApiError::BadRequest(msg) => HttpResponse::BadRequest().json(ErrorResponse { error: msg }),
            ApiError::Validation(msg) => HttpResponse::BadRequest().json(ErrorResponse { error: msg }),
            ApiError::InvalidCredentials => HttpResponse::Unauthorized().json(ErrorResponse { error: "Invalid credentials" }),
            ApiError::TokenExpired => HttpResponse::Unauthorized().json(ErrorResponse { error: "Token expired" }),
            ApiError::InvalidToken(msg) => HttpResponse::Unauthorized().json(ErrorResponse { error: msg }),
            ApiError::Conflict(msg) => HttpResponse::Conflict().json(ErrorResponse { error: msg }),
            ApiError::Database(msg) => HttpResponse::InternalServerError().json(ErrorResponse { error: msg }),
            ApiError::Internal(msg) => HttpResponse::InternalServerError().json(ErrorResponse { error: msg }),
        }
    }
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub type ApiResult<T> = Result<T, ApiError>;
```

### 2. Async/Await Patterns

**Always use async for I/O operations**

```rust
// Good: Use async for I/O operations
pub async fn get_user(repo: &UserRepository, id: i64) -> Result<User, ApiError> {
    repo.find_by_id(id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::NotFound(format!("User {} not found", id)))
}

// Use tokio::try_join! for parallel operations
async fn get_user_data(user_id: i64) -> Result<(User, Vec<Order>), ApiError> {
    let user_future = user_repo.find_by_id(user_id);
    let orders_future = order_repo.find_by_user(user_id);

    let (user, orders) = tokio::try_join!(user_future, orders_future)?;

    Ok((user?, orders?))
}

// NEVER block the async runtime
// Bad:
async fn bad_example() {
    let data = std::fs::read_to_string("file.txt").unwrap();
}

// Good:
async fn good_example() -> Result<String, std::io::Error> {
    let data = tokio::fs::read_to_string("file.txt").await?;
    Ok(data)
}
```

### 3. Repository Pattern

**Define repository traits for testability**

```rust
use async_trait::async_trait;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: CreateUser) -> Result<User, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<User>, ApiError>;
    async fn find_by_username(&self, username: &str, tenant_id: i64) -> Result<Option<User>, ApiError>;
    async fn find_by_email(&self, email: &str, tenant_id: i64) -> Result<Option<User>, ApiError>;
    async fn find_all(&self, tenant_id: i64) -> Result<Vec<User>, ApiError>;
    async fn update(&self, id: i64, tenant_id: i64, user: UpdateUser) -> Result<User, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
    async fn username_exists(&self, username: &str, tenant_id: i64) -> Result<bool, ApiError>;
    async fn email_exists(&self, email: &str, tenant_id: i64) -> Result<bool, ApiError>;
}

// In-memory implementation for testing/development
pub struct InMemoryUserRepository {
    users: Mutex<HashMap<i64, User>>,
    next_id: Mutex<i64>,
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn create(&self, user: CreateUser, hashed_password: String) -> Result<User, ApiError> {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let new_user = User::new(
            id,
            user.username,
            user.email,
            user.full_name,
            hashed_password,
            user.tenant_id,
        );

        self.users.lock().unwrap().insert(id, new_user.clone());
        Ok(new_user)
    }

    // ... other methods
}
```

### 4. State Management with Actix-web

```rust
use actix_web::web;
use std::sync::Arc;

// Use AppState for dependency injection
pub struct AppState {
    pub user_repo: Arc<dyn UserRepository>,
    pub auth_service: Arc<AuthService>,
}

// Extract state in handlers
pub async fn get_users(
    state: web::Data<AppState>,
    claims: AuthClaims,
) -> ApiResult<HttpResponse> {
    let users = state.user_repo.find_all(claims.tenant_id).await?;
    Ok(HttpResponse::Ok().json(users))
}
```

### 5. Configuration Management

```rust
use serde::Deserialize;
use std::convert::TryFrom;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration: i64,
    pub refresh_expiration: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8000,
            },
            database: DatabaseConfig {
                url: "postgres://postgres:postgres@localhost:5432/turerp".to_string(),
                max_connections: 5,
            },
            jwt: JwtConfig {
                secret: "dev-secret-change-in-production".to_string(),
                expiration: 3600,
                refresh_expiration: 604800,
            },
        }
    }
}

impl Config {
    pub fn new() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()
    }
}
```

### 6. Validation with Validators

```rust
use validator::Validate;

#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 50))]
    pub username: String,

    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8))]
    pub password: String,

    #[validate(length(min = 1, max = 100))]
    pub full_name: String,
}

// Validate in handler
pub async fn create_user(
    state: web::Data<AppState>,
    payload: web::Json<CreateUserRequest>,
) -> ApiResult<HttpResponse> {
    payload.validate()
        .map_err(|e| ApiError::Validation(e.to_string()))?;

    // ... proceed with creation
}
```

### 7. JWT Authentication

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, decode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthClaims {
    pub sub: String,       // User ID
    pub tenant_id: i64,
    pub username: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

pub struct JwtService {
    secret: String,
    expiration: i64,
    algorithm: Algorithm,
}

impl JwtService {
    pub fn generate_tokens(&self, user_id: i64, tenant_id: i64, username: String, role: Role) -> Result<TokenPair, ApiError> {
        let now = Utc::now().timestamp();

        let access_claims = AuthClaims {
            sub: user_id.to_string(),
            tenant_id,
            username: username.clone(),
            role: role.to_string(),
            exp: now + self.expiration,
            iat: now,
        };

        // ... encode tokens
    }
}
```

### 8. Module Structure

```
turerp/
├── src/
│   ├── main.rs                 # Application entry point
│   ├── lib.rs                  # Library root, re-exports
│   ├── config.rs               # Configuration management
│   ├── error.rs                # Error types (thiserror)
│   ├── domain/                 # Domain layer (DDD)
│   │   ├── mod.rs
│   │   ├── auth/
│   │   │   ├── mod.rs          # Auth service
│   │   │   └── ...
│   │   ├── user/
│   │   │   ├── mod.rs
│   │   │   ├── model.rs        # User, Role, etc.
│   │   │   ├── repository.rs  # Repository trait & implementations
│   │   │   └── service.rs      # Business logic
│   │   └── tenant/
│   │       └── ...
│   ├── api/                    # API layer
│   │   ├── mod.rs
│   │   ├── auth/
│   │   │   └── ...
│   │   └── users/
│   │       └── ...
│   ├── middleware/
│   │   ├── mod.rs
│   │   └── auth.rs
│   └── utils/
│       ├── mod.rs
│       ├── password.rs
│       └── jwt.rs
├── tests/
│   ├── integration/
│   └── unit/
├── Cargo.toml
└── .env.example
```

---

## TDD Workflow

### 1. Write a Failing Test FIRST

**Rule: Never implement code without a failing test**

```rust
// src/domain/cari/mod.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cari::model::CariType;

    #[tokio::test]
    async fn test_create_cari_account() {
        // Arrange
        let repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
        let service = CariService::new(repo);

        let create = CreateCari {
            code: "C001".to_string(),
            name: "Test Customer".to_string(),
            cari_type: CariType::Customer,
            tenant_id: 1,
            ..Default::default()
        };

        // Act
        let result = service.create_cari(create).await;

        // Assert
        assert!(result.is_ok());
        let cari = result.unwrap();
        assert_eq!(cari.code, "C001");
    }

    #[tokio::test]
    async fn test_cari_code_must_be_unique() {
        // Arrange
        let repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
        let service = CariService::new(repo);

        let create = CreateCari {
            code: "C001".to_string(),
            name: "Test Customer".to_string(),
            cari_type: CariType::Customer,
            tenant_id: 1,
            ..Default::default()
        };

        service.create_cari(create.clone()).await.unwrap();

        // Act
        let result = service.create_cari(create).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Conflict(_)));
    }
}
```

### 2. Make the Test Pass with Minimal Code

```rust
// src/domain/cari/service.rs

pub struct CariService {
    repo: BoxCariRepository,
}

impl CariService {
    pub fn new(repo: BoxCariRepository) -> Self {
        Self { repo }
    }

    pub async fn create_cari(&self, create: CreateCari) -> Result<CariResponse, ApiError> {
        create.validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;

        // Check if code exists
        if self.repo.code_exists(&create.code, create.tenant_id).await? {
            return Err(ApiError::Conflict(format!("Cari code '{}' already exists", create.code)));
        }

        let cari = self.repo.create(create).await?;
        Ok(cari.into())
    }
}
```

### 3. Refactor

- Improve code structure
- Add proper error handling
- Optimize performance
- Ensure tests still pass

### Running Tests

```bash
# All tests
cargo test

# Specific module
cargo test --lib domain::cari

# With output
cargo test -- --nocapture

# Watch mode (requires cargo-watch)
cargo watch -x test
```

### Test Organization

```rust
// Unit tests - same file as implementation
#[cfg(test)]
mod tests {
    use super::*;
    // ... tests
}

// Integration tests - tests/ directory
#[cfg(test)]
mod integration_tests {
    use actix_web::{test, web, App};

    #[tokio::test]
    async fn test_cari_endpoint() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state()))
                .configure(cari::configure)
        );

        let req = test::TestRequest::post()
            .uri("/api/cari")
            .set_json(&CreateCariRequest { ... })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }
}
```

---

## Code Conventions

### Naming
- Use `snake_case` for variables and functions
- Use `CamelCase` for types and enums
- Use `UPPER_SNAKE_CASE` for constants
- Prefix async functions with `_` if they don't use `await`

### Imports
```rust
// Order: std -> external -> internal
use std::sync::Arc;

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use tokio::try_join;

use crate::config::Config;
use crate::domain::cari::model::Cari;
use crate::error::ApiError;
```

### Documentation
```rust
/// Creates a new cari account in the system.
///
/// # Arguments
/// * `create` - Cari creation data
///
/// # Returns
/// * `Ok(CariResponse)` - The created cari account
/// * `Err(ApiError)` - If creation fails
///
/// # Errors
/// Returns [`ApiError::BadRequest`] if validation fails
/// Returns [`ApiError::Conflict`] if cari code already exists
/// Returns [`ApiError::Database`] if database operation fails
pub async fn create_cari(&self, create: CreateCari) -> Result<CariResponse, ApiError> {
    // Implementation
}
```

---

## Common Pitfalls to Avoid

1. **Don't block async runtime**: Use `tokio::fs` instead of `std::fs`
2. **Don't clone unnecessarily**: Use references or `Arc`
3. **Avoid `.unwrap()` in production**: Use proper error handling
4. **Don't forget to handle `None`**: Always handle `Option` types
5. **Avoid circular dependencies**: Use traits for dependency injection

---

## Security Considerations

1. **Password Handling**: Always hash passwords with bcrypt
2. **JWT Secrets**: Use strong, random secrets
3. **SQL Injection**: Use parameterized queries (handled by SQLx)
4. **Input Validation**: Validate all user input
5. **Rate Limiting**: Implement rate limiting for auth endpoints
6. **CORS**: Configure appropriate CORS policies
7. **HTTPS**: Use HTTPS in production

---

## References
- [Rust Book](https://doc.rust-lang.org/book/)
- [Actix-web Documentation](https://actix.rs/docs/)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Thiserror Documentation](https://docs.rs/thiserror/)

---

## OpenAPI / Swagger Desteği

### Kurulum

```toml
# Cargo.toml
utoipa = "4"
utoipa-swagger-ui = { version = "6", features = ["actix-web"] }
```

### Model Tanımları

Tüm request/response tiplerine `ToSchema` derive ekle:

```rust
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub user: UserResponse,
    pub tokens: TokenPair,
}
```

### Endpoint Annotation

Her endpoint için `#[utoipa::path]` kullan:

```rust
/// Register a new user
///
/// Creates a new user account in the system
#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = LoginResponse),
        (status = 400, description = "Validation error"),
        (status = 409, description = "User already exists")
    )
)]
pub async fn register(
    auth_service: web::Data<AuthService>,
    payload: web::Json<RegisterRequest>,
) -> ApiResult<HttpResponse> {
    // ...
}
```

### OpenAPI Spec Tanımlama

```rust
// src/api/mod.rs
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Turerp ERP API",
        description = "Multi-tenant SaaS ERP system API",
        version = "0.1.0",
    ),
    paths(
        crate::api::auth::register,
        crate::api::auth::login,
        // ...
    ),
    components(
        schemas(
            LoginRequest,
            LoginResponse,
            RegisterRequest,
            // ...
        )
    ),
    tags(
        (name = "Auth", description = "Authentication endpoints"),
        (name = "Users", description = "User management endpoints")
    )
)]
pub struct ApiDoc;
```

### Swagger UI Endpoint

```rust
// src/main.rs
use utoipa_swagger_ui::SwaggerUi;

HttpServer::new(move || {
    App::new()
        .service(
            SwaggerUi::new("/swagger-ui/{_:.*}")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
})
```

### Kullanım

Sunucu çalıştıktan sonra:
- **Swagger UI**: `http://localhost:8080/swagger-ui/`
- **OpenAPI JSON**: `http://localhost:8080/api-docs/openapi.json`
- **OpenAPI YAML**: `http://localhost:8080/api-docs/openapi.yaml`

---

## Code Review Süreci

### Review Checklist

- [ ] Build başarılı mı? (`cargo check`)
- [ ] Clippy uyarıları var mı? (`cargo clippy -- -D warnings`)
- [ ] Format doğru mu? (`cargo fmt --check`)
- [ ] Testler geçiyor mu? (`cargo test`)
- [ ] Ambiguous glob re-export yok mu?
- [ ] Unused imports temizlendi mi?
- [ ] Dead code kaldırıldı mı?
- [ ] Fonksiyon argüman sayısı max 7 mi?

### Yaygın Hatalar

1. **Ambiguous Glob Re-exports**: `pub use module::*;` yerine explicit export kullan
2. **Too Many Arguments**: 7'den fazla argüman için struct kullan
3. **Mutex Unwrap**: `lock().unwrap()` yerine proper error handling kullan