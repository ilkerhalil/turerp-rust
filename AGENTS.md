# Turerp ERP - Developer Guide

## Project Overview
Multi-tenant SaaS ERP system built with Rust, Actix-web, and SQLx.

**Current Production Score: 8.5/10**

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         API Layer                                │
│  (Actix-web handlers, OpenAPI/Swagger, routing)                 │
├─────────────────────────────────────────────────────────────────┤
│                       Middleware Layer                           │
│  (JWT Auth, Rate Limiting, Request ID, CORS)                     │
├─────────────────────────────────────────────────────────────────┤
│                       Service Layer                              │
│  (Business logic, validation, orchestration)                     │
├─────────────────────────────────────────────────────────────────┤
│                      Repository Layer                            │
│  (Data access, trait-based, InMemory & PostgreSQL)               │
├─────────────────────────────────────────────────────────────────┤
│                       Domain Models                              │
│  (Entities, DTOs, value objects)                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Domain Modules

| Domain | Description | Status |
|--------|-------------|--------|
| `auth` | Authentication & JWT tokens | ✅ Complete |
| `user` | User management | ✅ Complete + PostgreSQL |
| `tenant` | Multi-tenancy | ✅ Complete + PostgreSQL |
| `cari` | Customer/Vendor accounts | ✅ Complete + PostgreSQL |
| `product` | Product catalog | ✅ Complete |
| `stock` | Inventory management | ✅ Complete |
| `invoice` | Invoicing | ✅ Complete |
| `accounting` | Chart of accounts | ✅ Complete |
| `sales` | Sales orders | ✅ Complete |
| `purchase` | Purchase orders | ✅ Complete |
| `manufacturing` | BOM & work orders | ✅ Complete |
| `crm` | CRM (leads, tickets) | ✅ Complete |
| `hr` | HR & payroll | ✅ Complete |
| `project` | Project management | ✅ Complete |

---

## Quick Start

```bash
# Development (in-memory storage)
cargo run

# Production (PostgreSQL)
cargo run --features postgres

# Run tests
cargo test

# Run with PostgreSQL tests
cargo test --features postgres

# Code quality
cargo clippy -- -D warnings
cargo fmt --check
```

### Environment Variables

```bash
# Server
TURERP_SERVER_HOST=0.0.0.0
TURERP_SERVER_PORT=8080

# Database (PostgreSQL feature)
TURERP_DATABASE_URL=postgres://user:pass@localhost:5432/turerp
TURERP_DATABASE_MAX_CONNECTIONS=10

# JWT
TURERP_JWT_SECRET=your-secret-key
TURERP_JWT_ACCESS_TOKEN_EXPIRATION=3600
TURERP_JWT_REFRESH_TOKEN_EXPIRATION=604800

# CORS
TURERP_CORS_ALLOWED_ORIGINS=["http://localhost:3000"]
TURERP_CORS_ALLOWED_METHODS=["GET","POST","PUT","DELETE"]
TURERP_CORS_ALLOWED_HEADERS=["Authorization","Content-Type"]
```

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
    Database(String),

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

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::NotFound(msg) => HttpResponse::NotFound().json(ErrorResponse { error: msg.clone() }),
            ApiError::Unauthorized(msg) => HttpResponse::Unauthorized().json(ErrorResponse { error: msg.clone() }),
            ApiError::BadRequest(msg) => HttpResponse::BadRequest().json(ErrorResponse { error: msg.clone() }),
            ApiError::Validation(msg) => HttpResponse::BadRequest().json(ErrorResponse { error: msg.clone() }),
            ApiError::InvalidCredentials => HttpResponse::Unauthorized().json(ErrorResponse { error: "Invalid credentials".to_string() }),
            ApiError::TokenExpired => HttpResponse::Unauthorized().json(ErrorResponse { error: "Token expired".to_string() }),
            ApiError::InvalidToken(msg) => HttpResponse::Unauthorized().json(ErrorResponse { error: msg.clone() }),
            ApiError::Conflict(msg) => HttpResponse::Conflict().json(ErrorResponse { error: msg.clone() }),
            ApiError::Database(msg) => {
                tracing::error!("Database error: {}", msg);
                HttpResponse::InternalServerError().json(ErrorResponse { error: "Internal database error".to_string() })
            }
            ApiError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                HttpResponse::InternalServerError().json(ErrorResponse { error: "Internal error".to_string() })
            }
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
```

### 2. Repository Pattern

**Define repository traits for testability and multiple implementations**

```rust
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: CreateUser, hashed_password: String) -> Result<User, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<User>, ApiError>;
    async fn find_by_username(&self, username: &str, tenant_id: i64) -> Result<Option<User>, ApiError>;
    async fn find_all(&self, tenant_id: i64) -> Result<Vec<User>, ApiError>;
    async fn update(&self, id: i64, tenant_id: i64, user: UpdateUser) -> Result<User, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

