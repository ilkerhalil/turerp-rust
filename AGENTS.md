# Turerp ERP - Developer Guide

## Project Overview
Multi-tenant SaaS ERP system built with Rust, Actix-web, and SQLx.

**Current Production Score: 9.2/10**

*Note: Score reflects full OpenAPI coverage (724 handlers documented), comprehensive test suite (70+ test files, 1921+ tests), 50+ domain modules, and production-ready observability via OpenTelemetry/Aspire Dashboard.*

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         API Layer                                │
│  (Actix-web handlers, OpenAPI/Swagger via utoipa, REST + GraphQL)│
├─────────────────────────────────────────────────────────────────┤
│                       Middleware Layer                           │
│  (JWT Auth, Rate Limiting, Request ID, CORS, API Key, IP       │
│   Whitelist, Idempotency, Security Headers, Audit, Metrics,      │
│   Tenant Context)                                                │
├─────────────────────────────────────────────────────────────────┤
│                       Service Layer                              │
│  (Business logic, validation, orchestration, event bus)          │
├─────────────────────────────────────────────────────────────────┤
│                      Repository Layer                            │
│  (Data access, trait-based, InMemory & PostgreSQL)               │
├─────────────────────────────────────────────────────────────────┤
│                       Domain Models                              │
│  (Entities, DTOs, value objects, i18n)                           │
└─────────────────────────────────────────────────────────────────┘
```

### Domain Modules

| Domain | Description | Status |
|--------|-------------|--------|
| `auth` | Authentication & JWT tokens | Complete |
| `user` | User management with role-based access | Complete + PostgreSQL |
| `tenant` | Multi-tenancy with subdomain routing | Complete - Tenant CRUD + TenantConfig REST API |
| `cari` | Customer/Vendor accounts with credit limits | Complete + PostgreSQL |
| `product` | Product catalog, categories, units, barcodes | Complete |
| `product/variant` | Product variant CRUD | Complete |
| `stock` | Warehouses, stock levels, movements, valuation | Complete |
| `invoice` | Invoice creation, status, payments | Complete |
| `sales` | Sales orders, quotations, conversion | Complete |
| `purchase` | Purchase orders, goods receipt, purchase requests (approval workflow) | Complete |
| `accounting` | Chart of accounts, journal entries, trial balance | Complete |
| `assets` | Fixed assets, depreciation, maintenance | Complete |
| `project` | Project management, WBS, costs, profitability | Complete |
| `manufacturing` | BOM, work orders, routing, material requirements | Complete |
| `quality_control` | Inspections, non-conformance reports (NCR), QC service | Complete |
| `inter_company` | Inter-company transactions, transfer pricing | Complete |
| `crm` | Leads, opportunities, campaigns, support tickets | Complete |
| `hr` | Employee management, attendance, leave, payroll, SGK/e-Bildirge | Complete |
| `shift` | Shift planning, assignments, attendance, overtime | Complete |
| `feature` | Feature flags & tenant-specific toggles | Complete + API v1 |
| `settings` | Per-tenant configuration management with typed values & categories | Complete + API v1 |
| `audit` | Request audit trail, mpsc batch persistence | Complete + API v1 |
| `bank` | Turkish bank integration, statements, reconciliation, rules | Complete |
| `barcode` | Barcode/QR generation for products, invoices, entities | Complete |
| `api_key` | API key management with HMAC hashing and scope validation | Complete |
| `ip_whitelist` | Tenant-scoped IP access control with CIDR support | Complete |
| `archive` | Data archiving policies, jobs, record restoration | Complete |
| `custom_field` | Dynamic module attributes with typed values | Complete |
| `currency` | Multi-currency support, exchange rates, conversion | Complete |
| `tax` | Turkish tax rate management, calculation, KVB period tracking | Complete |
| `subscription` | SaaS subscription plans, billing, invoices | Complete |
| `notification` | In-app notifications, email/SMS/push delivery | Complete |
| `push` | FCM push notification token management and delivery | Complete |
| `webhook` | Webhook endpoints, delivery management, retries | Complete |
| `workflow` | Configurable approval workflows for documents and processes | Complete |
| `event` | Event bus, outbox pattern, dead letter queue (DLQ), CDC | Complete |
| `search` | Full-text search across entities with reindexing | Complete |
| `report` | Report generation (XLSX, PDF, CSV) | Complete |
| `document` | Document management with metadata and tags | Complete |
| `file` | File upload/download with S3-compatible storage, presigned URLs | Complete |
| `import` | Bulk data import with validation and templates | Complete |
| `dashboard` | BI dashboard KPIs, charts, widget management | Complete |
| `forecasting` | Inventory demand forecasting, reorder suggestions, stock alerts | Complete |
| `observability` | Health checks, SLI/SLO, alert rules, sparklines, Aspire Dashboard | Complete |
| `resilience` | Circuit breaker and retry monitoring | Complete |
| `rate_limit` | Rate limiting statistics and admin dashboard | Complete |
| `mfa` | TOTP-based multi-factor authentication with backup codes | Complete |
| `ldap` | LDAP/Active Directory user synchronization and configuration | Complete |
| `efatura` | Turkish e-Fatura (electronic invoicing) integration with GIB | Complete |
| `earchive` | Turkish e-Arsiv Fatura and E-Serbest Meslek Makbuzu | Complete |
| `edefter` | Turkish e-Defter (electronic ledger) with XML generation | Complete |
| `edefter/blockchain` | Hash-chain, Merkle tree, verification for e-Defter compliance | Complete |
| `customer_portal` | Self-service portal for customers | Complete |
| `vendor_portal` | Self-service portal for vendors | Complete |
| `company` | Company profile and legal information | Complete |
| `cost_center` | Cost center and profit center with allocations | Complete |
| `chart_of_accounts` | Chart of accounts tree, trial balance, recalculation | Complete |
| `job` | Background job scheduler (cron-based) | Complete |

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
5. Open a pull request on GitHub - **include the issue number** in the PR title or body
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

### Milestone & Issue Workflow Rules

**Work sequentially by milestone - never skip ahead.**

| Milestone | Phase | Priority |
|-----------|-------|----------|
| **v1.0 Production Readiness** | Core infrastructure | P0 |
| **v1.1 Enterprise Foundation** | Enterprise features | P1 / P2 |
| **v1.2 Scale & Observability** | Resilience & monitoring | P1 / P2 |
| **v1.3 Advanced & AI** | Emerging tech & portals | P3 |

#### Issue Picking Order

1. **Milestone order first:** Finish v1.0 before touching v1.1, v1.1 before v1.2, etc.
2. **Priority within milestone:** P0 -> P1 -> P2 -> P3
3. **Epics last:** Close all child issues of an epic before closing the epic itself
4. **One issue at a time:** Pick ONE open issue, create a branch, implement, open PR

#### Sequential Merge Rule

```
Issue A: branch -> PR -> CI pass -> merge to main -> Issue B: branch -> PR -> ...
```

- **Never start Issue B before Issue A's PR is merged to `main`.**
- Wait for CI to pass and PR to merge before picking the next issue.
- If a PR is blocked (review pending, CI failing), fix it - do not switch to another issue.

#### Why Sequential?

- Prevents branch drift and merge conflicts
- Keeps `main` always deployable
- Makes rollback trivial (one PR = one change)
- Enforces focus: one feature, one branch, one merge

---

## Quick Start

```bash
# Development (in-memory storage)
cargo run

