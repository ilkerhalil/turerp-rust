# Turerp ERP Implementation Plan

## Project Overview
Multi-tenant SaaS ERP system built with Rust using Actix-web and SQLx.

## Implementation Strategy
- Start with core foundational modules (Auth, Tenants, Users)
- Build core business modules (Cari, Products, Stock, Invoices)
- Extend to sales/purchase, HR, accounting
- Add specialized modules (Projects, Manufacturing, CRM)

---

## Phase 1: Foundation (Week 1-2)

### 1.1 Project Setup
- [x] Initialize Rust project with Cargo
- [x] Set up project structure (domain-driven design)
- [x] Configure logging and error handling
- [x] Set up database migrations framework (sqlx migrations)

### 1.2 Authentication Module
- [x] User model and repository
- [x] Password hashing (bcrypt with cost 12)
- [x] JWT token generation and validation
- [x] Registration endpoint
- [x] Login endpoints
- [x] Token refresh mechanism
- [x] OpenAPI/Swagger documentation
- [x] Rate limiting (10 req/min per IP)
- [x] Password complexity validation (12+ chars, complexity rules)

### 1.3 Tenants Module
- [x] Tenant model and repository
- [x] Subdomain validation
- [x] Tenant CRUD operations
- [x] Database routing per tenant (TenantPoolRegistry)

### 1.4 Users Module
- [x] User management within tenant
- [x] Role assignment (Admin, User, Viewer)
- [x] User CRUD endpoints
- [x] User validation tests

### 1.5 Feature Flags Module
- [x] Feature flag model
- [x] CRUD operations
- [x] Tenant-specific toggles
- [x] API endpoints (v1)
- [x] Admin authorization for modifications

### 1.6 Configuration Module
- [x] Global config management
- [x] Environment-based configuration
- [x] Production validation (JWT secret strength)
- [x] Tenant-specific config (TenantConfigRepository)
- [x] Encrypted storage for sensitive values (AES-256-GCM)

---

## Phase 2: Core Business Modules (Week 3-4)

### 2.1 Cari Module (Customer/Vendor)
- [x] CariAccount model
- [x] CRUD operations
- [x] Account type filtering (customer/vendor/both)
- [x] Search by code/name
- [x] Credit limit management
- [x] Status management (Active, Passive, Blocked)
- [x] PostgreSQL repository implementation

### 2.2 Products Module
- [x] Product model
- [x] Category management
- [x] Unit of measure
- [x] Product variants (CRUD operations)
- [x] Barcode support

### 2.3 Stock Module
- [x] Warehouse management
- [x] Stock levels
- [x] Stock movements (IN/OUT/TRANSFER)
- [x] Stock valuation
- [x] Insufficient stock handling

### 2.4 Invoices Module
- [x] Invoice model
- [x] Invoice creation
- [x] Payment tracking
- [x] Invoice status management
- [x] Tax calculations

---

## Phase 3: Sales & Purchase (Week 5)

### 3.1 Sales Module
- [x] Sales orders
- [x] Quotations
- [x] Quotation to order conversion
- [x] Sales pricing
- [x] Tax and discount calculations

### 3.2 Purchase Module
- [x] Purchase requests (approval workflow)
- [x] Purchase orders
- [x] Goods receipt
- [x] Vendor management
- [x] Status transition validation (state machine)

---

## Phase 4: HR Module (Week 6)

### 4.1 HR Module
- [x] Employee management
- [x] Payroll
- [x] Attendance tracking
- [x] Leave management
- [x] Leave types

---

## Phase 5: Finance (Week 7)

### 5.1 Accounting Module
- [x] Chart of accounts
- [x] Journal entries
- [x] Trial balance
- [x] Account balances
- [x] Posting mechanism

### 5.2 Assets Module
- [x] Fixed assets model and repository
- [x] Depreciation calculations (straight-line, declining balance)
- [x] Maintenance tracking
- [x] In-memory repository for testing

---

## Phase 6: Projects (Week 8)

### 6.1 Projects Module
- [x] Project management
- [x] Work Breakdown Structure (WBS)
- [x] Project costs

### 6.2 Project Finance
- [x] Budget tracking
- [x] Profitability analysis

---

## Phase 7: Manufacturing (Week 9-10)

### 7.1 Manufacturing Module
- [x] Work orders
- [x] Routing
- [x] Production tracking
- [x] Production time calculation

