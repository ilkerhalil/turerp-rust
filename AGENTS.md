# Turerp ERP - Developer Guide

## Project Overview
Multi-tenant SaaS ERP system built with Rust, Actix-web, and SQLx.

**Current Production Score: 8.7/10**

*Note: Score adjusted to reflect partial OpenAPI coverage (~13/170 handlers documented), legacy route drift (dead code not wired in production), and missing Viewer role enforcement.*

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
| `user` | User management with role-based access | ✅ Complete + PostgreSQL |
| `tenant` | Multi-tenancy with subdomain routing | ✅ Complete — Tenant CRUD + TenantConfig REST API (5 endpoints) + optional encryption |
| `cari` | Customer/Vendor accounts with credit limits | ✅ Complete + PostgreSQL |
| `product` | Product catalog, categories, units, barcodes | ✅ Complete |
| `product/variant` | Product variant CRUD | ✅ Complete — AdminUser enforced for create/update/delete |
| `stock` | Warehouses, stock levels, movements, valuation | ✅ Complete |
| `invoice` | Invoice creation, status, payments | ✅ Complete — Payments route shadow bug fixed |
| `sales` | Sales orders, quotations, conversion | ✅ Complete |
| `purchase` | Purchase orders, goods receipt, purchase requests (approval workflow) | ✅ Complete |
| `accounting` | Chart of accounts, journal entries, trial balance | ✅ Complete |
| `assets` | Fixed assets, depreciation, maintenance | ✅ Complete — Maintenance-records route shadow bug fixed |
| `project` | Project management, WBS, costs, profitability | ✅ Complete |
| `manufacturing` | BOM, work orders, routing, material requirements, quality control | ✅ Complete — Inspection + NCR REST APIs added with validation |
| `crm` | Leads, opportunities, campaigns, support tickets | ✅ Complete |
| `hr` | Employee management, attendance, leave, payroll | ✅ Complete |
| `feature` | Feature flags & tenant-specific toggles | ✅ Complete + API v1 |
| `settings` | Per-tenant configuration management with typed values & categories | ✅ Complete + API v1 |
| `audit` | Request audit trail, mpsc batch persistence | ✅ Complete + API v1 |

---

## Git Workflow

### Branching Rules

| Rule | Description |
|------|-------------|
| **No direct pushes to `main`** | All changes must be made on a feature branch and merged via pull request. |
| **Branch from `main`** | Always create your feature branch from the latest `main`. |
| **Branch naming** | Use `feature/<issue-number>-<short-description>` or `fix/<issue-number>-<short-description>`. Include the issue number so PR validation passes. |

### Creating a Pull Request

**Every PR must reference an existing, open issue.** The CI will fail if no valid issue is linked.