pub type BoxUserRepository = Arc<dyn UserRepository>;

// In-memory implementation for testing
pub struct InMemoryUserRepository {
    users: parking_lot::Mutex<HashMap<i64, User>>,
    next_id: parking_lot::Mutex<i64>,
}

// PostgreSQL implementation for production
#[cfg(feature = "postgres")]
pub struct PostgresUserRepository {
    pool: Arc<PgPool>,
}
```

### 3. PostgreSQL Repository Implementation

**Use runtime queries with FromRow for type safety**

```rust
use sqlx::FromRow;

/// Database row representation (separate from domain model)
#[derive(Debug, FromRow)]
struct UserRow {
    id: i64,
    username: String,
    email: String,
    role: String,  // Stored as string in DB
    // ...
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            role: row.role.parse().unwrap_or_else(|e| {
                tracing::warn!("Invalid role in database: {}", e);
                Role::default()
            }),
            // ...
        }
    }
}

/// Helper for consistent error mapping
fn map_sqlx_error(e: sqlx::Error, entity: &str) -> ApiError {
    match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("{} not found", entity)),
        _ => {
            let msg = e.to_string();
            if msg.contains("duplicate key") || msg.contains("unique constraint") {
                ApiError::Conflict(format!("{} already exists", entity))
            } else {
                ApiError::Database(format!("Failed to operate on {}: {}", entity, e))
            }
        }
    }
}
```

### 4. parking_lot::Mutex instead of std::sync::Mutex

**Why: `std::sync::Mutex::lock().unwrap()` can panic!**

```rust
// ❌ Bad: Can panic on poisoned mutex
use std::sync::Mutex;
let guard = self.users.lock().unwrap();

// ✅ Good: parking_lot::Mutex never poisons
use parking_lot::Mutex;
let guard = self.users.lock();  // Returns MutexGuard directly, no Result
```

### 5. Lazy Static for Regex

**Why: Compile regex patterns once, not every call**

```rust
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref UPPERCASE_REGEX: Regex = Regex::new(r"[A-Z]").unwrap();
    static ref LOWERCASE_REGEX: Regex = Regex::new(r"[a-z]").unwrap();
    static ref DIGIT_REGEX: Regex = Regex::new(r"[0-9]").unwrap();
    static ref SPECIAL_REGEX: Regex = Regex::new(r"[^A-Za-z0-9]").unwrap();
}