### 7.2 BOM Module
- [x] Bill of Materials management
- [x] Material requirements calculation

### 7.3 Quality Control
- [x] Inspections
- [x] Non-conformance reports (NCR)

### 7.4 Shop Floor
- [x] Work order operations tracking

---

## Phase 8: CRM (Week 11)

### 8.1 CRM Module
- [x] Lead management
- [x] Opportunity tracking
- [x] Campaign management
- [x] Support tickets
- [x] Sales pipeline
- [x] Dashboard metrics

---

## Phase 9: Integration & Polish (Week 12)

### 9.1 API Documentation
- [x] OpenAPI/Swagger UI
- [x] API versioning (/api/v1/ prefix, backward compatibility)
- [x] Rate limiting (governor crate)

### 9.2 Testing & Security
- [x] Unit tests (225 passing)
- [x] Integration tests (38 passing)
- [x] Security tests (27 passing - OWASP Top 10)
- [x] Request ID middleware
- [x] Audit logging middleware
- [x] JWT authentication middleware
- [x] Password complexity validation
- [x] Production config validation
- [x] Admin role authorization (AdminUser extractor)
- [x] Tenant isolation enforcement
- [x] SQL injection tests
- [x] Input validation tests
- [x] JWT tampering tests
- [x] Authentication security tests
- [x] HTTP method security tests
- [x] IDOR / tenant isolation tests
- [x] Authorization tests (normal user cannot access admin endpoints)

### 9.3 Code Quality
- [x] cargo check passes
- [x] cargo clippy passes (no warnings)
- [x] cargo fmt passes
- [x] No ambiguous glob re-exports
- [x] No unused imports
- [x] No dead code
- [x] Proper error handling (thiserror)
- [x] Request tracing (tracing crate)

---

## Phase 10: Security Hardening v2

### 10.1 Authorization Enhancement
- [x] Role-based authorization middleware
- [x] Admin-only endpoint protection for Users API
- [x] Permission checks for delete/update operations

### 10.2 Data Integrity
- [x] Migrate all monetary values from f64 to Decimal
- [x] Migrate all quantity values from f64 to Decimal
- [x] Update service layer calculations

### 10.3 Concurrency Safety
- [x] Consolidate multiple Mutex fields in repositories
- [x] Apply single inner struct pattern consistently

### 10.4 Code Quality
- [x] Add #[must_use] attributes to important return types
- [x] Fix AdminUser extractor role comparison (case sensitivity bug)
- [x] Fix all Clippy warnings (needless_borrows, manual_range_contains, etc.)
- [x] Fix InvoiceStatus and ProjectStatus Default implementations (derive macro)
- [x] Add trusted proxy configuration for rate limiting
- [x] Improve error context in database operations

---

## Phase 11: Decimal Migration & Code Quality (Complete)

### 11.1 Decimal Migration - Financial Precision
- [x] Sales module - all monetary fields migrated to Decimal
- [x] Invoice module - subtotal, tax, discount, total, quantity
- [x] Stock module - quantity, reserved_quantity, avg_cost, total_value
- [x] Product module - purchase_price, sale_price, tax_rate, price_modifier
- [x] HR module - salary, hours, payroll calculations
- [x] Accounting module - debit, credit, balances
- [x] Project module - budget, costs, revenue, profit
- [x] Manufacturing module - quantities, hours, scrap_percentage
- [x] CRM module - opportunity value, campaign budget
- [x] Cari module - credit_limit, current_balance
- [x] All tests updated to use `rust_decimal_macros::dec!`

### 11.2 Mutex Consolidation - Thread Safety
- [x] Sales repositories - single inner struct pattern
- [x] Accounting repositories - atomic state updates
- [x] Stock repositories - reduced lock contention
- [x] Manufacturing repositories - consistent state
- [x] HR repositories - thread-safe operations
- [x] Cari repository - atomic updates
- [x] Project repositories - single lock acquisition
- [x] Tenant repositories - config isolation
- [x] CRM repositories - thread-safe CRUD
- [x] Invoice repositories - payment tracking
- [x] Product repositories - variant handling
- [x] Purchase repositories - order management

### 11.3 Clippy Fixes - Code Quality
- [x] needless_borrows_for_generic_args in security_test.rs
- [x] manual_range_contains in hr/model.rs
- [x] needless_borrow in tenant/model.rs
- [x] len_zero in tenant/service.rs
- [x] unnecessary_literal_unwrap in error.rs
- [x] default_constructed_unit_structs in middleware/tenant.rs
- [x] derivable_impls for InvoiceStatus and ProjectStatus