1. Create an issue (or pick an existing open one) for the work
2. Create a branch: `git checkout -b feature/<issue-number>-<short-description>`
3. Make changes and commit with [Conventional Commits](https://www.conventionalcommits.org/)
4. Push the branch: `git push -u origin feature/<issue-number>-<short-description>`
5. Open a pull request on GitHub — **include the issue number** in the PR title or body
6. Merge only after CI passes and approval

### PR Issue Reference Rules

| Rule | Description | Example |
|------|-------------|---------|
| **Reference required** | Every PR must link to at least one open issue | `fixes #42`, `closes #42`, `related to #42` |
| **Branch naming** | Include issue number in branch name | `feature/42-auth`, `fix/42-login-bug` |
| **PR title** | Optionally include issue number | `feat: add user auth (#42)` |
| **PR body** | Use GitHub keywords for auto-close on merge | `Fixes #42` |
| **Validation** | CI checks that referenced issues exist and are open | Fails if issue is missing or closed |

**Supported keywords:** `fixes`, `closes`, `closed`, `close`, `resolves`, `resolved`, `resolve`, `related to`, `refs`, `references`, `issue`, `#<number>`

**Branch naming convention:**
```bash
git checkout -b feature/42-user-auth        # Feature with issue #42
git checkout -b fix/123-memory-leak         # Bug fix with issue #123
git checkout -b docs/5-contributing-guide   # Docs with issue #5
```

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
TURERP_SERVER_PORT=8000

# Database (PostgreSQL feature)
TURERP_DATABASE_URL=postgres://user:pass@localhost:5432/turerp
TURERP_DB_MAX_CONNECTIONS=10
TURERP_DB_MIN_CONNECTIONS=5

# JWT
TURERP_JWT_SECRET=your-secret-key
TURERP_JWT_ACCESS_EXPIRATION=3600
TURERP_JWT_REFRESH_EXPIRATION=604800

# CORS
TURERP_CORS_ORIGINS=http://localhost:3000
TURERP_CORS_METHODS=GET,POST,PUT,DELETE,OPTIONS
TURERP_CORS_HEADERS=Authorization,Content-Type
TURERP_CORS_CREDENTIALS=true
TURERP_CORS_MAX_AGE=3600

# Rate Limiting
TURERP_TRUSTED_PROXIES=
TURERP_RATE_LIMIT_REQUESTS_PER_MINUTE=10
TURERP_RATE_LIMIT_BURST=3

# Metrics
TURERP_METRICS_ENABLED=true
TURERP_METRICS_PATH=/metrics
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
│   ├── lib.rs                  # Library exports, AppState, create_app_state
│   ├── config.rs               # Configuration management
│   ├── error.rs                # Error types (thiserror)
│   ├── api/                    # API layer
│   │   ├── mod.rs              # API module + OpenAPI (legacy routes, 13 documented paths)
│   │   ├── auth.rs             # Legacy auth routes (deprecated, dead code)
│   │   ├── users.rs            # Legacy users routes (deprecated, dead code)
│   │   └── v1/                 # API version 1 (all production routes)
│   │       ├── mod.rs
│   │       ├── auth.rs
│   │       ├── users.rs
│   │       ├── cari.rs
│   │       ├── stock.rs
│   │       ├── invoice.rs
│   │       ├── sales.rs
│   │       ├── hr.rs
│   │       ├── accounting.rs
│   │       ├── assets.rs
│   │       ├── project.rs
│   │       ├── manufacturing.rs
│   │       ├── crm.rs
│   │       ├── tenant.rs
│   │       ├── feature_flags.rs
│   │       ├── product_variants.rs
│   │       ├── purchase_requests.rs
│   │       ├── settings.rs          # Configuration management REST API
│   │       └── audit.rs
│   ├── middleware/
│   │   ├── mod.rs              # Middleware exports
│   │   ├── auth.rs           # JWT authentication + AuthUser/AdminUser extractors
│   │   ├── rate_limit.rs     # Rate limiting (governor 0.8, trusted proxy support)
│   │   ├── request_id.rs     # Request ID tracking
│   │   ├── audit.rs          # Audit logging (channel-based batch persistence)
│   │   ├── metrics.rs        # Prometheus metrics collection
│   │   └── tenant.rs         # Tenant context middleware
│   ├── domain/                 # Domain layer (DDD)
│   │   ├── mod.rs
│   │   ├── auth/             # Authentication domain
│   │   ├── user/             # User domain
│   │   ├── tenant/           # Tenant domain (includes TenantConfig)
│   │   ├── cari/             # Customer/Vendor domain
│   │   ├── product/          # Product domain (includes variants)
│   │   ├── stock/            # Stock domain
│   │   ├── invoice/          # Invoice domain
│   │   ├── sales/            # Sales domain
│   │   ├── purchase/         # Purchase domain (includes requests)
│   │   ├── hr/               # HR domain
│   │   ├── accounting/       # Accounting domain
│   │   ├── assets/           # Fixed assets domain
│   │   ├── project/          # Project domain
│   │   ├── manufacturing/    # Manufacturing domain
│   │   ├── crm/              # CRM domain
│   │   ├── audit/            # Audit log domain
│   │   ├── settings/         # Configuration management domain
│   │   └── feature/          # Feature flags domain
│   ├── common/
│   │   └── pagination.rs     # Pagination utilities (PaginatedResult, PaginationParams)
│   ├── db/
│   │   ├── mod.rs            # DB module
│   │   ├── pool.rs           # Connection pool, migrations
│   │   ├── error.rs          # Centralized DB error handling (map_sqlx_error)
│   │   └── tenant_registry.rs # Tenant pool registry
│   └── utils/
│       ├── mod.rs
│       ├── jwt.rs            # JWT utilities
│       ├── password.rs       # Password utilities
│       └── encryption.rs     # AES-256-GCM encryption
├── migrations/
│   ├── 001_initial_schema.sql
│   ├── 002_add_tenant_db_name.sql
│   ├── 003_business_modules.sql
│   ├── 004_composite_indexes.sql
│   └── 005_audit_logs.sql
├── tests/
│   ├── api_integration_test.rs   # Integration tests (38 tests)
│   └── security_test.rs          # Security tests (27 tests)
├── Cargo.toml
└── lefthook.yml
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
        // Outermost: touches request first, response last
        .wrap(middleware::Compress::default())          // 1. Response compression
        .wrap(configure_cors(&config.cors))           // 2. CORS handling
        .wrap(middleware::Logger::default())          // 3. Access logging
        .wrap(AuditLoggingMiddleware::with_sender(audit_sender.clone())) // 4. Audit logging
        .wrap(JwtAuthMiddleware::new(...))            // 5. JWT validation
        .wrap(RateLimitMiddleware::with_config(&config.rate_limit)) // 6. Rate limiting
        .wrap(MetricsMiddleware::new())               // 7. Metrics collection
        .wrap(TenantMiddleware)                       // 8. Tenant context (after auth)
        // Innermost: touches request last, response first
        .wrap(RequestIdMiddleware)                      // 9. Request ID for tracing
        .app_data(web::JsonConfig::default().limit(1024 * 1024)) // 1MB JSON limit
        .app_data(app_state.auth_service.clone())
        .app_data(app_state.user_service.clone())
        .app_data(app_state.jwt_service.clone())
        #[cfg(feature = "postgres")]
        .app_data(app_state.db_pool.clone())
        .route("/health", web::get().to(health_check))
        .route("/health/live", web::get().to(health_live))
        .route("/health/ready", web::get().to(health_ready))
        .route("/metrics", web::get().to(metrics_endpoint))
        .service(web::scope("/api").configure(v1_*_configure))
        .service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", ...))
})
```
```

---

## Health Check

**In-memory mode:**
```json
{
  "status": "ok",
  "service": "turerp-erp",
  "version": "0.1.0",
  "storage": "in-memory"
}
```

**PostgreSQL mode:**
```json
{
  "status": "ok",
  "service": "turerp-erp",
  "version": "0.1.0",
  "storage": "postgresql",
  "database": "healthy",
  "latency_ms": 2
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

**Access Swagger UI:** `http://localhost:8000/swagger-ui/`

**OpenAPI JSON:** `http://localhost:8000/api-docs/openapi.json`

~13 paths (auth, users, feature flags) are registered in the `ApiDoc` OpenAPI schema. Most v1 business module endpoints are annotated with `#[utoipa::path]` but are not yet included in the schema, so they do not appear in Swagger UI.