pub fn validate_password(password: &str) -> Result<(), PasswordValidationError> {
    if password.len() < 12 {
        return Err(PasswordValidationError::TooShort);
    }
    if !UPPERCASE_REGEX.is_match(password) {
        return Err(PasswordValidationError::MissingUppercase);
    }
    // ...
}
```

---

## Module Structure

```
turerp/
├── src/
│   ├── main.rs                 # Application entry point
│   ├── lib.rs                  # Library root, re-exports, AppState
│   ├── config.rs               # Configuration management
│   ├── error.rs                # Error types (thiserror)
│   ├── db/                     # Database layer (PostgreSQL)
│   │   ├── mod.rs
│   │   └── pool.rs             # Connection pool, migrations
│   ├── domain/                 # Domain layer (DDD)
│   │   ├── mod.rs
│   │   ├── auth/               # Authentication
│   │   │   ├── mod.rs
│   │   │   └── service.rs
│   │   ├── user/               # User management
│   │   │   ├── mod.rs
│   │   │   ├── model.rs
│   │   │   ├── repository.rs   # Trait + InMemory impl
│   │   │   ├── postgres_repository.rs  # PostgreSQL impl
│   │   │   └── service.rs
│   │   ├── tenant/             # Multi-tenancy
│   │   │   ├── mod.rs
│   │   │   ├── model.rs
│   │   │   ├── repository.rs
│   │   │   ├── postgres_repository.rs
│   │   │   └── service.rs
│   │   ├── cari/              # Customer/Vendor
│   │   │   ├── mod.rs
│   │   │   ├── model.rs
│   │   │   ├── repository.rs
│   │   │   ├── postgres_repository.rs
│   │   │   └── service.rs
│   │   └── [other domains]/
│   ├── api/                    # API layer
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   └── users.rs
│   ├── middleware/
│   │   ├── mod.rs
│   │   ├── auth.rs            # JWT authentication
│   │   ├── rate_limit.rs      # Rate limiting (governor)
│   │   └── request_id.rs      # Request ID tracing
│   └── utils/
│       ├── mod.rs
│       ├── password.rs        # Password hashing/validation
│       └── jwt.rs             # JWT utilities
├── migrations/
│   └── 001_initial_schema.sql  # Database schema
├── tests/
│   └── api_integration_test.rs
├── Cargo.toml
└── .env.example
```

---

## Feature Flags

```toml
# Cargo.toml
[features]
default = []
postgres = ["sqlx/postgres"]  # PostgreSQL storage
```

### Usage

```rust
// Conditional compilation for PostgreSQL
#[cfg(feature = "postgres")]
pub mod postgres_repository;

#[cfg(feature = "postgres")]
use crate::db;

// In lib.rs - AppState with optional db_pool
pub struct AppState {
    pub auth_service: web::Data<AuthService>,
    pub user_service: web::Data<UserService>,
    pub jwt_service: web::Data<JwtService>,
    #[cfg(feature = "postgres")]
    pub db_pool: web::Data<Arc<PgPool>>,
}
```

---

## Middleware Stack

**Order matters! Security middlewares first.**

```rust
HttpServer::new(move || {
    App::new()
        .wrap(RequestIdMiddleware)      // 1. Request ID for tracing
        .wrap(RateLimitMiddleware::new()) // 2. Rate limiting (before auth)
        .wrap(middleware::Logger::default()) // 3. Logging
        .wrap(configure_cors(&config.cors)) // 4. CORS
        // Note: JwtAuthMiddleware applied per-route via service config
        .app_data(app_state.auth_service.clone())
        .app_data(app_state.user_service.clone())
        .app_data(app_state.jwt_service.clone())
        #[cfg(feature = "postgres")]
        .app_data(app_state.db_pool.clone())
        .route("/health", web::get().to(health_check))
        // ... routes
})
.shutdown_timeout(30)  // Graceful shutdown
.run()
.await
```

---

## Health Check

**In-memory mode:**
```json
{
  "status": "ok",
  "service": "turerp-erp",
  "storage": "in-memory"
}
```

**PostgreSQL mode:**
```json
{
  "status": "ok",
  "service": "turerp-erp",
  "storage": "postgresql",
  "database": "healthy"
}
```

---

## Database Migrations

```sql
-- migrations/001_initial_schema.sql