---

## Current Status: Phase 12 - Complete ✅

### Completed Modules
| Module | Status | Notes |
|--------|--------|-------|
| Auth | ✅ Complete | JWT, bcrypt, rate limiting, OpenAPI, role-based auth |
| Tenant | ✅ Complete | Subdomain routing, PostgreSQL repo, TenantConfig |
| User | ✅ Complete | CRUD + roles + validation tests + admin auth |
| Cari | ✅ Complete | Customer/Vendor + PostgreSQL repo, Decimal |
| Product | ✅ Complete | Categories, units, variants, Decimal |
| Stock | ✅ Complete | Warehouses, movements, Decimal |
| Invoice | ✅ Complete | Payments, status, Decimal |
| Sales | ✅ Complete | Orders, quotations, Decimal |
| Purchase | ✅ Complete | Orders, goods receipt, purchase requests, Decimal |
| HR | ✅ Complete | Employees, attendance, leave, Decimal |
| Accounting | ✅ Complete | Journal entries, trial balance, Decimal |
| Assets | ✅ Complete | Fixed assets, depreciation, maintenance |
| Project | ✅ Complete | WBS, costs, profitability, Decimal |
| Manufacturing | ✅ Complete | Work orders, BOM, routing, NCR, Decimal |
| CRM | ✅ Complete | Leads, opportunities, tickets, Decimal |
| Feature Flags | ✅ Complete | CRUD, tenant-specific, API v1, admin auth |
| Product Variants | ✅ Complete | CRUD, API v1 |
| Purchase Requests | ✅ Complete | CRUD, approval workflow, API v1, state machine, pagination |
| Audit | ✅ Complete | Request audit trail, batch persistence, admin query API |

### Infrastructure & Operations
| Feature | Status | Notes |
|---------|--------|-------|
| Centralized Error Handling | ✅ Complete | `map_sqlx_error` with PG error codes (23505, 23503) |
| Trusted Proxy Config | ✅ Complete | `TURERP_TRUSTED_PROXIES` for rate limiting behind LBs |
| Composite DB Indexes | ✅ Complete | `tenant_id + created_at DESC` on all multi-tenant tables |
| Health Checks | ✅ Complete | `/health/live` (liveness), `/health/ready` (readiness + DB) |
| Prometheus Metrics | ✅ Complete | `http_requests_total`, `http_request_duration_seconds`, `/metrics` |
| Pagination | ✅ Complete | All 14 list endpoints return `PaginatedResult<T>` |
| Audit Log API | ✅ Complete | `GET /api/v1/audit-logs` with filtering + pagination |

### Test Coverage
- **314 tests passing** (249 unit + 38 integration + 27 security)
- Unit tests for all domain modules
- Model validation tests
- Service business logic tests
- Middleware tests
- Config validation tests
- Error handling tests
- Integration tests covering all business modules (38 tests)
- Security tests - OWASP Top 10 (27 tests)

### Code Quality
- ✅ cargo check passes
- ✅ cargo clippy passes (0 warnings)
- ✅ cargo fmt passes
- ✅ No ambiguous glob re-exports
- ✅ No unused imports
- ✅ No dead code
- ✅ Comprehensive error handling
- ✅ Request tracing

### Security Features
- ✅ JWT authentication with HS256
- ✅ Password hashing with bcrypt (cost 12)
- ✅ Password complexity validation
- ✅ Rate limiting (10 req/min per IP) with trusted proxy support
- ✅ Request ID tracking
- ✅ Audit logging middleware with batch persistence
- ✅ Production config validation
- ✅ SQL injection prevention (parameterized queries)
- ✅ Admin role authorization for sensitive operations (AdminUser extractor)
- ✅ Tenant isolation enforced at API layer
- ✅ Secure public path matching (exact match, no bypass)
- ✅ Encryption key memory security (zeroize on drop)
- ✅ Decimal precision for financial values (all modules)
- ✅ Required tenant_id in registration (no default tenant exposure)
- ✅ Thread-safe in-memory repositories (single mutex pattern)
- ✅ #[must_use] attributes on important return types
- ✅ Forbidden (403) error type for authorization failures
- ✅ AdminUser extractor role comparison (lowercase "admin" fix)
- ✅ Centralized DB error handling with PG error code detection
- ⚠️ Default admin credentials (dev only, warning in migrations)