# Production (PostgreSQL) — set TURERP_DATABASE_URL
cargo run

# Run tests
cargo test

# Code quality
cargo clippy -- -D warnings
cargo fmt --check

# Generate OpenAPI JSON
cargo run --bin gen_openapi
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

# OpenTelemetry / Aspire Dashboard
TURERP_OTEL_ENABLED=true
TURERP_OTEL_ENDPOINT=http://localhost:4317
TURERP_OTEL_SERVICE_NAME=turerp

# S3 / File Storage
TURERP_S3_ENDPOINT=
TURERP_S3_BUCKET=
TURERP_S3_ACCESS_KEY=
TURERP_S3_SECRET_KEY=

# Redis
TURERP_REDIS_URL=redis://localhost:6379

# SMTP / Email
TURERP_SMTP_HOST=
TURERP_SMTP_PORT=587
TURERP_SMTP_USER=
TURERP_SMTP_PASSWORD=

# Vault (optional)
TURERP_VAULT_ADDR=
TURERP_VAULT_TOKEN=
TURERP_VAULT_PATH=

# e-Fatura / GIB
TURERP_GIB_API_URL=
TURERP_GIB_USERNAME=
TURERP_GIB_PASSWORD=
TURERP_GIB_TEST_MODE=true
```

---

## Rust Best Practices

### 1. Error Handling

**Use `thiserror` for custom error types**

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
// Bad: Can panic on poisoned mutex
use std::sync::Mutex;
let guard = self.users.lock().unwrap();

// Good: parking_lot::Mutex never poisons
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
│   ├── config.rs               # Configuration management (env + file)
│   ├── error.rs                # Error types (thiserror)
│   ├── api/                    # API layer
│   │   ├── mod.rs              # API module + OpenAPI spec (724 documented paths)
│   │   ├── auth.rs             # Legacy auth routes (deprecated)
│   │   ├── users.rs            # Legacy users routes (deprecated)
│   │   └── v1/                 # API version 1 (all production routes)
│   │       ├── mod.rs          # Module exports + route configuration
│   │       ├── auth.rs         # Login, register, refresh, me
│   │       ├── mfa.rs          # TOTP setup, verify, disable, backup codes
│   │       ├── users.rs        # User CRUD + soft delete
│   │       ├── cari.rs         # Customer/Vendor CRUD
│   │       ├── stock.rs        # Warehouses, movements, levels
│   │       ├── invoice.rs      # Invoices, payments, status
│   │       ├── sales.rs        # Sales orders, quotations
│   │       ├── purchase_orders.rs      # Purchase orders
│   │       ├── purchase_requests.rs    # Purchase requests + approval
│   │       ├── goods_receipts.rs       # Goods receipts
│   │       ├── hr/             # HR submodule
│   │       │   ├── employees.rs
│   │       │   ├── attendance.rs
│   │       │   ├── leave.rs
│   │       │   ├── payroll.rs
│   │       │   └── sgk.rs      # e-Bildirge / SGK
│   │       ├── accounting.rs   # Accounts, journal entries, trial balance
│   │       ├── assets.rs       # Fixed assets, depreciation, maintenance
│   │       ├── project.rs      # Projects, WBS, costs, profitability
│   │       ├── manufacturing/  # Manufacturing submodule
│   │       │   ├── work_orders.rs
│   │       │   ├── boms.rs
│   │       │   ├── routings.rs
│   │       │   └── quality_control.rs
│   │       ├── crm/            # CRM submodule
│   │       │   ├── leads.rs
│   │       │   ├── opportunities.rs
│   │       │   ├── campaigns.rs
│   │       │   └── tickets.rs
│   │       ├── tenant.rs       # Tenant CRUD + config
│   │       ├── feature_flags.rs        # Feature flag management
│   │       ├── product_variants/       # Products submodule
│   │       │   ├── products.rs
│   │       │   ├── categories.rs
│   │       │   ├── units.rs
│   │       │   └── variants.rs
│   │       ├── bank.rs         # Bank accounts, statements, reconciliation
│   │       ├── barcode.rs      # Barcode/QR generation
│   │       ├── api_keys.rs     # API key management
│   │       ├── ip_whitelist.rs # IP access control
│   │       ├── archive.rs      # Archiving policies and jobs
│   │       ├── custom_fields.rs        # Dynamic attributes
│   │       ├── currency.rs     # Currencies and exchange rates
│   │       ├── tax/            # Tax submodule
│   │       │   ├── rates.rs
│   │       │   └── periods.rs
│   │       ├── subscription.rs # Subscription plans and billing
│   │       ├── notifications.rs        # In-app notifications
│   │       ├── push_notifications.rs   # FCM push notifications
│   │       ├── webhooks.rs     # Webhook management
│   │       ├── workflow.rs     # Approval workflows
│   │       ├── events.rs       # Event bus, outbox, DLQ, CDC
│   │       ├── search.rs       # Full-text search
│   │       ├── reports.rs      # Report generation
│   │       ├── documents.rs    # Document management
│   │       ├── files.rs        # File upload/download
│   │       ├── import.rs       # Bulk data import
│   │       ├── dashboard.rs    # KPIs and charts
│   │       ├── forecasting.rs  # Demand forecasting
│   │       ├── observability.rs        # Health, SLI/SLO, alerts
│   │       ├── resilience.rs   # Circuit breakers, retry stats
│   │       ├── rate_limits.rs  # Rate limit statistics
│   │       ├── audit.rs        # Audit log retrieval
│   │       ├── settings.rs     # Configuration management
│   │       ├── shifts.rs       # Shift planning
│   │       ├── chart_of_accounts.rs    # Chart of accounts
│   │       ├── cost_centers.rs # Cost center management
│   │       ├── ldap.rs         # LDAP/AD sync
│   │       ├── efatura.rs      # e-Fatura integration
│   │       ├── earchive.rs     # e-Arsiv integration
│   │       ├── edefter.rs      # e-Defter integration
│   │       ├── edefter_blockchain.rs   # Blockchain verification
│   │       ├── customer_portal.rs      # Customer self-service
│   │       ├── vendor_portal.rs        # Vendor self-service
│   │       ├── graphql.rs      # GraphQL endpoint
│   │       └── companies.rs    # Company profiles
│   ├── middleware/
│   │   ├── mod.rs              # Middleware exports
│   │   ├── auth.rs             # JWT authentication + extractors
│   │   ├── rate_limit.rs       # Rate limiting (governor 0.8)
│   │   ├── request_id.rs       # Request ID tracking
│   │   ├── audit.rs            # Audit logging (channel-based)
│   │   ├── metrics.rs          # Prometheus metrics collection
│   │   ├── tenant.rs           # Tenant context middleware
│   │   ├── api_key.rs          # API key validation
│   │   ├── ip_whitelist.rs     # IP whitelist validation
│   │   ├── idempotency.rs      # Idempotency key handling
│   │   └── security_headers.rs # Security headers (HSTS, CSP, etc.)
│   ├── domain/                 # Domain layer (DDD)
│   │   ├── mod.rs
│   │   ├── auth/               # Authentication domain
│   │   ├── user/               # User domain
│   │   ├── tenant/             # Tenant domain
│   │   ├── cari/               # Customer/Vendor domain
│   │   ├── product/            # Product domain
│   │   ├── stock/              # Stock domain
│   │   ├── invoice/            # Invoice domain
│   │   ├── sales/              # Sales domain
│   │   ├── purchase/           # Purchase domain
│   │   ├── accounting/         # Accounting domain
│   │   ├── assets/             # Fixed assets domain
│   │   ├── project/            # Project domain
│   │   ├── manufacturing/      # Manufacturing domain
│   │   ├── quality_control/    # Quality control domain (NCR, inspections)
│   │   ├── inter_company/      # Inter-company transactions domain
│   │   ├── crm/                # CRM domain
│   │   ├── hr/                 # HR domain
│   │   ├── shift/              # Shift planning domain
│   │   ├── audit/              # Audit log domain
│   │   ├── settings/           # Configuration domain
│   │   ├── feature/            # Feature flags domain
│   │   ├── bank/               # Bank integration domain
│   │   ├── barcode/            # Barcode domain
│   │   ├── api_key/            # API key domain
│   │   ├── ip_whitelist/       # IP whitelist domain
│   │   ├── archive/            # Archive domain
│   │   ├── custom_field/       # Custom fields domain
│   │   ├── currency/           # Currency domain
│   │   ├── tax/                # Tax domain
│   │   ├── subscription/       # Subscription domain
│   │   ├── notification/       # Notification domain
│   │   ├── webhook/            # Webhook domain
│   │   ├── workflow/           # Workflow domain
│   │   ├── event/              # Event bus domain
│   │   ├── search/             # Search domain
│   │   ├── report/             # Report domain
│   │   ├── document/           # Document domain
│   │   ├── file/               # File storage domain
│   │   ├── import/             # Import domain
│   │   ├── dashboard/          # Dashboard domain
│   │   ├── forecasting/        # Forecasting domain
│   │   ├── observability/      # Observability domain
│   │   ├── resilience/         # Resilience domain
│   │   ├── mfa/                # MFA domain
│   │   ├── ldap/               # LDAP domain
│   │   ├── efatura/            # e-Fatura domain
│   │   ├── earchive/           # e-Arsiv domain
│   │   ├── edefter/            # e-Defter domain
│   │   ├── customer_portal/    # Customer portal domain
│   │   ├── vendor_portal/      # Vendor portal domain
│   │   ├── company/            # Company domain
│   │   ├── cost_center/        # Cost center domain
│   │   ├── chart_of_accounts/  # Chart of accounts domain
│   │   └── job/                # Job scheduler domain
│   ├── common/
│   │   ├── mod.rs              # Common exports
│   │   ├── pagination.rs       # Pagination utilities
│   │   ├── notifications.rs    # Notification helpers
│   │   ├── import/             # Import utilities
│   │   ├── reports/            # Report generation utilities
│   │   ├── circuit_breaker.rs  # Circuit breaker implementation
│   │   └── retry.rs            # Retry policies
│   ├── db/
│   │   ├── mod.rs              # DB module
│   │   ├── pool.rs             # Connection pool, migrations
│   │   ├── error.rs            # Centralized DB error handling
│   │   └── tenant_registry.rs  # Tenant pool registry
│   ├── graphql/
│   │   ├── mod.rs              # GraphQL schema exports
│   │   ├── query.rs            # GraphQL queries
│   │   └── mutation.rs         # GraphQL mutations
│   ├── i18n/
│   │   └── mod.rs              # Internationalization utilities
│   ├── cache/
│   │   └── mod.rs              # Redis caching layer
│   ├── utils/
│   │   ├── mod.rs
│   │   ├── jwt.rs              # JWT utilities
│   │   ├── password.rs         # Password utilities
│   │   └── encryption.rs       # AES-256-GCM encryption
│   └── bin/
│       └── gen_openapi.rs      # OpenAPI JSON generator binary
├── migrations/
│   ├── 001_initial_schema.sql
│   ├── 002_add_tenant_db_name.sql
│   ├── 003_business_modules.sql
│   ├── 004_composite_indexes.sql
│   ├── 005_audit_logs.sql
│   ├── 006_settings.sql
│   ├── 007_soft_delete.sql
│   ├── 008_custom_fields.sql
│   ├── 009_chart_of_accounts.sql
│   ├── 010_webhooks.sql
│   ├── 011_edefter.sql
│   ├── 012_tax_engine.sql
│   ├── 013_efatura.sql
│   ├── 014_api_keys.sql
│   ├── 015_currency.sql
│   ├── 015_mfa.sql
│   ├── 016_full_text_search.sql
│   ├── 017_notifications.sql
│   ├── 018_jobs.sql
│   ├── 019_soft_delete_users_tenants.sql
│   ├── 020_soft_delete_complete.sql
│   ├── 021_files_table.sql
│   ├── 021_outbox.sql
│   ├── 022_cdc_triggers.sql
│   ├── 023_companies.sql
│   ├── 023_cost_centers.sql
│   ├── 024_workflows.sql
│   ├── 025_bank_integration.sql
│   ├── 026_subscriptions.sql
│   ├── 027_observability.sql
    └── 028_missing_repos.sql
├── tests/
│   ├── common/
│   │   ├── mod.rs
│   │   ├── app.rs              # Test app factory
│   │   ├── auth.rs             # Test authentication helpers
│   │   ├── factories.rs        # Test data factories
│   │   └── assertions.rs       # Test assertions
│   ├── api_integration_test.rs
│   ├── security_test.rs
│   ├── health_check_test.rs
│   ├── performance_test.rs
│   ├── soft_delete_test.rs
│   ├── p0_cross_module_test.rs
│   ├── bank_account_test.rs
│   ├── bank_reconciliation_test.rs
│   ├── bank_transaction_test.rs
│   ├── cost_center_allocation_test.rs
│   ├── cost_center_crud_test.rs
│   ├── customer_portal_test.rs
│   ├── vendor_portal_test.rs
│   ├── dashboard_integration_test.rs
│   ├── files_integration_test.rs
│   ├── observability_test.rs
│   ├── subscription_plan_test.rs
│   ├── subscription_auth_test.rs
│   ├── subscription_billing_test.rs
│   ├── workflow_template_test.rs
│   ├── workflow_instance_test.rs
│   └── workflow_auth_test.rs
├── Cargo.toml
├── lefthook.yml
└── openapi.json                # Generated OpenAPI 3.0.3 spec (516 paths)
```

