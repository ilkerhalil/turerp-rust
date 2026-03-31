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
- [x] Unit tests (217 passing)
- [x] Integration tests (14 passing)
- [x] Security tests (8 passing)
- [x] Request ID middleware
- [x] JWT authentication middleware
- [x] Password complexity validation
- [x] Production config validation
- [x] Admin role authorization
- [x] Tenant isolation enforcement
- [x] Security audit (OWASP Top 10)
- [x] SQL injection tests
- [x] Input validation tests
- [x] JWT tampering tests
- [x] Authentication security tests
- [x] HTTP method security tests

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

## Current Status: Phase 9 - Complete ✅ (Security Hardened)

### Completed Modules
| Module | Status | Notes |
|--------|--------|-------|
| Auth | ✅ Complete | JWT, bcrypt, rate limiting, OpenAPI |
| Tenant | ✅ Complete | Subdomain routing, PostgreSQL repo, TenantConfig |
| User | ✅ Complete | CRUD + roles + validation tests |
| Cari | ✅ Complete | Customer/Vendor + PostgreSQL repo |
| Product | ✅ Complete | Categories, units, variants |
| Stock | ✅ Complete | Warehouses, movements |
| Invoice | ✅ Complete | Payments, status |
| Sales | ✅ Complete | Orders, quotations |
| Purchase | ✅ Complete | Orders, goods receipt, purchase requests |
| HR | ✅ Complete | Employees, attendance, leave |
| Accounting | ✅ Complete | Journal entries, trial balance |
| Assets | ✅ Complete | Fixed assets, depreciation, maintenance |
| Project | ✅ Complete | WBS, costs, profitability |
| Manufacturing | ✅ Complete | Work orders, BOM, routing, NCR |
| CRM | ✅ Complete | Leads, opportunities, tickets |
| Feature Flags | ✅ Complete | CRUD, tenant-specific, API v1, admin auth |
| Product Variants | ✅ Complete | CRUD, API v1 |
| Purchase Requests | ✅ Complete | CRUD, approval workflow, API v1, state machine, pagination |
| Tenant DB Routing | ✅ Complete | Multi-tenant isolation middleware |
| Tenant Config | ✅ Complete | Per-tenant settings, encrypted values |

### Test Coverage
- **239 tests passing** (217 unit + 14 integration + 8 security)
- Unit tests for all domain modules
- Model validation tests
- Service business logic tests
- Middleware tests
- Config validation tests
- Error handling tests
- Security tests (OWASP Top 10)

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
- ✅ Rate limiting (10 req/min per IP)
- ✅ Request ID tracking
- ✅ Production config validation
- ✅ SQL injection prevention (parameterized queries)
- ✅ Admin role authorization for sensitive operations
- ✅ Tenant isolation enforced at API layer
- ✅ Secure public path matching (exact match, no bypass)
- ✅ Encryption key memory security (zeroize on drop)
- ✅ Decimal precision for financial values (no floating-point errors)
- ✅ Required tenant_id in registration (no default tenant exposure)
- ✅ Thread-safe in-memory repositories (single mutex pattern)
- ⚠️ Default admin credentials (dev only, warning in migrations)

---

## Pending Implementation

### High Priority
| Feature | Description | Status |
|---------|-------------|--------|
| Tenant DB Routing | Multi-tenant database isolation | ✅ Complete |
| Security Audit | OWASP Top 10 review | ✅ Complete |
| API Versioning | /v1/, /v2/ endpoints | ✅ Complete |
| Feature Flags | A/B testing, gradual rollout | ✅ Complete |

### Medium Priority
| Feature | Description | Status |
|---------|-------------|--------|
| Product Variants | Size, color, etc. | ✅ Complete |
| Purchase Requests | Approval workflow, pagination | ✅ Complete |
| Fixed Assets | Depreciation tracking | ✅ Complete |
| Tenant-specific Config | Per-tenant settings | ✅ Complete |
| Encrypted Config | Sensitive value encryption | ✅ Complete |

### Low Priority
| Feature | Description | Status |
|---------|-------------|--------|
| API Analytics | Request metrics | Planned |
| Performance Testing | Load testing with realistic data | Planned |
| Monitoring | Prometheus/Grafana metrics | Planned |

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

### Product Variants (v1)
- `GET /api/v1/products/{product_id}/variants` - List variants (auth)
- `POST /api/v1/products/{product_id}/variants` - Create variant (auth)
- `GET /api/v1/variants/{id}` - Get variant (auth)
- `PUT /api/v1/variants/{id}` - Update variant (auth)
- `DELETE /api/v1/variants/{id}` - Delete variant (auth)

### Health Check
- `GET /health` - Health check endpoint

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
│   │       ├── feature_flags.rs
│   │       ├── product_variants.rs
│   │       └── purchase_requests.rs
│   ├── middleware/
│   │   ├── mod.rs        # Middleware exports
│   │   ├── auth.rs       # JWT authentication
│   │   ├── rate_limit.rs # Rate limiting
│   │   ├── request_id.rs # Request ID tracking
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
│   │   ├── assets/       # Fixed assets domain (NEW)
│   │   ├── project/      # Project domain
│   │   ├── manufacturing/# Manufacturing domain
│   │   ├── crm/          # CRM domain
│   │   └── feature/      # Feature flags domain
│   ├── common/
│   │   └── pagination.rs # Pagination utilities (NEW)
│   ├── db/
│   │   ├── mod.rs        # DB module
│   │   ├── pool.rs       # Connection pool
│   │   └── tenant_registry.rs # Tenant pool registry
│   └── utils/
│       ├── jwt.rs        # JWT utilities
│       ├── password.rs   # Password utilities
│       └── encryption.rs # AES-256-GCM encryption (NEW)
├── migrations/
│   └── 001_initial_schema.sql
├── tests/
│   ├── api_integration_test.rs
│   └── security_test.rs  # OWASP security tests (NEW)
│   ├── db/
│   │   ├── mod.rs        # DB module
│   │   ├── pool.rs       # Connection pool
│   │   └── tenant_registry.rs # Tenant pool registry
│   └── utils/
│       ├── jwt.rs        # JWT utilities
│       └── password.rs   # Password utilities
├── migrations/
│   └── 001_initial_schema.sql
├── tests/
│   └── api_integration_test.rs
└── docker-compose.yml
```

---

## Next Steps

1. ~~Product Variants~~ - Complete CRUD for product variants ✅
2. ~~Purchase Requests~~ - Implement approval workflow ✅
3. ~~Tenant Database Routing~~ - Implement per-tenant database isolation ✅
4. ~~Tenant-Specific Config~~ - Per-tenant settings ✅
5. ~~Feature Flags Admin Auth~~ - Security hardening ✅
6. ~~Security Audit~~ - Complete OWASP Top 10 review ✅
7. ~~Fixed Assets~~ - Depreciation tracking ✅
8. ~~Encrypted Config~~ - Sensitive value encryption ✅
9. **Performance Testing** - Load testing with realistic data
10. **Monitoring** - Add Prometheus/Grafana metrics