---

## Remaining Work

### Medium Priority
| Feature | Description | Status |
|---------|-------------|--------|
| Performance Testing | Load testing with realistic data | Planned |
| API Response Caching | Cache frequently accessed data | Planned |

### Low Priority
| Feature | Description | Status |
|---------|-------------|--------|
| API Rate Limit Dashboard | Visual dashboard for rate limit metrics | Planned |
| Webhook System | Event-driven notifications | Planned |

---

## Dependencies (Cargo.toml)

```toml
# Web framework
actix-web = "4"
actix-rt = "2"
actix-cors = "0.7"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio-native-tls", "postgres", "macros", "chrono"] }

# Authentication
jsonwebtoken = "9"
bcrypt = "0.15"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Validation
validator = { version = "0.16", features = ["derive"] }

# OpenAPI / Swagger
utoipa = "4"
utoipa-swagger-ui = { version = "6", features = ["actix-web"] }

# Async
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }

# Error handling
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Rate limiting
governor = "0.6"

# UUID
uuid = { version = "1.0", features = ["v4"] }

# Configuration
config = "0.14"
dotenv = "0.15"

# Precise decimal for financial calculations
rust_decimal = { version = "1.36", features = ["serde"] }

# Secure memory zeroing
zeroize = "1.8"
```

---

## API Endpoints Summary

### Auth (v1)
- `POST /api/v1/auth/register` - Register new user
- `POST /api/v1/auth/login` - Login (rate limited: 10 req/min)
- `POST /api/v1/auth/refresh` - Refresh token
- `GET /api/v1/auth/me` - Current user (requires auth)

### Users (v1)
- `GET /api/v1/users` - List users (requires auth)
- `POST /api/v1/users` - Create user (requires auth)
- `GET /api/v1/users/{id}` - Get user (requires auth)
- `PUT /api/v1/users/{id}` - Update user (requires auth)
- `DELETE /api/v1/users/{id}` - Delete user (requires auth)

### Tenants
- `GET /api/tenants` - List tenants
- `POST /api/tenants` - Create tenant
- `GET /api/tenants/{id}` - Get tenant
- `PUT /api/tenants/{id}` - Update tenant
- `DELETE /api/tenants/{id}` - Delete tenant

### Cari
- `GET /api/cari` - List cari accounts
- `GET /api/cari/{id}` - Get cari account
- `GET /api/cari/code/{code}` - Get cari by code
- `POST /api/cari` - Create cari account
- `PUT /api/cari/{id}` - Update cari account
- `DELETE /api/cari/{id}` - Delete cari account
- `GET /api/cari/{id}/orders` - Get customer's sales orders

### Products (v1)
- `GET /api/v1/products/{product_id}/variants` - List variants (auth)
- `POST /api/v1/products/{product_id}/variants` - Create variant (auth)
- `GET /api/v1/variants/{id}` - Get variant (auth)
- `PUT /api/v1/variants/{id}` - Update variant (auth)
- `DELETE /api/v1/variants/{id}` - Delete variant (auth)

### Stock
- `GET /api/stock/warehouses` - List warehouses
- `POST /api/stock/warehouses` - Create warehouse
- `GET /api/stock/warehouses/{id}` - Get warehouse
- `PUT /api/stock/warehouses/{id}` - Update warehouse
- `DELETE /api/stock/warehouses/{id}` - Delete warehouse
- `GET /api/stock/movements` - List stock movements
- `POST /api/stock/movements` - Create stock movement
- `GET /api/stock/summary` - Stock summary

### Invoices
- `GET /api/invoices` - List invoices
- `POST /api/invoices` - Create invoice
- `GET /api/invoices/{id}` - Get invoice
- `PUT /api/invoices/{id}` - Update invoice
- `DELETE /api/invoices/{id}` - Delete invoice

### Sales
- `GET /api/sales` - List sales orders
- `POST /api/sales` - Create sales order
- `GET /api/sales/{id}` - Get sales order
- `PUT /api/sales/{id}` - Update sales order
- `DELETE /api/sales/{id}` - Delete sales order

### HR
- `GET /api/hr/employees` - List employees
- `POST /api/hr/employees` - Create employee
- `GET /api/hr/employees/{id}` - Get employee
- `PUT /api/hr/employees/{id}` - Update employee
- `DELETE /api/hr/employees/{id}` - Delete employee