---

## Runtime Backend Selection

PostgreSQL and in-memory backends are selected at runtime via configuration, not compile-time feature flags.

```rust
// lib.rs — unified AppState (no #[cfg(feature = "postgres")])
pub struct AppState {
    pub auth_service: web::Data<AuthService>,
    pub user_service: web::Data<UserService>,
    pub jwt_service: web::Data<JwtService>,
    pub db_pool: Option<web::Data<Arc<PgPool>>>,
}

// create_app_state_unified() chooses backend at runtime:
//   - If TURERP_DATABASE_URL is set → PostgreSQL path
//   - Otherwise → in-memory path
```

All repository traits have dual implementations (`InMemory*` and `Postgres*`). The `create_app_state_unified()` function in `lib.rs` wires the correct set at startup based on `config.database_url`.

---

## Middleware Stack

**Order matters! First `.wrap()` = innermost (touches request LAST).**

```rust
HttpServer::new(move || {
    App::new()
        // Innermost first: touches request LAST, response FIRST
        .wrap(TracingMiddleware)                      // 1. Tracing (needs AuthClaims + RequestId)
        .wrap(RequestIdMiddleware)                    // 2. Request ID generation
        // Outermost last: touches request FIRST, response LAST
        .wrap(middleware::Compress::default())        // 3. Response compression
        .wrap(configure_cors(&config.cors))           // 4. CORS handling
        .wrap(AuditLoggingMiddleware::with_sender(audit_sender.clone())) // 5. Audit logging (after auth)
        .wrap(JwtAuthMiddleware::new(...))            // 6. JWT validation
        .wrap(IpWhitelistMiddleware)                   // 7. IP whitelist (after JWT)
        .wrap(RateLimitMiddleware::with_config(&config.rate_limit)) // 8. Rate limiting (outermost security)
        .wrap(MetricsMiddleware::new())               // 9. Metrics collection
        .wrap(TenantMiddleware)                       // 10. Tenant context (after auth)
        .wrap(SecurityHeadersMiddleware)              // 11. Security headers
        .app_data(web::JsonConfig::default().limit(1024 * 1024)) // 1MB JSON limit
        // Services registered via AppState::register_services()
})
```

