# Turerp ERP

[![CI](https://github.com/ilkerhalil/turerp-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/ilkerhalil/turerp-rust/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-314%20passing-brightgreen)]()
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Modern, multi-tenant SaaS ERP system** - Built with Rust, Actix-web, and SQLx.

## Features

### Core Modules
| Module | Description |
|--------|-------------|
| **Auth** | JWT-based authentication, bcrypt password hashing, token refresh |
| **Tenant** | Subdomain-based multi-tenant architecture, tenant isolation |
| **User** | User management, roles (Admin, User, Viewer) |
| **Cari** | Customer/Vendor accounts, credit limit management |
| **Product** | Product catalog, categories, units, barcode support |
| **Stock** | Warehouse management, stock movements, valuation |
| **Invoice** | Invoice creation, payment tracking, tax calculations |

### Business Modules
| Module | Description |
|--------|-------------|
| **Sales** | Sales orders, quotations, pricing |
| **Purchase** | Purchase orders, goods receipt, vendor management |
| **HR** | Employee management, payroll, leave tracking |
| **Accounting** | Chart of accounts, journal entries, trial balance |
| **Assets** | Fixed assets, depreciation calculation, maintenance tracking |

### Advanced Modules
| Module | Description |
|--------|-------------|
| **Projects** | Project management, WBS, project costs, profitability analysis |
| **Manufacturing** | Work orders, routing, production tracking |
| **BOM** | Bill of Materials management, material requirements planning |
| **Quality Control** | Quality inspections, non-conformance reports (NCR) |
| **CRM** | Leads, opportunities, campaigns, support tickets |
| **Audit** | Request audit trail, batch persistence, admin query API |

### Infrastructure & Operations
| Feature | Description |
|---------|-------------|
| **Prometheus Metrics** | `http_requests_total`, `http_request_duration_seconds`, `/metrics` endpoint |
| **Health Checks** | `/health/live` (liveness), `/health/ready` (readiness + DB check + version) |
| **Pagination** | All list endpoints return `PaginatedResult<T>` with page/per_page/total |
| **Centralized Error Handling** | `map_sqlx_error` with PG error code detection (23505, 23503) |
| **Trusted Proxy** | Configurable trusted proxies for rate limiting behind load balancers |
| **Composite DB Indexes** | `tenant_id + created_at DESC` on all multi-tenant tables |

## Tech Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| **Backend** | Rust | 1.70+ |
| **Web Framework** | Actix-web | 4.x |
| **Database** | PostgreSQL | 14+ |
| **ORM** | SQLx (runtime queries) | 0.8 |
| **Auth** | JWT + bcrypt | - |
| **Validation** | validator | 0.16 |
| **Rate Limiting** | governor | 0.8 |
| **Synchronization** | parking_lot (Mutex) | 0.12 |
| **Metrics** | metrics + metrics-exporter-prometheus | 0.24/0.16 |
| **API Docs** | utoipa (OpenAPI/Swagger) | 4.x |
| **Logging** | tracing | 0.1 |

## Quick Start

### Prerequisites
- Rust 1.70+
- PostgreSQL 14+
- (Optional) Docker & Docker Compose

### Running with Docker

```bash
cd turerp
docker-compose up -d
# API: http://localhost:8080
# Swagger UI: http://localhost:8080/swagger-ui/
```

### Development Setup

```bash
# Clone the repository
git clone https://github.com/ilkerhalil/turerp-rust.git
cd turerp-rust/turerp

# Build and run (in-memory storage - development)
cargo run

# Run with PostgreSQL (production)
export TURERP_DATABASE_URL="postgres://postgres:postgres@localhost:5432/turerp"
export TURERP_JWT_SECRET="your-secret-key-change-in-production"
cargo run --features postgres

# Run tests
cargo test

# PostgreSQL tests
cargo test --features postgres
```

### Storage Options

| Mode | Command | Use Case |
|------|---------|----------|
| **In-Memory** | `cargo run` | Development, testing |
| **PostgreSQL** | `cargo run --features postgres` | Production |

**Note**: In-memory mode stores all data in RAM. Data is lost on server restart. Use PostgreSQL for production.

### Pre-commit & Pre-push Hooks (Lefthook)

This project uses lefthook to prevent CI failures. Automatic checks run on every commit and push.

**Setup:**
```bash
# Install lefthook (one-time)
cargo install lefthook

# Activate git hooks
lefthook install
```

**Running Checks:**

| Hook | Commands | Description |
|------|----------|-------------|
| `pre-commit` | `cargo fmt --check` | Code format check |
| `pre-commit` | `cargo clippy -- -D warnings` | Lint errors |
| `pre-push` | `cargo test` | All tests |
| `pre-push` | `cargo audit` | Security audit |
| `commit-msg` | Conventional commits | Commit message format |

**Commit Message Format:**
```
type(scope): description

# Examples:
feat: add rate limiting middleware
fix: auth token validation bug
docs: update README
ci: add lefthook configuration
```

**Types:** feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert

## Project Structure

```
turerp-rust/
├── turerp/                    # Main application (Rust crate)
│   ├── src/
│   │   ├── api/               # HTTP handlers (Actix-web)
│   │   ├── config/           # Configuration
│   │   ├── db/                # Database layer (PostgreSQL)
│   │   │   ├── mod.rs
│   │   │   ├── pool.rs       # Connection pool, migrations
│   │   │   └── error.rs      # Centralized DB error handling
│   │   ├── domain/           # Domain modules (business logic)
│   │   │   ├── auth/          # Authentication
│   │   │   ├── user/         # User management
│   │   │   │   ├── mod.rs
│   │   │   │   ├── model.rs
│   │   │   │   ├── repository.rs      # InMemory impl
│   │   │   │   ├── postgres_repository.rs  # PostgreSQL impl
│   │   │   │   └── service.rs
│   │   │   ├── tenant/        # Tenant management
│   │   │   │   └── postgres_repository.rs
│   │   │   ├── cari/          # Cari accounts
│   │   │   │   └── postgres_repository.rs
│   │   │   ├── product/       # Product management
│   │   │   ├── stock/         # Stock management
│   │   │   ├── invoice/       # Invoice management
│   │   │   ├── sales/         # Sales module
│   │   │   ├── purchase/      # Purchase module
│   │   │   ├── hr/            # HR module
│   │   │   ├── accounting/    # Accounting module
│   │   │   ├── assets/        # Assets module
│   │   │   ├── project/       # Project management
│   │   │   ├── manufacturing/ # Manufacturing module
│   │   │   ├── audit/         # Audit log module
│   │   │   └── crm/           # CRM module
│   │   ├── common/
│   │   │   └── pagination.rs  # Pagination helpers
│   │   ├── middleware/       # HTTP middleware
│   │   │   ├── auth.rs        # JWT authentication
│   │   │   ├── rate_limit.rs  # Rate limiting (governor, trusted proxy)
│   │   │   ├── metrics.rs     # Prometheus metrics collection
│   │   │   ├── audit.rs       # Audit logging (channel-based batch persistence)
│   │   │   └── request_id.rs  # Request ID tracing
│   │   ├── utils/             # Utility functions
│   │   │   ├── jwt.rs         # JWT utilities
│   │   │   ├── password.rs    # Password utilities
│   │   │   └── encryption.rs  # AES-256-GCM encryption
│   │   ├── error/             # Error handling
│   │   ├── lib.rs             # Library entry point
│   │   └── main.rs            # Application entry point
│   ├── migrations/
│   │   ├── 001_initial_schema.sql    # Core schema
│   │   ├── 002_add_tenant_db_name.sql
│   │   ├── 003_business_modules.sql  # Business module tables
│   │   ├── 004_composite_indexes.sql # Pagination indexes
│   │   └── 005_audit_logs.sql        # Audit log table
│   ├── tests/                 # Integration tests
│   └── Cargo.toml             # Dependencies
├── docs/                      # Project documentation
│   └── modules/               # Module details
├── .github/                   # GitHub Actions CI/CD
├── AGENTS.md                  # AI agent configuration
├── IMPLEMENTATION_PLAN.md     # Implementation plan
└── lefthook.yml               # Pre-commit/pre-push hooks
```

## API Endpoints

### Authentication (Public - No JWT required)
```
POST /api/auth/register   - Register new user
POST /api/auth/login      - Login (returns JWT token)
POST /api/auth/refresh    - Token refresh
```

### Authentication (Protected - JWT required)
```
GET  /api/auth/me         - Current user info 🔒
```

### Users (Protected - JWT required)
```
GET    /api/users         - User list (paginated) 🔒
POST   /api/users         - Create user 🔒
GET    /api/users/{id}    - User details 🔒
PUT    /api/users/{id}    - Update user 🔒
DELETE /api/users/{id}    - Delete user 🔒
```

### Audit Logs (Admin only)
```
GET    /api/v1/audit-logs - List audit logs (paginated, filterable) 🔒👑
```

### Health & Monitoring
```
GET /health        - Health check (alias for readiness)
GET /health/live   - Liveness probe (always 200)
GET /health/ready  - Readiness probe (checks DB, version, latency)
GET /metrics       - Prometheus metrics
```

🔒 = JWT Bearer token required
👑 = Admin role required

### Swagger UI
- **Swagger UI**: `http://localhost:8000/swagger-ui/`
- **OpenAPI Spec**: `http://localhost:8000/api-docs/openapi.json`

**Note**: Click the "Authorize" button in Swagger UI to enter a Bearer token.

## Architecture

### Multi-Tenant Flow
```
Request → Subdomain Detection → Tenant Lookup → DB Routing → API Response
   ↓
JWT Token → User Authentication → Role-Based Access
```

### Module Dependencies
```
┌─────────────────────────────────────────────────────────────┐
│                    Authentication (Auth)                     │
├─────────────────────────────────────────────────────────────┤
│  Users  │  Tenants  │  Feature Flags  │  Configuration       │
├─────────┴───────────┴──────────────────┴──────────────────┤
│                      Core Modules                            │
│  Cari  │  Products  │  Stock  │  Invoices                  │
├─────────┴───────────┴─────────┴────────────────────────────┤
│                   Business Modules                           │
│  Sales  │  Purchase  │  HR  │  Accounting  │  Assets        │
├─────────┴───────────┴──────┴──────────────┴────────────────┤
│                    Security Layer                            │
│     OWASP Tests  │  Input Validation  │  Rate Limiting     │
├───────────┴─────────────────┴───────┴──────┴────────────────┤
│                   Extended Modules                           │
│  Projects  │  Manufacturing  │  BOM  │  QC  │  Shop Floor   │
├───────────┴─────────────────┴───────┴──────┴────────────────┤
│                         CRM                                  │
│     Leads  │  Opportunities  │  Campaigns  │  Tickets         │
└─────────────────────────────────────────────────────────────┘
```

## Testing

```bash
# All tests (314 tests)
cargo test

# Security tests
cargo test --test security_test

# Specific module tests
cargo test --lib domain::cari

# Test coverage
cargo tarpaulin --out Html
```

## CI/CD

Automated via GitHub Actions:
- Format check (`cargo fmt --check`)
- Clippy linting (`cargo clippy`)
- Test execution (`cargo test`)
- Security audit (`cargo audit`)

## Environment Variables

### Required Variables

| Variable | Description |
|----------|-------------|
| `TURERP_DATABASE_URL` | PostgreSQL connection string (e.g., `postgres://user:pass@host:5432/db`) |
| `TURERP_JWT_SECRET` | JWT signing key (min 32 characters in production, secure random string) |

### Optional Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TURERP_ENV` | Environment (`development` / `production`) | `development` |
| `TURERP_SERVER_HOST` | Server host | `0.0.0.0` |
| `TURERP_SERVER_PORT` | Server port | `8000` |
| `TURERP_DB_MAX_CONNECTIONS` | Max DB connections | `10` |
| `TURERP_DB_MIN_CONNECTIONS` | Min DB connections | `5` |
| `TURERP_JWT_ACCESS_EXPIRATION` | Access token duration (seconds) | `3600` (1 hour) |
| `TURERP_JWT_REFRESH_EXPIRATION` | Refresh token duration (seconds) | `604800` (7 days) |
| `TURERP_CORS_ORIGINS` | Allowed origins (comma-separated) | `*` |
| `TURERP_CORS_METHODS` | Allowed HTTP methods | `GET,POST,PUT,DELETE,OPTIONS` |
| `TURERP_CORS_HEADERS` | Allowed headers | `Content-Type,Authorization` |
| `TURERP_CORS_CREDENTIALS` | CORS credentials | `true` |
| `TURERP_TRUSTED_PROXIES` | Comma-separated trusted proxy IPs for rate limiting | (none) |
| `TURERP_RATE_LIMIT_RPM` | Rate limit requests per minute | `10` |
| `TURERP_RATE_LIMIT_BURST` | Rate limit burst size | `3` |
| `TURERP_METRICS_ENABLED` | Enable Prometheus metrics | `true` |
| `TURERP_METRICS_PATH` | Metrics endpoint path | `/metrics` |
| `RUST_LOG` | Log level | `info` |

## Security

### OWASP Top 10 Protection

The system has been tested against OWASP Top 10 vulnerabilities:
- **SQL Injection Prevention** - Parameterized queries
- **JWT Token Security** - Token validation and tampering protection
- **Authentication Security** - Strong password policies, rate limiting
- **Authorization** - Role-based access control (Admin, User, Viewer)
- **Input Validation** - All inputs are validated
- **HTTP Method Security** - Disallowed methods are rejected

### Security Hardening (Code Review)

Additional security measures:
- **Public Path Matching** - Exact match to prevent route bypass
- **Encryption Key Security** - Memory cleanup with `zeroize`
- **Financial Precision** - `Decimal` type for monetary calculations (all modules)
- **Role-Based Access** - AdminUser extractor for endpoint protection
- **Mutex Pattern** - Thread-safe repositories with single inner struct
- **Tenant Isolation** - Mandatory `tenant_id` to prevent tenant data exposure
- **Thread Safety** - Single mutex pattern to prevent race conditions
- **Must-Use Attributes** - `#[must_use]` for important return values
- **Audit Trail** - All authenticated requests are logged with batch persistence
- **Centralized DB Errors** - PostgreSQL error code detection (unique violation, foreign key)
- **Prometheus Metrics** - Request counters, latency histograms, in-flight gauges

### JWT Authentication

All API endpoints (except auth) require a JWT Bearer token:

```bash
# Get token
curl -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"password"}'

# Authenticated request
curl http://localhost:8000/api/users \
  -H "Authorization: Bearer <access_token>"
```

### Rate Limiting

Auth endpoints are protected with rate limiting:
- **Limit**: 10 requests/minute (configurable via `TURERP_RATE_LIMIT_RPM`)
- **Burst**: 3 requests (configurable via `TURERP_RATE_LIMIT_BURST`)
- **Trusted Proxies**: Configure `TURERP_TRUSTED_PROXIES` to trust `X-Forwarded-For` headers behind load balancers

### Password Requirements

Passwords must meet the following criteria:
- Minimum 12 characters
- At least 1 uppercase letter
- At least 1 lowercase letter
- At least 1 digit
- At least 1 special character

### Production Warnings

In production environment (`TURERP_ENV=production`):
- JWT secret must be at least 32 characters
- JWT secret must not contain weak patterns like "dev", "test", "password"
- CORS wildcard (`*`) usage is not recommended

### Tenant Isolation

Each tenant has its own database and is isolated via `tenant_id` from the JWT token. Users can only access their own tenant's data.

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: amazing feature'`)
4. Push the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Documentation

- [Module Documentation](docs/README.md) - Detailed description of all modules
- [Implementation Plan](IMPLEMENTATION_PLAN.md) - Development roadmap
- [Application README](turerp/README.md) - Main application documentation

## License

MIT License

```
Copyright (c) 2024 Turerp Team

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
```

---

**Turerp Team** - Built with Rust for modern ERP solutions.