### Accounting
- `GET /api/accounting/accounts` - List accounts
- `POST /api/accounting/accounts` - Create account
- `GET /api/accounting/accounts/tree` - Account tree
- `GET /api/accounting/journal` - List journal entries
- `POST /api/accounting/journal` - Create journal entry

### Assets
- `GET /api/assets` - List assets
- `POST /api/assets` - Create asset
- `GET /api/assets/{id}` - Get asset
- `PUT /api/assets/{id}` - Update asset
- `DELETE /api/assets/{id}` - Delete asset
- `GET /api/assets/{id}/depreciation` - Depreciation schedule

### Project
- `GET /api/projects` - List projects
- `POST /api/projects` - Create project
- `GET /api/projects/{id}` - Get project
- `PUT /api/projects/{id}` - Update project

### Manufacturing
- `GET /api/manufacturing/work-orders` - List work orders
- `POST /api/manufacturing/work-orders` - Create work order
- `GET /api/manufacturing/work-orders/{id}` - Get work order
- `PUT /api/manufacturing/work-orders/{id}` - Update work order

### CRM
- `GET /api/crm/leads` - List leads
- `POST /api/crm/leads` - Create lead
- `GET /api/crm/opportunities` - List opportunities
- `POST /api/crm/opportunities` - Create opportunity

### Feature Flags (v1)
- `GET /api/v1/feature-flags` - List flags (auth, tenant-isolated)
- `POST /api/v1/feature-flags` - Create flag (admin only)
- `GET /api/v1/feature-flags/{id}` - Get flag (auth)
- `PUT /api/v1/feature-flags/{id}` - Update flag (admin only)
- `DELETE /api/v1/feature-flags/{id}` - Delete flag (admin only)
- `POST /api/v1/feature-flags/{id}/enable` - Enable flag (admin only)
- `POST /api/v1/feature-flags/{id}/disable` - Disable flag (admin only)
- `GET /api/v1/feature-flags/check/{name}` - Check if enabled (auth)

### Purchase Requests (v1)
- `GET /api/v1/purchase-requests` - List requests (auth, tenant-isolated)
- `POST /api/v1/purchase-requests` - Create request (auth)
- `GET /api/v1/purchase-requests/{id}` - Get request (auth)
- `PUT /api/v1/purchase-requests/{id}` - Update request (auth)
- `DELETE /api/v1/purchase-requests/{id}` - Delete request (auth)
- `POST /api/v1/purchase-requests/{id}/submit` - Submit for approval (auth)
- `POST /api/v1/purchase-requests/{id}/approve` - Approve request (admin only)
- `POST /api/v1/purchase-requests/{id}/reject` - Reject request (admin only)

### Audit Logs (v1)
- `GET /api/v1/audit-logs` - List audit logs (admin only, with filtering and pagination)

### Health Check
- `GET /health` - Health check endpoint (alias for readiness)
- `GET /health/live` - Liveness probe (always 200)
- `GET /health/ready` - Readiness probe (checks DB, returns version + latency)

### Metrics
- `GET /metrics` - Prometheus metrics (http_requests_total, http_request_duration_seconds)

### Swagger UI
- `/swagger-ui/` - Interactive API documentation
- `/api-docs/openapi.json` - OpenAPI JSON spec

---

## Running the Project

```bash
# Development
cargo run

# Tests
cargo test

# Tests with output
cargo test -- --nocapture

# Build
cargo build --release

# Swagger UI
# Visit http://localhost:8080/swagger-ui/
```

---

## Docker Deployment

### Quick Start
```bash
cd turerp
docker-compose up -d --build
```

### Environment Variables
| Variable | Description | Default |
|----------|-------------|--------|
| TURERP_DATABASE_URL | PostgreSQL connection string | Required |
| TURERP_JWT_SECRET | Secret key for JWT tokens (min 32 chars in prod) | Required |
| TURERP_SERVER_HOST | Server host | 0.0.0.0 |
| TURERP_SERVER_PORT | Server port | 8000 |
| TURERP_ENV | Environment (development/production) | development |
| TURERP_CORS_ORIGINS | Comma-separated allowed origins | * |
| RUST_LOG | Log level | info |

---

## Project Structure

