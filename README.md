# Turerp ERP

[![CI](https://github.com/ilkerhalil/turerp-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/ilkerhalil/turerp-rust/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

**Modern, multi-tenant SaaS ERP system** - Built with Rust, Actix-web, SQLx, and OpenTelemetry.

## Features

### Core Modules

| Module | Description |
|--------|-------------|
| **Auth** | JWT-based authentication, bcrypt password hashing, token refresh |
| **MFA** | TOTP-based multi-factor authentication with backup codes |
| **Tenant** | Subdomain-based multi-tenant architecture, tenant isolation, tenant configs |
| **User** | User management, roles (Admin, User, Viewer), soft delete |
| **Company** | Multi-company support with `company_id` isolation, inter-company transfers |
| **Feature Flags** | Tenant-specific feature toggles, admin controls, enable/disable endpoints |
| **Settings** | Per-tenant configuration management, typed values, categories, bulk update |
| **Cari** | Customer/Vendor accounts, credit limit management, soft delete |
| **Product** | Product catalog, categories, units, variants, barcode support |
| **Stock** | Warehouse management, stock movements, valuation, levels |
| **Invoice** | Invoice creation, payment tracking, tax calculations, status workflow |

### Business Modules

| Module | Description |
|--------|-------------|
| **Sales** | Sales orders, quotations, pricing, conversion to orders, status workflow |
| **Purchase** | Purchase orders, goods receipt, purchase requests (approval workflow) |
| **HR** | Employee management, attendance, leave types/requests, payroll, SGK e-Bildirge |
| **Shift Planning** | Shift creation, assignments, clock-in/out, attendance, overtime calculation |
| **Accounting** | Chart of accounts, journal entries, trial balance, posting, voiding |
| **Assets** | Fixed assets, depreciation calculation, maintenance tracking, write-off/dispose |
| **Cost Center** | Cost centers, profit centers, allocation rules, profitability reporting |
| **Chart of Accounts** | Account tree, children, recalculation, trial balance |
| **Tax** | Turkish tax rate management (KDV, OIV, etc.), calculation, KVB period tracking |
| **Currency** | Multi-currency support, exchange rates, conversion, effective rates |

### Advanced Modules

| Module | Description |
|--------|-------------|
| **Projects** | Project management, WBS, project costs, profitability analysis |
| **Manufacturing** | Work orders, routing, BOM, material requirements, production tracking |
| **Quality Control** | Quality inspections, non-conformance reports (NCR) |
| **CRM** | Leads, opportunities, campaigns, support tickets, pipeline value |
| **Custom Fields** | Dynamic field definitions per module (String/Number/Date/Boolean/Select), JSONB values |
| **Webhook System** | Event-driven webhook subscriptions, delivery tracking, retry logic |
| **Workflow Engine** | Approval workflows with templates, instances, steps, approval/rejection/resubmit |
| **Bank Integration** | Bank accounts, statements, MT940/CAMT.053/XML parsers, auto-reconciliation |
| **Subscription Billing** | Subscription plans, recurring subscriptions, billing cycles, invoices |
| **Event Bus** | PostgreSQL outbox, Redis Streams, DLQ with retry, CDC triggers, background scheduler |
| **Notifications** | Email/SMS/InApp notifications with template engine, retry support |
| **Push Notifications** | FCM push token management, broadcast, per-user delivery |
| **Job Scheduler** | Background jobs with priority, retry, exponential backoff, cron scheduling |
| **Search** | Full-text search with fuzzy matching, reindexing, tenant-isolated |
| **Reports** | PDF/Excel/XML/CSV/JSON report generation with tenant isolation |
| **Idempotency** | Per-endpoint idempotency keys with 24h TTL cache |
| **API Keys** | Scoped API key authentication with HMAC hashing |
| **IP Whitelist** | Tenant-scoped IP access control with CIDR support |
| **File Upload** | S3/MinIO backend, presigned URLs, metadata tracking, version history, soft delete |
| **Document Management** | Document metadata, tags, attachment to any entity |
| **Import/Export** | CSV/JSON bulk import with validation, async processing, template downloads |
| **Archive** | Data archiving policies, scheduled jobs, record restoration |
| **LDAP/AD** | LDAP/Active Directory user synchronization and configuration |
| **Audit** | Request audit trail, mpsc batch persistence, log retrieval |
| **Rate Limits** | Rate limiting statistics and admin dashboard |
| **Security Headers** | HSTS, CSP, X-Frame-Options, X-Content-Type-Options |

### Turkish Compliance

| Module | Description |
|--------|-------------|
| **e-Fatura** | Turkish e-Fatura (electronic invoicing) integration with GIB |
| **e-Arsiv** | Turkish e-Arsiv Fatura and E-Serbest Meslek Makbuzu |
| **e-Defter** | Turkish e-Defter (electronic ledger) with XML generation, Saklayici integration |
| **e-Defter Blockchain** | Hash-chain compliance, Merkle tree, period verification |

### Portals

| Module | Description |
|--------|-------------|
| **Customer Portal** | Self-service portal: orders, invoices, payments, support tickets, PDF download |
| **Vendor Portal** | Self-service portal: purchase orders, invoices, payments, delivery notes, PDF download |

### Infrastructure & Operations

| Feature | Description |
|---------|-------------|
| **GraphQL** | Alternative query interface for complex data fetching |
| **i18n** | Per-request localization (`Accept-Language`: `en`, `tr`) |
| **Prometheus Metrics** | `http_requests_total`, `http_request_duration_seconds`, `/metrics` endpoint |
| **OpenTelemetry** | Aspire Dashboard integration via OTLP (traces, metrics, logs) |
| **Health Checks** | `/health/live` (liveness), `/health/ready` (readiness + DB check) |
| **Circuit Breaker** | Per-service circuit breaker with configurable thresholds, manual reset |
| **Retry** | Exponential backoff, jitter, per-policy configuration, statistics tracking |
| **Redis Cache** | Cache-aside pattern, TTL per namespace, invalidation on writes, hit/miss metrics |
| **CDC** | PostgreSQL LISTEN/NOTIFY triggers, CDC-to-DomainEvent bridge |
| **DB Router** | Read replica routing with session tracking and health checks |
| **Tracing** | OpenTelemetry-compatible distributed tracing with W3C propagation |
| **Pagination** | All list endpoints return `PaginatedResult<T>` with page/per_page/total |
| **Centralized Error Handling** | `map_sqlx_error` with PG error code detection (23505, 23503) |
| **Trusted Proxy** | Configurable trusted proxies for rate limiting behind load balancers |
| **Composite DB Indexes** | `tenant_id + created_at DESC` on all multi-tenant tables |
| **Soft Delete** | Universal soft delete with restore, destroy, and deleted-list endpoints |

## Tech Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| **Backend** | Rust | 1.70+ |
| **Web Framework** | Actix-web | 4.x |
| **Database** | PostgreSQL | 14+ |
| **ORM** | SQLx (runtime queries) | 0.8 |
| **Auth** | JWT + bcrypt | - |
| **MFA** | TOTP (totp-rs) | 5.x |
| **Validation** | validator | 0.20 |
| **Rate Limiting** | governor | 0.8 |
| **Synchronization** | parking_lot (Mutex) | 0.12 |
| **Metrics** | metrics + metrics-exporter-prometheus | 0.24/0.16 |
| **OpenTelemetry** | opentelemetry + opentelemetry-otlp | 0.28 |
| **API Docs** | utoipa (OpenAPI/Swagger) | 4.x |
| **GraphQL** | async-graphql | 7.x |
| **Logging** | tracing | 0.1 |
| **Redis** | redis | 0.27 |
| **S3 Storage** | rust-s3 | 0.35 |
| **Job Scheduler** | cron | 0.15 |
| **Email** | lettre | 0.11 |
| **Reports** | rust_xlsxwriter, printpdf, csv | - |
| **LDAP** | ldap3 | 0.11 |
| **Barcodes** | qrcode, barcodes, image | - |
| **XML** | quick-xml | 0.37 |

## Contributing

All changes must go through a **pull request**. Direct pushes to `main` are prohibited.

See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`AGENTS.md`](AGENTS.md) for the full workflow, commit conventions, code quality guidelines, and architecture details.

**Quick reference:**
```bash
git checkout -b feature/<issue-number>-<short-description>
# Make changes
git commit -m "feat(scope): description"
git push -u origin feature/<issue-number>-<short-description>
# Open PR on GitHub (must reference an open issue)
```

## Quick Start

### Prerequisites
- Rust 1.70+
- PostgreSQL 14+
- (Optional) Docker & Docker Compose
- (Optional) Redis
- (Optional) S3/MinIO

### Running with Docker

```bash
cd turerp
docker-compose up -d
# API: http://localhost:8000
# Swagger UI: http://localhost:8000/swagger-ui/
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

# Generate OpenAPI spec
cargo run --bin gen_openapi
```

### Storage Options

| Mode | Command | Use Case |
|------|---------|----------|
| **In-Memory** | `cargo run` | Development, testing |
| **PostgreSQL** | `cargo run --features postgres` | Production |

**Note**: In-memory mode stores all data in RAM. Data is lost on server restart. Use PostgreSQL for production.

### Pre-commit & Pre-push Hooks (Lefthook)

This project uses lefthook to prevent CI failures.

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
│   │   ├── main.rs            # Application entry point
│   │   ├── lib.rs             # Library exports, AppState
│   │   ├── config.rs          # Configuration management
│   │   ├── error.rs           # Error types (thiserror)
│   │   ├── api/               # HTTP handlers (Actix-web)
│   │   │   ├── mod.rs         # API module + OpenAPI spec (647 handlers)
│   │   │   ├── auth.rs        # Legacy auth routes (deprecated)
│   │   │   ├── users.rs       # Legacy users routes (deprecated)
│   │   │   └── v1/            # API version 1 (all production routes)
│   │   │       ├── mod.rs     # Module exports + route configuration
│   │   │       ├── auth.rs    # Login, register, refresh, me
│   │   │       ├── mfa.rs     # TOTP setup, verify, disable, backup codes
│   │   │       ├── users.rs   # User CRUD + soft delete
│   │   │       ├── cari.rs    # Customer/Vendor CRUD
│   │   │       ├── stock.rs   # Warehouses, movements, levels
│   │   │       ├── invoice.rs # Invoices, payments, status
│   │   │       ├── sales.rs   # Sales orders, quotations
│   │   │       ├── purchase_orders.rs      # Purchase orders
│   │   │       ├── purchase_requests.rs    # Purchase requests + approval
│   │   │       ├── goods_receipts.rs       # Goods receipts
│   │   │       ├── hr/         # HR submodule
│   │   │       │   ├── employees.rs
│   │   │       │   ├── attendance.rs
│   │   │       │   ├── leave.rs
│   │   │       │   ├── payroll.rs
│   │   │       │   └── sgk.rs  # e-Bildirge / SGK
│   │   │       ├── accounting.rs    # Accounts, journal entries
│   │   │       ├── assets.rs        # Fixed assets, depreciation
│   │   │       ├── project.rs       # Projects, WBS, costs
│   │   │       ├── manufacturing/   # Manufacturing submodule
│   │   │       │   ├── work_orders.rs
│   │   │       │   ├── boms.rs
│   │   │       │   ├── routings.rs
│   │   │       │   └── quality_control.rs
│   │   │       ├── crm/             # CRM submodule
│   │   │       │   ├── leads.rs
│   │   │       │   ├── opportunities.rs
│   │   │       │   ├── campaigns.rs
│   │   │       │   └── tickets.rs
│   │   │       ├── tenant.rs        # Tenant CRUD + config
│   │   │       ├── feature_flags.rs # Feature flag management
│   │   │       ├── product_variants/ # Products submodule
│   │   │       │   ├── products.rs
│   │   │       │   ├── categories.rs
│   │   │       │   ├── units.rs
│   │   │       │   └── variants.rs
│   │   │       ├── bank.rs          # Bank accounts, statements
│   │   │       ├── barcode.rs       # Barcode/QR generation
│   │   │       ├── api_keys.rs      # API key management
│   │   │       ├── ip_whitelist.rs  # IP access control
│   │   │       ├── archive.rs       # Archiving policies
│   │   │       ├── custom_fields.rs # Dynamic attributes
│   │   │       ├── currency.rs      # Currencies, exchange rates
│   │   │       ├── tax/             # Tax submodule
│   │   │       │   ├── rates.rs
│   │   │       │   └── periods.rs
│   │   │       ├── subscription.rs  # Subscription billing
│   │   │       ├── notifications.rs # In-app notifications
│   │   │       ├── push_notifications.rs # FCM push
│   │   │       ├── webhooks.rs      # Webhook management
│   │   │       ├── workflow.rs      # Approval workflows
│   │   │       ├── events.rs        # Event bus, outbox, DLQ
│   │   │       ├── search.rs        # Full-text search
│   │   │       ├── reports.rs       # Report generation
│   │   │       ├── documents.rs     # Document management
│   │   │       ├── files.rs         # File upload/download
│   │   │       ├── import.rs        # Bulk data import
│   │   │       ├── dashboard.rs     # KPIs and charts
│   │   │       ├── forecasting.rs   # Demand forecasting
│   │   │       ├── observability.rs # Health, SLI/SLO, alerts
│   │   │       ├── resilience.rs    # Circuit breakers, retry
│   │   │       ├── rate_limits.rs   # Rate limit statistics
│   │   │       ├── audit.rs         # Audit log retrieval
│   │   │       ├── settings.rs      # Configuration
│   │   │       ├── shifts.rs        # Shift planning
│   │   │       ├── chart_of_accounts.rs
│   │   │       ├── cost_centers.rs
│   │   │       ├── ldap.rs          # LDAP/AD sync
│   │   │       ├── efatura.rs       # e-Fatura integration
│   │   │       ├── earchive.rs      # e-Arsiv integration
│   │   │       ├── edefter.rs       # e-Defter integration
│   │   │       ├── edefter_blockchain.rs
│   │   │       ├── customer_portal.rs
│   │   │       ├── vendor_portal.rs
│   │   │       ├── graphql.rs       # GraphQL endpoint
│   │   │       └── companies.rs     # Company profiles
│   │   ├── middleware/          # HTTP middleware
│   │   │   ├── auth.rs          # JWT + extractors
│   │   │   ├── rate_limit.rs    # Rate limiting
│   │   │   ├── metrics.rs       # Prometheus metrics
│   │   │   ├── audit.rs         # Audit logging
│   │   │   ├── tenant.rs        # Tenant context
│   │   │   ├── request_id.rs    # Request ID tracing
│   │   │   ├── api_key.rs       # API key validation
│   │   │   ├── ip_whitelist.rs  # IP whitelist validation
│   │   │   ├── idempotency.rs   # Idempotency keys
│   │   │   └── security_headers.rs
│   │   ├── domain/              # Domain layer (DDD)
│   │   │   ├── auth/
│   │   │   ├── user/
│   │   │   ├── tenant/
│   │   │   ├── cari/
│   │   │   ├── product/
│   │   │   ├── stock/
│   │   │   ├── invoice/
│   │   │   ├── sales/
│   │   │   ├── purchase/
│   │   │   ├── accounting/
│   │   │   ├── assets/
│   │   │   ├── project/
│   │   │   ├── manufacturing/
│   │   │   ├── crm/
│   │   │   ├── hr/              # + sgk submodule
│   │   │   ├── shift/
│   │   │   ├── bank/
│   │   │   ├── barcode/
│   │   │   ├── api_key/
│   │   │   ├── ip_whitelist/
│   │   │   ├── archive/
│   │   │   ├── custom_field/
│   │   │   ├── currency/
│   │   │   ├── tax/
│   │   │   ├── subscription/
│   │   │   ├── notification/    # + push submodule
│   │   │   ├── webhook/
│   │   │   ├── workflow/
│   │   │   ├── event/
│   │   │   ├── search/
│   │   │   ├── report/
│   │   │   ├── document/
│   │   │   ├── file/
│   │   │   ├── import/
│   │   │   ├── dashboard/
│   │   │   ├── forecasting/
│   │   │   ├── observability/
│   │   │   ├── resilience/
│   │   │   ├── mfa/
│   │   │   ├── ldap/
│   │   │   ├── efatura/
│   │   │   ├── earchive/
│   │   │   ├── edefter/         # + blockchain, gib submodules
│   │   │   ├── customer_portal/
│   │   │   ├── vendor_portal/
│   │   │   ├── company/
│   │   │   ├── cost_center/
│   │   │   ├── chart_of_accounts/
│   │   │   └── job/
│   │   ├── common/              # Common utilities
│   │   │   ├── pagination.rs
│   │   │   ├── circuit_breaker.rs
│   │   │   ├── retry.rs
│   │   │   ├── import/
│   │   │   └── reports/
│   │   ├── db/                  # Database layer
│   │   │   ├── pool.rs          # Connection pool, migrations
│   │   │   └── error.rs         # DB error handling
│   │   ├── graphql/             # GraphQL schema
│   │   ├── i18n/                # Internationalization
│   │   ├── cache/               # Redis caching
│   │   ├── utils/               # Utilities
│   │   │   ├── jwt.rs
│   │   │   ├── password.rs
│   │   │   └── encryption.rs
│   │   └── bin/
│   │       └── gen_openapi.rs   # OpenAPI JSON generator
│   ├── migrations/              # 27 SQL migration files
│   ├── tests/                   # 26 integration test files
│   ├── Cargo.toml
│   └── openapi.json             # Generated OpenAPI 3.0.3 spec
├── docs/                        # Project documentation
├── .github/                     # GitHub Actions CI/CD
├── AGENTS.md                    # AI agent configuration
├── CLAUDE.md                    # Claude Code configuration
├── CONTRIBUTING.md              # Contribution guidelines
├── lefthook.yml                 # Pre-commit/pre-push hooks
└── README.md                    # This file
```

## API Endpoints

### Authentication

```
POST /api/v1/auth/register                    - Register new user
POST /api/v1/auth/login                       - Login (returns JWT)
POST /api/v1/auth/refresh                     - Token refresh
GET  /api/v1/auth/me                          - Current user info (JWT)
```

### MFA

```
POST /api/v1/auth/mfa/setup                   - Setup TOTP
POST /api/v1/auth/mfa/verify-setup            - Verify TOTP setup
POST /api/v1/auth/mfa/verify                - Verify TOTP code
POST /api/v1/auth/mfa/disable               - Disable MFA
GET  /api/v1/auth/mfa/status                - MFA status
POST /api/v1/auth/mfa/regenerate-backup-codes - Regenerate backup codes
```

### Users

```
GET    /api/v1/users          - List users (paginated)
POST   /api/v1/users          - Create user (admin)
GET    /api/v1/users/{id}     - User details
PUT    /api/v1/users/{id}     - Update user
DELETE /api/v1/users/{id}     - Soft delete user
POST   /api/v1/users/{id}/restore   - Restore user
POST   /api/v1/users/{id}/destroy   - Hard delete user
GET    /api/v1/users/deleted        - List deleted users
```

### Core Business

```
GET    /api/v1/cari                    - List cari accounts
POST   /api/v1/cari                    - Create cari
GET    /api/v1/cari/{id}               - Cari details
PUT    /api/v1/cari/{id}               - Update cari
DELETE /api/v1/cari/{id}               - Soft delete cari
GET    /api/v1/cari/type/{type}        - Filter by type
GET    /api/v1/cari/search             - Search cari

GET    /api/v1/products                - List products
POST   /api/v1/products                - Create product
GET    /api/v1/products/{id}            - Product details
PUT    /api/v1/products/{id}            - Update product
DELETE /api/v1/products/{id}            - Soft delete product

GET    /api/v1/stock/warehouses         - List warehouses
POST   /api/v1/stock/warehouses         - Create warehouse
GET    /api/v1/stock/movements          - List stock movements
POST   /api/v1/stock/movements          - Create movement
GET    /api/v1/stock/summary/{id}       - Stock summary

GET    /api/v1/invoices                - List invoices
POST   /api/v1/invoices                - Create invoice
GET    /api/v1/invoices/{id}           - Invoice details
PUT    /api/v1/invoices/{id}/status    - Update status
DELETE /api/v1/invoices/{id}           - Soft delete invoice
GET    /api/v1/invoices/{id}/payments   - List payments
POST   /api/v1/invoices/{id}/payments   - Create payment
```

### Sales & Purchase

```
GET    /api/v1/sales/orders             - List sales orders
POST   /api/v1/sales/orders             - Create order
GET    /api/v1/sales/orders/{id}        - Order details
PUT    /api/v1/sales/orders/{id}/status - Update status
GET    /api/v1/sales/quotations         - List quotations
POST   /api/v1/sales/quotations         - Create quotation
POST   /api/v1/sales/quotations/{id}/convert - Convert to order

GET    /api/v1/purchase-orders          - List purchase orders
POST   /api/v1/purchase-orders          - Create order
GET    /api/v1/purchase-requests        - List purchase requests
POST   /api/v1/purchase-requests        - Create request
POST   /api/v1/purchase-requests/{id}/submit   - Submit request
POST   /api/v1/purchase-requests/{id}/approve  - Approve request
POST   /api/v1/purchase-requests/{id}/reject   - Reject request
```

### HR

```
GET    /api/v1/hr/employees             - List employees
POST   /api/v1/hr/employees             - Create employee
GET    /api/v1/hr/employees/{id}        - Employee details
PUT    /api/v1/hr/employees/{id}/status - Update status
POST   /api/v1/hr/employees/{id}/terminate - Terminate employee

GET    /api/v1/hr/attendance            - List attendance
POST   /api/v1/hr/attendance            - Record attendance
GET    /api/v1/hr/leave-requests        - List leave requests
POST   /api/v1/hr/leave-requests        - Create leave request
POST   /api/v1/hr/leave-requests/{id}/approve - Approve leave
POST   /api/v1/hr/leave-requests/{id}/reject    - Reject leave

GET    /api/v1/hr/payroll               - List payroll records
POST   /api/v1/hr/payroll/calculate     - Calculate payroll
POST   /api/v1/hr/payroll/{id}/paid     - Mark as paid
```

### Accounting & Finance

```
GET    /api/v1/accounting/accounts      - List accounts
POST   /api/v1/accounting/accounts      - Create account
GET    /api/v1/accounting/accounts/{id} - Account details
POST   /api/v1/accounting/journal-entries      - Create journal entry
GET    /api/v1/accounting/journal-entries      - List entries
POST   /api/v1/accounting/journal-entries/{id}/post   - Post entry
POST   /api/v1/accounting/journal-entries/{id}/void   - Void entry
GET    /api/v1/accounting/trial-balance          - Generate trial balance

GET    /api/v1/chart-of-accounts         - List chart accounts
GET    /api/v1/chart-of-accounts/tree    - Account tree
GET    /api/v1/chart-of-accounts/trial-balance - Trial balance

GET    /api/v1/assets                    - List assets
POST   /api/v1/assets                    - Create asset
GET    /api/v1/assets/{id}               - Asset details
POST   /api/v1/assets/{id}/depreciation  - Calculate depreciation
POST   /api/v1/assets/{id}/depreciation/record - Record depreciation
POST   /api/v1/assets/{id}/dispose       - Dispose asset
POST   /api/v1/assets/{id}/write-off     - Write off asset
POST   /api/v1/assets/{id}/maintenance/start - Start maintenance
POST   /api/v1/assets/{id}/maintenance/end   - End maintenance

GET    /api/v1/cost-centers              - List cost centers
POST   /api/v1/cost-centers              - Create cost center
GET    /api/v1/cost-centers/{id}         - Cost center details
GET    /api/v1/cost-centers/{id}/allocations - List allocations
GET    /api/v1/cost-centers/{id}/profitability - Profitability report
```

### Tax & Currency

```
GET    /api/v1/tax/rates                 - List tax rates
POST   /api/v1/tax/rates                 - Create tax rate
GET    /api/v1/tax/rates/effective       - Get effective rate
POST   /api/v1/tax/calculate             - Calculate tax
POST   /api/v1/tax/calculate-invoice     - Calculate invoice tax
GET    /api/v1/tax/periods               - List tax periods
POST   /api/v1/tax/periods               - Create tax period
POST   /api/v1/tax/periods/{id}/calculate - Calculate period tax
POST   /api/v1/tax/periods/{id}/file     - File tax period

GET    /api/v1/currencies                - List currencies
POST   /api/v1/currencies                - Create currency
GET    /api/v1/currencies/{code}         - Currency details
POST   /api/v1/exchange-rates            - Create exchange rate
GET    /api/v1/exchange-rates            - List rates
POST   /api/v1/exchange-rates/convert    - Convert amount
GET    /api/v1/exchange-rates/effective  - Get effective rate
```

### Manufacturing

```
GET    /api/v1/manufacturing/work-orders          - List work orders
POST   /api/v1/manufacturing/work-orders          - Create work order
GET    /api/v1/manufacturing/work-orders/{id}     - Work order details
PUT    /api/v1/manufacturing/work-orders/{id}/status - Update status
GET    /api/v1/manufacturing/work-orders/{id}/materials - List materials
GET    /api/v1/manufacturing/work-orders/{id}/operations  - List operations

GET    /api/v1/manufacturing/boms               - List BOMs
POST   /api/v1/manufacturing/boms               - Create BOM
GET    /api/v1/manufacturing/boms/{id}          - BOM details
GET    /api/v1/manufacturing/boms/{id}/lines     - BOM lines
GET    /api/v1/manufacturing/boms/product/{id}   - BOMs by product

GET    /api/v1/manufacturing/routings           - List routings
POST   /api/v1/manufacturing/routings           - Create routing
GET    /api/v1/manufacturing/routings/{id}      - Routing details
GET    /api/v1/manufacturing/routings/{id}/operations - Operations
GET    /api/v1/manufacturing/material-requirements/{id} - Material requirements

GET    /api/v1/manufacturing/inspections        - List inspections
POST   /api/v1/manufacturing/inspections        - Create inspection
GET    /api/v1/manufacturing/inspections/{id}   - Inspection details
GET    /api/v1/manufacturing/ncrs               - List NCRs
POST   /api/v1/manufacturing/ncrs               - Create NCR
GET    /api/v1/manufacturing/ncrs/{id}         - NCR details
```

### CRM

```
GET    /api/v1/crm/leads                 - List leads
POST   /api/v1/crm/leads                 - Create lead
GET    /api/v1/crm/leads/{id}            - Lead details
PUT    /api/v1/crm/leads/{id}/status     - Update status
POST   /api/v1/crm/leads/{id}/convert    - Convert lead

GET    /api/v1/crm/opportunities         - List opportunities
POST   /api/v1/crm/opportunities         - Create opportunity
GET    /api/v1/crm/opportunities/{id}    - Opportunity details
PUT    /api/v1/crm/opportunities/{id}/status - Update status
GET    /api/v1/crm/pipeline-value        - Pipeline value

GET    /api/v1/crm/campaigns             - List campaigns
POST   /api/v1/crm/campaigns             - Create campaign
GET    /api/v1/crm/campaigns/{id}        - Campaign details
PUT    /api/v1/crm/campaigns/{id}/status - Update status

GET    /api/v1/crm/tickets               - List tickets
POST   /api/v1/crm/tickets               - Create ticket
GET    /api/v1/crm/tickets/{id}          - Ticket details
PUT    /api/v1/crm/tickets/{id}/status   - Update status
POST   /api/v1/crm/tickets/{id}/resolve  - Resolve ticket
GET    /api/v1/crm/tickets/open-count    - Open tickets count
```

### Turkish Compliance

```
GET    /api/v1/efatura                   - List e-Fatura
POST   /api/v1/efatura                   - Create e-Fatura
GET    /api/v1/efatura/{id}              - e-Fatura details
GET    /api/v1/efatura/{id}/xml          - Get XML
POST   /api/v1/efatura/{id}/send         - Send to GIB
POST   /api/v1/efatura/{id}/cancel       - Cancel e-Fatura
GET    /api/v1/efatura/status/{uuid}     - Check status

GET    /api/v1/earchive                  - List e-Archive
POST   /api/v1/earchive/generate         - Generate e-Archive
GET    /api/v1/earchive/{id}             - e-Archive details
POST   /api/v1/earchive/{id}/sign        - Sign
POST   /api/v1/earchive/{id}/send        - Send
POST   /api/v1/earchive/{id}/cancel      - Cancel

GET    /api/v1/edefter/periods           - List e-Defter periods
POST   /api/v1/edefter/periods           - Create period
GET    /api/v1/edefter/periods/{id}      - Period details
POST   /api/v1/edefter/periods/{id}/populate  - Populate period
POST   /api/v1/edefter/periods/{id}/validate  - Validate period
POST   /api/v1/edefter/periods/{id}/send      - Send to Saklayici
POST   /api/v1/edefter/periods/{id}/sign      - Sign berat
GET    /api/v1/edefter/periods/{id}/yevmiye-xml     - Yevmiye XML
GET    /api/v1/edefter/periods/{id}/buyuk-defter-xml - Buyuk Defter XML
GET    /api/v1/edefter/periods/{id}/hash-chain     - Hash chain
GET    /api/v1/edefter/periods/{id}/merkle-tree    - Merkle tree
POST   /api/v1/edefter/periods/{id}/verify         - Verify period
GET    /api/v1/edefter/periods/{id}/hash-state       - Hash state
```

### Portals

```
POST   /api/v1/customer-portal/register        - Register customer
POST   /api/v1/customer-portal/login           - Login customer
GET    /api/v1/customer-portal/orders          - Customer orders
GET    /api/v1/customer-portal/invoices         - Customer invoices
GET    /api/v1/customer-portal/invoices/{id}/pdf - Invoice PDF
GET    /api/v1/customer-portal/payments        - Customer payments
POST   /api/v1/customer-portal/support-tickets  - Create support ticket

POST   /api/v1/vendor-portal/register          - Register vendor
POST   /api/v1/vendor-portal/login             - Login vendor
GET    /api/v1/vendor-portal/orders            - Vendor orders
GET    /api/v1/vendor-portal/invoices          - Vendor invoices
GET    /api/v1/vendor-portal/invoices/{id}/pdf  - Invoice PDF
GET    /api/v1/vendor-portal/payments          - Vendor payments
POST   /api/v1/vendor-portal/delivery-notes    - Create delivery note
```

### Infrastructure

```
GET    /api/v1/observability/health             - Health summary
GET    /api/v1/observability/health/live        - Liveness probe
GET    /api/v1/observability/health/ready        - Readiness probe
GET    /api/v1/observability/dashboard           - Dashboard summary
GET    /api/v1/observability/dashboard-url       - Aspire Dashboard URL
GET    /api/v1/observability/slos                - List SLOs
POST   /api/v1/observability/slos                - Create SLO
GET    /api/v1/observability/slos/compliance     - SLO compliance
POST   /api/v1/observability/slos/compliance/evaluate - Evaluate compliance
GET    /api/v1/observability/alert-rules         - List alert rules
POST   /api/v1/observability/alert-rules         - Create alert rule
DELETE /api/v1/observability/alert-rules/{id}    - Delete alert rule
GET    /api/v1/observability/alerts              - List alerts
POST   /api/v1/observability/alerts/evaluate     - Evaluate alerts
POST   /api/v1/observability/alerts/{id}/resolve - Resolve alert
GET    /api/v1/observability/sparklines/{metric} - Sparkline data

GET    /api/v1/resilience/circuit-breakers              - List circuit breakers
POST   /api/v1/resilience/circuit-breakers/{service}/reset - Reset circuit breaker
GET    /api/v1/resilience/retry-stats                   - Retry statistics

GET    /api/v1/events/outbox/pending         - Pending outbox events
POST   /api/v1/events/outbox/process         - Process outbox
POST   /api/v1/events/outbox/retry/{id}      - Retry outbox event
GET    /api/v1/events/dead-letters           - Dead letter queue
POST   /api/v1/events/dead-letters/{id}/retry - Retry dead letter
GET    /api/v1/events/dlq                    - DLQ list
POST   /api/v1/events/dlq/retry/{id}         - Retry DLQ item
POST   /api/v1/events/publish                - Publish event
GET    /api/v1/events/cdc/status             - CDC status

GET    /api/v1/search                        - Search across entities
POST   /api/v1/search/index                  - Index document
POST   /api/v1/search/reindex                - Reindex all
DELETE /api/v1/search/{entity_type}/{id}     - Remove from index

POST   /api/v1/reports/generate              - Generate report
GET    /api/v1/reports                       - List reports
GET    /api/v1/reports/{id}/download         - Download report
DELETE /api/v1/reports/{id}                  - Delete report

GET    /api/v1/notifications/in-app          - In-app notifications
GET    /api/v1/notifications/unread-count    - Unread count
POST   /api/v1/notifications/{id}/read       - Mark as read
POST   /api/v1/notifications/read-all        - Mark all as read
POST   /api/v1/notifications/send            - Send notification
POST   /api/v1/notifications/{id}/retry      - Retry notification

POST   /api/v1/notifications/push/register     - Register push token
POST   /api/v1/notifications/push/unregister  - Unregister token
POST   /api/v1/notifications/push/send         - Send push
POST   /api/v1/notifications/push/broadcast    - Broadcast push
GET    /api/v1/notifications/push/tokens       - List tokens
```

### Settings & Admin

```
GET    /api/v1/settings                    - List settings
POST   /api/v1/settings                    - Create setting
GET    /api/v1/settings/{key}              - Get by key
PUT    /api/v1/settings/{id}               - Update setting
POST   /api/v1/settings/bulk               - Bulk update
DELETE /api/v1/settings/{id}               - Soft delete setting
POST   /api/v1/settings/seed               - Seed defaults

GET    /api/v1/feature-flags               - List feature flags
POST   /api/v1/feature-flags               - Create flag
GET    /api/v1/feature-flags/{id}          - Flag details
PUT    /api/v1/feature-flags/{id}          - Update flag
POST   /api/v1/feature-flags/{id}/enable   - Enable flag
POST   /api/v1/feature-flags/{id}/disable  - Disable flag
GET    /api/v1/feature-flags/check/{name}  - Check flag status

GET    /api/v1/api-keys                    - List API keys
POST   /api/v1/api-keys                    - Create API key
GET    /api/v1/api-keys/{id}               - API key details
DELETE /api/v1/api-keys/{id}               - Delete API key
GET    /api/v1/api-keys/check-scope/{scope} - Check scope

GET    /api/v1/ip-whitelist                - List whitelist entries
POST   /api/v1/ip-whitelist                - Add entry
GET    /api/v1/ip-whitelist/{id}           - Entry details
PUT    /api/v1/ip-whitelist/{id}           - Update entry
DELETE /api/v1/ip-whitelist/{id}           - Remove entry

GET    /api/v1/audit-logs                  - List audit logs

GET    /api/v1/admin/rate-limits           - Rate limit statistics

GET    /api/v1/tenant-configs               - List tenant configs
GET    /api/v1/tenant-configs/{id_or_key}   - Config details
```

### Health & Monitoring (Unauthenticated)

```
GET /health        - Health check
GET /health/live   - Liveness probe
GET /health/ready  - Readiness probe
GET /metrics       - Prometheus metrics
```

### Swagger UI
- **Swagger UI**: `http://localhost:8000/swagger-ui/`
- **OpenAPI Spec**: `http://localhost:8000/api-docs/openapi.json`
- **OpenAPI File**: `turerp/openapi.json`

**Note**: Click the "Authorize" button in Swagger UI to enter a Bearer token. All 647 handlers are documented in the OpenAPI schema with request/response schemas and security definitions.

## Architecture

### Multi-Tenant Flow
```
Request → Subdomain Detection → Tenant Lookup → DB Routing → API Response
   ↓
JWT Token → User Authentication → Role-Based Access → Company Isolation
   ↓
OpenTelemetry → Aspire Dashboard (Traces, Metrics, Logs)
```

### Module Dependencies
```
┌─────────────────────────────────────────────────────────────┐
│                    Authentication (Auth + MFA)              │
├─────────────────────────────────────────────────────────────┤
│  Users  │  Tenants  │  Feature Flags  │  Settings  │  Company│
├─────────┴───────────┴──────────────────┴─────────┴──────────┤
│                      Core Modules                            │
│  Cari  │  Products  │  Stock  │  Invoices  │  Currency      │
├─────────┴───────────┴─────────┴────────────┴────────────────┤
│                   Business Modules                           │
│  Sales  │  Purchase  │  HR  │  Accounting  │  Assets        │
├─────────┴───────────┴──────┴──────────────┴──────────────────┤
│                    Turkish Compliance                        │
│     e-Fatura  │  e-Arsiv  │  e-Defter  │  e-Defter Blockchain│
├───────────┴─────────────────┴───────┴──────┴─────────────────┤
│                   Extended Modules                           │
│  Projects  │  Manufacturing  │  BOM  │  QC  │  Shop Floor│
├───────────┴─────────────────┴───────┴──────┴─────────────────┤
│                         CRM + Portals                        │
│  Leads  │  Opportunities  │  Campaigns  │  Tickets           │
│  Customer Portal  │  Vendor Portal                         │
├───────────┴─────────────────┴───────┴──────┴─────────────────┤
│                    Infrastructure                            │
│  Event Bus  │  Search  │  Reports  │  Notifications  │  Jobs  │
│  Webhooks  │  Workflows  │  Audit  │  Import/Export          │
├───────────┴─────────────────┴───────┴──────┴─────────────────┤
│                    Observability + Resilience                │
│  OpenTelemetry  │  Prometheus  │  Health  │  Circuit Breaker│
│  Retry  │  Redis Cache  │  CDC  │  Rate Limiting             │
└─────────────────────────────────────────────────────────────┘
```

## Testing

```bash
# All tests
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
- Clippy linting (`cargo clippy -- -D warnings`)
- Test execution (`cargo test`)
- Security audit (`cargo audit`)

## Environment Variables

### Required Variables

| Variable | Description |
|----------|-------------|
| `TURERP_DATABASE_URL` | PostgreSQL connection string |
| `TURERP_JWT_SECRET` | JWT signing key (min 32 characters in production) |

### Optional Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `TURERP_ENV` | Environment (`development` / `production`) | `development` |
| `TURERP_SERVER_HOST` | Server host | `0.0.0.0` |
| `TURERP_SERVER_PORT` | Server port | `8000` |
| `TURERP_DB_MAX_CONNECTIONS` | Max DB connections | `10` |
| `TURERP_DB_MIN_CONNECTIONS` | Min DB connections | `5` |
| `TURERP_JWT_ACCESS_EXPIRATION` | Access token duration (seconds) | `3600` |
| `TURERP_JWT_REFRESH_EXPIRATION` | Refresh token duration (seconds) | `604800` |
| `TURERP_CORS_ORIGINS` | Allowed origins (comma-separated) | `*` |
| `TURERP_CORS_METHODS` | Allowed HTTP methods | `GET,POST,PUT,DELETE,OPTIONS` |
| `TURERP_CORS_HEADERS` | Allowed headers | `Content-Type,Authorization` |
| `TURERP_CORS_CREDENTIALS` | CORS credentials | `true` |
| `TURERP_TRUSTED_PROXIES` | Trusted proxy IPs for rate limiting | (none) |
| `TURERP_RATE_LIMIT_REQUESTS_PER_MINUTE` | Rate limit RPM | `10` |
| `TURERP_RATE_LIMIT_BURST` | Rate limit burst | `3` |
| `TURERP_METRICS_ENABLED` | Enable Prometheus metrics | `true` |
| `TURERP_METRICS_PATH` | Metrics endpoint | `/metrics` |
| `RUST_LOG` | Log level | `info` |

### Redis

| Variable | Description | Default |
|----------|-------------|---------|
| `TURERP_REDIS_ENABLED` | Enable Redis caching | `false` |
| `TURERP_REDIS_URL` | Redis connection string | `redis://127.0.0.1:6379` |
| `TURERP_REDIS_TTL` | Default cache TTL (seconds) | `300` |

### S3 / File Storage

| Variable | Description |
|----------|-------------|
| `S3_ENDPOINT` | S3/MinIO endpoint |
| `S3_BUCKET` | S3 bucket name |
| `S3_ACCESS_KEY` | S3 access key |
| `S3_SECRET_KEY` | S3 secret key |
| `S3_REGION` | S3 region | `us-east-1` |

### OpenTelemetry / Aspire Dashboard

| Variable | Description | Default |
|----------|-------------|---------|
| `TURERP_OTEL_ENABLED` | Enable OpenTelemetry | `true` |
| `TURERP_OTEL_ENDPOINT` | OTLP endpoint | `http://localhost:4317` |
| `TURERP_OTEL_SERVICE_NAME` | Service name | `turerp` |

### SMTP / Email

| Variable | Description |
|----------|-------------|
| `TURERP_SMTP_HOST` | SMTP server host |
| `TURERP_SMTP_PORT` | SMTP server port | `587` |
| `TURERP_SMTP_USER` | SMTP username |
| `TURERP_SMTP_PASSWORD` | SMTP password |

### Vault (Secrets)

| Variable | Description |
|----------|-------------|
| `TURERP_VAULT_ADDR` | HashiCorp Vault address |
| `TURERP_VAULT_TOKEN` | Vault token |
| `TURERP_VAULT_PATH` | Secret path |

### e-Fatura / GIB

| Variable | Description | Default |
|----------|-------------|---------|
| `TURERP_GIB_API_URL` | GIB API endpoint | |
| `TURERP_GIB_USERNAME` | GIB username | |
| `TURERP_GIB_PASSWORD` | GIB password | |
| `TURERP_GIB_TEST_MODE` | Test mode | `true` |

### CDC

| Variable | Description | Default |
|----------|-------------|---------|
| `TURERP_CDC_ENABLED` | Enable CDC triggers | `false` |
| `TURERP_CDC_CHANNELS` | CDC channels | `invoice_changes,stock_changes` |

## Security

### OWASP Top 10 Protection

- **SQL Injection Prevention** - Parameterized queries via SQLx
- **JWT Token Security** - Token validation and tampering protection
- **MFA/TOTP** - Time-based one-time passwords with backup codes
- **Authentication Security** - Strong password policies, rate limiting
- **API Key Authentication** - HMAC-hashed keys with scope validation
- **IP Whitelisting** - CIDR-based tenant-scoped access control
- **Authorization** - Role-based access control (Admin, User, Viewer)
- **Input Validation** - All inputs validated via `validator` crate
- **Security Headers** - HSTS, CSP, X-Frame-Options, X-Content-Type-Options
- **Idempotency** - Per-endpoint idempotency keys with TTL
- **Encryption** - AES-256-GCM for sensitive data at rest
- **Secrets Management** - HashiCorp Vault integration
- **Tenant Isolation** - Mandatory `tenant_id` filtering
- **Audit Trail** - All authenticated requests logged with batch persistence
- **Graceful Shutdown** - 30-second timeout for in-flight requests

### JWT Authentication

All API endpoints (except auth, customer portal login/register, vendor portal login/register, health, metrics) require a JWT Bearer token:

```bash
# Get token
curl -X POST http://localhost:8000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"password"}'

# Authenticated request
curl http://localhost:8000/api/v1/users \
  -H "Authorization: Bearer <access_token>"
```

### Rate Limiting

Auth endpoints are protected with rate limiting:
- **Limit**: 10 requests/minute (configurable)
- **Burst**: 3 requests (configurable)
- **Trusted Proxies**: Configure `TURERP_TRUSTED_PROXIES` for load balancer support

### Password Requirements

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

Each tenant's data is isolated via `tenant_id` from the JWT token. All database queries filter by `tenant_id`. Users can only access their own tenant's data.

## Contributing

1. Create an issue (or pick an existing open one)
2. Create a feature branch: `git checkout -b feature/<issue-number>-<short-description>`
3. Commit your changes with [Conventional Commits](https://www.conventionalcommits.org/)
4. Push the branch: `git push origin feature/<issue-number>-<short-description>`
5. Open a Pull Request (must reference the open issue)
6. Merge only after CI passes and approval

See [`AGENTS.md`](AGENTS.md) for detailed workflow rules.

## Documentation

- [`AGENTS.md`](AGENTS.md) - Developer guide, architecture, best practices
- [`CONTRIBUTING.md`](CONTRIBUTING.md) - Contribution workflow, commit conventions
- [`CLAUDE.md`](CLAUDE.md) - Claude Code configuration and agent rules
- [`docs/`](docs/) - Project documentation

## License

GNU Affero General Public License v3.0 (AGPL-3.0)

TurERP is free software: you can redistribute it and/or modify it under the terms
of the GNU Affero General Public License as published by the Free Software Foundation,
either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this
program. If not, see <https://www.gnu.org/licenses/>.

**SaaS Copyleft Notice:** If you run a modified version of TurERP on a network server,
Section 13 of the AGPL requires you to make the Corresponding Source available to users
interacting with it remotely.

---

Copyright (c) 2024 Turerp Team

**Turerp Team** - Built with Rust for modern ERP solutions.