**Key ordering rule:** TracingMiddleware must be innermost so `AuthClaims` and `RequestId` are already populated in extensions when it runs.

---

## Health Check

Two endpoints are exposed. Use them according to the table below.

| Endpoint | Purpose | Use for |
|---|---|---|
| `GET /health/live` | Liveness — process is up and accepting requests. No dependency checks. | Docker `HEALTHCHECK`, Kubernetes `livenessProbe` |
| `GET /health/ready` | Readiness — DB / cache / scheduler are reachable. | Kubernetes `readinessProbe`, compose `condition: service_healthy` |

**Response — `/health/live` (in-memory mode):**
```json
{
  "status": "ok",
  "service": "turerp-erp",
  "version": "0.1.0",
  "storage": "in-memory"
}
```

**Response — `/health/ready` (PostgreSQL mode):**
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

### Docker HEALTHCHECK

The `turerp` Dockerfile uses `wget` (not in `debian:bookworm-slim` by default, so the Dockerfile installs it via `apt-get install -y --no-install-recommends wget` — see the runtime stage in `turerp/Dockerfile`):

```dockerfile
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://127.0.0.1:8080/health/live >/dev/null 2>&1 || exit 1
```

- `--interval=30s`: a healthy container is checked every 30s.
- `--timeout=5s`: a single probe must complete within 5s; longer than this and the container is marked unhealthy.
- `--start-period=10s`: gives the process 10s to bind the port before the first probe runs.
- `--retries=3`: three consecutive failures before marking unhealthy (avoids flapping on transient blips).