-- Tenants table
CREATE TABLE IF NOT EXISTS tenants (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    subdomain VARCHAR(255) UNIQUE NOT NULL,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL,
    email VARCHAR(255) NOT NULL,
    full_name VARCHAR(100),
    password VARCHAR(255) NOT NULL,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    role VARCHAR(20) NOT NULL DEFAULT 'user',
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- Cari (Customer/Vendor) table
CREATE TABLE IF NOT EXISTS cari (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL,
    name VARCHAR(200) NOT NULL,
    cari_type VARCHAR(20) NOT NULL DEFAULT 'customer',
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    -- ... other fields
);

-- Indexes for tenant isolation
CREATE UNIQUE INDEX idx_users_username_tenant ON users(username, tenant_id);
CREATE UNIQUE INDEX idx_cari_code_tenant ON cari(code, tenant_id);
CREATE INDEX idx_cari_tenant_type ON cari(tenant_id, cari_type);
```

---

## Testing

### Run Tests

```bash
# All tests
cargo test

# Specific module
cargo test --lib domain::cari

# With output
cargo test -- --nocapture

# Watch mode
cargo watch -x test
```

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_cari_account() {
        let repo = Arc::new(InMemoryCariRepository::new()) as BoxCariRepository;
        let service = CariService::new(repo);

        let create = CreateCari {
            code: "C001".to_string(),
            name: "Test Customer".to_string(),
            cari_type: CariType::Customer,
            tenant_id: 1,
            ..Default::default()
        };

        let result = service.create_cari(create).await;
        assert!(result.is_ok());
    }
}
```

---

## Security Considerations

### ✅ Implemented

1. **Password Hashing**: bcrypt with cost 12
2. **Password Validation**: 12+ chars, uppercase, lowercase, digit, special
3. **Rate Limiting**: governor crate (10 req/min, burst 3)
4. **JWT Authentication**: Bearer token with tenant claims
5. **CORS**: Configurable origins, methods, headers
6. **Tenant Isolation**: All queries filter by `tenant_id`
7. **SQL Injection Prevention**: Parameterized queries via sqlx
8. **Request Tracing**: X-Request-ID header for debugging
9. **Graceful Shutdown**: 30-second timeout for in-flight requests

### 🔒 Production Checklist

- [ ] Change default JWT secret
- [ ] Enable HTTPS
- [ ] Configure proper CORS origins (not `*`)
- [ ] Set up database backups
- [ ] Enable connection pooling limits
- [ ] Configure rate limiting per endpoint
- [ ] Set up logging aggregation
- [ ] Enable health checks in load balancer

---

## Code Conventions

### Naming
- `snake_case` for variables, functions, modules
- `CamelCase` for types, enums, traits
- `UPPER_SNAKE_CASE` for constants

### Imports Order
```rust
// 1. Standard library
use std::sync::Arc;

// 2. External crates
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};

// 3. Internal modules
use crate::config::Config;
use crate::domain::cari::model::Cari;
use crate::error::ApiError;
```

### Error Handling
```rust
// ✅ Good: Use map_err for conversions
repo.find_by_id(id, tenant_id).await?
    .ok_or(ApiError::NotFound(format!("User {} not found", id)))?;

// ✅ Good: Use helper for sqlx errors
.fetch_one(&*self.pool).await
    .map_err(|e| map_sqlx_error(e, "User"))?;

// ❌ Bad: Silent unwrap
let user = repo.find_by_id(id).await.unwrap();
```

---

## Common Pitfalls

1. **Don't use `std::sync::Mutex`**: Use `parking_lot::Mutex` instead
2. **Don't compile regex every call**: Use `lazy_static!`
3. **Don't skip password validation**: Always validate with `create.validate_password()?`
4. **Don't forget tenant isolation**: Always filter by `tenant_id`
5. **Don't use `.unwrap()` in production**: Handle errors properly
6. **Don't block async runtime**: Use `tokio::fs` instead of `std::fs`

---

## References

- [Rust Book](https://doc.rust-lang.org/book/)
- [Actix-web Documentation](https://actix.rs/docs/)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [utoipa (OpenAPI)](https://docs.rs/utoipa/)
- [parking_lot Mutex](https://docs.rs/parking_lot/)
- [governor (Rate Limiting)](https://docs.rs/governor/)

---

## Lefthook (Pre-commit Hooks)

Pre-commit and pre-push hooks are configured in `lefthook.yml`:

```bash
# Runs on commit
- cargo fmt --check
- cargo clippy -- -D warnings

# Runs on push
- cargo test

# Validates commit message format
- conventional commits (feat:, fix:, docs:, etc.)
```

---

## OpenAPI / Swagger

**Access Swagger UI:** `http://localhost:8080/swagger-ui/`

**OpenAPI JSON:** `http://localhost:8080/api-docs/openapi.json`

All endpoints are documented with `#[utoipa::path]` annotations.