```
turerp/
├── Cargo.toml
├── src/
│   ├── main.rs           # Application entry point
│   ├── lib.rs            # Library exports
│   ├── config.rs         # Configuration management
│   ├── error.rs          # Error types
│   ├── api/
│   │   ├── mod.rs        # API module + OpenAPI
│   │   └── v1/           # API version 1
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
│   │       └── audit.rs
│   ├── middleware/
│   │   ├── mod.rs        # Middleware exports
│   │   ├── auth.rs       # JWT authentication
│   │   ├── rate_limit.rs # Rate limiting (with trusted proxy support)
│   │   ├── request_id.rs # Request ID tracking
│   │   ├── audit.rs      # Audit logging (channel-based batch persistence)
│   │   ├── metrics.rs    # Prometheus metrics collection
│   │   └── tenant.rs     # Tenant context middleware
│   ├── domain/
│   │   ├── auth/         # Auth domain
│   │   ├── user/         # User domain
│   │   ├── tenant/       # Tenant domain (includes TenantConfig)
│   │   ├── cari/         # Customer/Vendor domain
│   │   ├── product/      # Product domain (includes variants)
│   │   ├── stock/        # Stock domain
│   │   ├── invoice/      # Invoice domain
│   │   ├── sales/        # Sales domain
│   │   ├── purchase/     # Purchase domain (includes requests)
│   │   ├── hr/           # HR domain
│   │   ├── accounting/   # Accounting domain
│   │   ├── assets/       # Fixed assets domain
│   │   ├── project/      # Project domain
│   │   ├── manufacturing/# Manufacturing domain
│   │   ├── crm/          # CRM domain
│   │   ├── audit/        # Audit log domain
│   │   └── feature/      # Feature flags domain
│   ├── common/
│   │   └── pagination.rs # Pagination utilities (PaginatedResult, PaginationParams)
│   ├── db/
│   │   ├── mod.rs        # DB module
│   │   ├── pool.rs       # Connection pool
│   │   ├── error.rs      # Centralized DB error handling (map_sqlx_error)
│   │   └── tenant_registry.rs # Tenant pool registry
│   └── utils/
│       ├── jwt.rs        # JWT utilities
│       ├── password.rs   # Password utilities
│       └── encryption.rs # AES-256-GCM encryption
├── migrations/
│   ├── 001_initial_schema.sql
│   ├── 002_add_tenant_db_name.sql
│   ├── 003_business_modules.sql
│   ├── 004_composite_indexes.sql
│   └── 005_audit_logs.sql
├── tests/
│   ├── api_integration_test.rs   # Integration tests (38 tests)
│   └── security_test.rs          # Security tests (27 tests)
└── docker-compose.yml
```

---

## Phase 12: Infrastructure & Operations (Complete ✅)

### 12A: Centralized Error Handling & Trusted Proxy
- [x] Extract `map_sqlx_error` to `db/error.rs` with PG error code detection (23505, 23503)
- [x] Add `RateLimitConfig` with trusted_proxies, requests_per_minute, burst_size
- [x] Rewrite rate limiting to only trust `X-Forwarded-For` from configured proxies

### 12B: Database Indexes & Health Checks
- [x] Add `004_composite_indexes.sql` with `tenant_id + created_at DESC` indexes
- [x] Add `/health/live` (liveness) and `/health/ready` (readiness with DB check) endpoints
- [x] Add `/health` as backwards-compatible alias for readiness

### 12C: Prometheus Metrics
- [x] Add `metrics` and `metrics-exporter-prometheus` dependencies
- [x] Create `MetricsMiddleware` recording `http_requests_total`, `http_request_duration_seconds`
- [x] Add `/metrics` endpoint with configurable path and enabled flag

### 12D: Pagination for All List Endpoints
- [x] Add `PaginatedResult::map()` for type transformations
- [x] Switch all 14 list endpoints to accept `PaginationParams` and return `PaginatedResult`
- [x] PostgreSQL repos use `COUNT(*) OVER()` for efficient total count
- [x] In-memory repos implement skip/take pagination

### 12E: Audit Log Domain & API
- [x] Add `005_audit_logs.sql` with indexes for tenant+created_at, tenant+user_id, tenant+path
- [x] Create audit domain module (model, repository, service, postgres_repository)
- [x] Add `GET /api/v1/audit-logs` endpoint with filtering and pagination (admin-only)
- [x] Rewrite `AuditLoggingMiddleware` with mpsc channel for non-blocking batch persistence
- [x] Spawn background audit writer with 5s flush interval and 100-event buffer