Use `127.0.0.1` (not `localhost`) to skip DNS resolution — saves ~1-2ms and avoids resolver-related flakiness.

---

## Database Migrations

Migrations are run automatically on startup via `sqlx::migrate!()`. See `migrations/` directory for all SQL files.

Key migration areas:
- **001-005**: Core schema (users, tenants, cari, audit)
- **006-010**: Configuration (settings, soft delete, custom fields, chart of accounts, webhooks)
- **011-015**: Turkish compliance (edefter, tax, efatura, api keys, currency, mfa)
- **016-020**: Infrastructure (search, notifications, jobs, soft delete completion, outbox, CDC)
- **021-024**: Business features (files, companies, cost centers, workflows)
- **025-027**: Integrations (bank, subscriptions, observability)

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

### Implemented

1. **Password Hashing**: bcrypt with cost 12
2. **Password Validation**: 12+ chars, uppercase, lowercase, digit, special
3. **Rate Limiting**: governor crate (10 req/min, burst 3)
4. **JWT Authentication**: Bearer token with tenant claims
5. **MFA/TOTP**: Time-based one-time passwords with backup codes
6. **CORS**: Configurable origins, methods, headers
7. **Tenant Isolation**: All queries filter by `tenant_id`
8. **SQL Injection Prevention**: Parameterized queries via sqlx
9. **Request Tracing**: X-Request-ID header for debugging
10. **Graceful Shutdown**: 30-second timeout for in-flight requests
11. **API Key Authentication**: HMAC-hashed keys with scope validation
12. **IP Whitelisting**: CIDR-based tenant-scoped access control
13. **Security Headers**: HSTS, CSP, X-Frame-Options, etc.
14. **Idempotency**: Idempotency key handling for safe retries
15. **Encryption**: AES-256-GCM for sensitive data at rest
16. **Secrets Management**: HashiCorp Vault integration

### Production Checklist

- [ ] Change default JWT secret
- [ ] Enable HTTPS
- [ ] Configure proper CORS origins (not `*`)
- [ ] Set up database backups
- [ ] Enable connection pooling limits
- [ ] Configure rate limiting per endpoint
- [ ] Set up logging aggregation (OpenTelemetry)
- [ ] Enable health checks in load balancer
- [ ] Configure Vault for secrets
- [ ] Set up Redis for caching
- [ ] Configure S3-compatible storage
- [ ] Set up monitoring dashboards (Aspire)

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
// Good: Use map_err for conversions
repo.find_by_id(id, tenant_id).await?
    .ok_or(ApiError::NotFound(format!("User {} not found", id)))?;

// Good: Use helper for sqlx errors
.fetch_one(&*self.pool).await
    .map_err(|e| map_sqlx_error(e, "User"))?;

// Bad: Silent unwrap
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
7. **Don't forget OpenAPI annotations**: Add `#[utoipa::path(...)]` to every new handler
8. **Don't forget to register handlers**: Add new handlers to `api/mod.rs` paths list

---

## Code Review & Production Readiness

This section captures the lessons from the round-3 production hardening pass. Apply these on every non-trivial change.

### Adversarial review for 3+ file changes

A single agent reviewing its own work tends to find "it works" rather than "it breaks." For any change touching 3 or more files, run **at least 2 parallel reviewer agents with distinct lenses** before merging. Example lens combinations:

| Change shape | Lens 1 | Lens 2 |
|---|---|---|
| API/feature | correctness | security |
| Database/schema | schema/index design | performance + tenant isolation |
| Deployment/config | deploy correctness | security defaults |
| Refactor | idiom/style | regression risk |

Each reviewer must end with a verdict (`SAFE` / `NEEDS REVIEW` / `RISKY`) and the lead synthesizes a verdict table before any commit.

### Production failure-mode checklist

For every change, reason about each of the following before merging. If the answer is "I don't know" or "we'll find out in prod", the change is not ready.

1. **Silent error swallowing.** Does any `Err` arm `warn!()` and continue? If yes, the schema/state can diverge from the recorded history. Either propagate the error, or make the recovery state observable (e.g. record the failure in a separate table).
2. **Default-value escape hatches.** Does any env var, config, or secret have a default that "works"? If yes, the operator can deploy with a publicly known value. Use `env:?` (compose) or `required-env` (Rust) for secrets. Never default a key, password, or encryption material.
3. **Panic paths in startup.** Does any code path `panic!`/`unwrap`/`expect` during construction or config parsing? Container orchestrators will restart-loop the container instead of surfacing the error. Validate config at startup and return errors.
4. **Untested happy-path assumptions.** Does the code assume an input format, header, or env var that we haven't verified? Read the source, grep for the function, confirm it exists and does what you think.
5. **Inconsistent fix application.** If you fixed a bug pattern in one place (e.g. a function used `std::sync::Mutex` instead of `parking_lot::Mutex`), grep the whole codebase for the same pattern. Don't ship a half-applied fix.
6. **Backward-compat on already-migrated DBs.** Any new constraint, index, or column needs a plan for databases that already have the prior schema. `IF NOT EXISTS`, `IF EXISTS`, `DROP IF EXISTS`, and `NOT VALID` are the standard tools.

### Pre-merge verification matrix

Before opening a PR that touches the listed file types, run the corresponding verifications:

| File type | Required checks |
|---|---|
| `migrations/*.sql` | Migration set re-runs cleanly on a fresh DB AND on a snapshot of the current production schema. New CHECK/UNIQUE constraints are tested on both. |
| `turerp/src/**/*.rs` | `cargo clippy -- -D warnings`, `cargo fmt --check`, full `cargo test` suite, OpenAPI spec regenerated and diff reviewed. |
| `turerp/Dockerfile`, `docker-compose.yml` | `docker compose config` parses without error; required env vars documented in AGENTS.md; HEALTHCHECK command exits 0 against a running container. |
| `turerp/migrations/*.sql` + Rust | Migration order verified — no cross-file reference that depends on a later file. |
| `turerp/src/**/auth/**`, `turerp/src/**/user/**`, MFA, JWT, password, RBAC | **Live auth smoke test** — see the rule below. In addition to unit tests, the PR must include a live curl-based verification of both the happy path and at least one negative path (e.g. wrong password, expired token, missing role, MFA required but not provided). |

### Endpoint-existence rule

Before referencing any HTTP path in code, config, or docs (e.g. a Docker HEALTHCHECK, a readiness probe, a CORS example), **verify the path actually exists in the current source**. Don't trust memory, comments from a prior PR, or another agent's claim — grep the route registration site and the public-path list in `turerp/src/middleware/auth.rs`.

### Live auth / permission smoke test (mandatory for auth-touching PRs)

PRs that touch authentication, authorization, MFA, JWT, password handling, RBAC, or any other access-control code path must include a **live smoke test of the full request/response cycle** — not just unit tests. The PR #147 incident (login authenticated users with any password) would have been caught by such a test, but was invisible to both code review and the unit-test suite.

Concretely, every such PR must demonstrate, in the PR body or in a captured transcript:

1. **Happy path** — register/login → receive valid JWT → call a protected endpoint → 200.
2. **At least one negative path per security boundary** — for login, the canonical negative is "wrong password returns 401, not 200"; for JWT, "expired/invalid token returns 401, not 200"; for RBAC, "user without the required role returns 403, not 200"; for MFA, "valid password but no MFA code returns 403 with a temporary token, not a full session".
3. **Brute-force / rate-limit / lockout path** — when applicable, demonstrate the protective control firing (e.g. 5 wrong attempts → "temporarily locked" 401).
4. **Regression check on adjacent endpoints** — `/health/*`, `/metrics`, an unrelated authenticated endpoint — to confirm the change did not break neighbouring flows.

The test must run against a real container (`docker compose up -d ...`), not against mocks. Unit tests are necessary but not sufficient for auth code: they cannot catch the class of bug where the function chain compiles cleanly but the security boundary is dropped on the floor at the integration seam (e.g. `verify_password(...)?` returning `Ok(())` on a mismatch — the unit test passes, the live test fails).

If a PR touching auth cannot run a live smoke test (e.g. CI-only environment without a DB), it must say so explicitly and propose when the test will be run.

### Shared password verification API

Use `crate::utils::password::check_password` (not `verify_password`) anywhere the result is the basis for an authentication decision. `check_password` collapses bcrypt's `Ok(false)` into `Err(ApiError::InvalidCredentials)` so the `?` operator cannot silently drop the negative case. The earlier `verify_credentials` bug in PR #147 is exactly what `check_password` is designed to prevent.

---

## OpenAPI / Swagger

**Access Swagger UI:** `http://localhost:8000/swagger-ui/`

**OpenAPI JSON:** `http://localhost:8000/api-docs/openapi.json`

**OpenAPI file:** `turerp/openapi.json` (generated via `cargo run --bin gen_openapi`)

All 724 v1 business module endpoints are annotated with `#[utoipa::path]` and registered in the `ApiDoc` OpenAPI schema. The spec includes:
- Full path coverage for all REST endpoints
- Request/response schemas for all DTOs
- Security scheme (Bearer JWT)
- Tags for logical grouping
- Error response documentation

**To regenerate after adding new endpoints:**
```bash
cargo run --bin gen_openapi
```

---

## GraphQL

**Endpoint:** `POST /graphql`

GraphQL API provides an alternative query interface for complex data fetching. See `src/graphql/` for schema definitions.

---

## i18n (Internationalization)

The application supports per-request localization via the `Accept-Language` header. Supported languages:
- `en` (default)
- `tr`

---

## References

- [Rust Book](https://doc.rust-lang.org/book/)
- [Actix-web Documentation](https://actix.rs/docs/)
- [SQLx Documentation](https://docs.rs/sqlx/)
- [utoipa (OpenAPI)](https://docs.rs/utoipa/)
- [parking_lot Mutex](https://docs.rs/parking_lot/)
- [governor (Rate Limiting)](https://docs.rs/governor/)
- [OpenTelemetry Rust](https://docs.rs/opentelemetry/)
- [async-graphql](https://docs.rs/async-graphql/)

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
