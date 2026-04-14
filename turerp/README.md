# Turerp ERP

[![CI](https://github.com/turerp/turerp-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/turerp/turerp-rust/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-314%20passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](#license)

**Multi-tenant SaaS ERP system** - Built with Rust, Actix-web, and SQLx.

## Features

### Core Modules
- **Authentication**: JWT-based auth, bcrypt password hashing, token refresh
- **Tenant Management**: Subdomain-based multi-tenant architecture
- **User Management**: Roles (Admin, User, Viewer), CRUD operations
- **Cari**: Customer/Vendor accounts, credit limit management
- **Products**: Category management, units, barcode support
- **Stock**: Warehouse management, stock movements, valuation
- **Invoices**: Invoice creation, payment tracking, tax calculations

### Business Modules
- **Sales**: Orders, quotations, pricing
- **Purchase**: Orders, goods receipt, vendor management
- **HR**: Employee management, payroll, leave tracking
- **Accounting**: Chart of accounts, journal entries, trial balance

### Advanced Modules
- **Project Management**: WBS, project costs, profitability analysis
- **Manufacturing**: Work orders, routing, BOM management
- **Quality Control**: Inspections, non-conformance reports (NCR)
- **CRM**: Leads, opportunities, campaign management
- **Audit**: Request audit trail, batch persistence, admin query API

### Infrastructure
- **Prometheus Metrics**: Request counters, latency histograms, `/metrics` endpoint
- **Health Checks**: Liveness (`/health/live`) and readiness (`/health/ready`) probes
- **Pagination**: All list endpoints return `PaginatedResult<T>`
- **Trusted Proxy**: Configurable trusted proxies for rate limiting behind load balancers

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Web Framework | Actix-web 4 |
| Database | PostgreSQL + SQLx |
| Auth | JWT + bcrypt |
| Validation | validator |
| Serialization | serde |
| Error Handling | thiserror + anyhow |
| Logging | tracing |
| Metrics | metrics + metrics-exporter-prometheus |
| API Docs | utoipa (OpenAPI/Swagger) |

## Installation

### Prerequisites
- Rust 1.70+
- PostgreSQL 14+
- (Optional) Docker & Docker Compose

### Quick Start (Docker)

```bash
cd turerp
docker-compose up -d
# API: http://localhost:8080
# Swagger UI: http://localhost:8080/swagger-ui/
```

> **Note**: Docker uses port 8080, local development uses port 8000.

### Development Setup

```bash
# Install Rust (rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Create PostgreSQL database
createdb turerp

# Set environment variables
export TURERP_DATABASE_URL="postgres://postgres:postgres@localhost:5432/turerp"
export TURERP_JWT_SECRET="your-secret-key-change-in-production"

# Clone the repository
git clone https://github.com/turerp/turerp-rust.git
cd turerp-rust/turerp

# Install dependencies and build
cargo build

# Run tests
cargo test

# Start the server
cargo run
# API: http://localhost:8000
# Swagger UI: http://localhost:8000/swagger-ui/
```

## API Endpoints

### Authentication
```
POST /api/auth/register  - Register new user
POST /api/auth/login     - Login
POST /api/auth/refresh   - Token refresh
GET  /api/auth/me        - Current user info
```

### Users
```
GET    /api/users        - User list (paginated)
POST   /api/users        - Create user
GET    /api/users/{id}   - User details
PUT    /api/users/{id}   - Update user
DELETE /api/users/{id}   - Delete user
```

### Audit Logs (Admin only)
```
GET /api/v1/audit-logs   - List audit logs (paginated, filterable)
```

### Health & Monitoring
```
GET /health        - Health check (alias for readiness)
GET /health/live   - Liveness probe (always 200)
GET /health/ready  - Readiness probe (checks DB, version, latency)
GET /metrics       - Prometheus metrics
```

### Tenant
```
GET    /api/tenants        - Tenant list
POST   /api/tenants        - Create tenant
GET    /api/tenants/{id}   - Tenant details
PUT    /api/tenants/{id}   - Update tenant
DELETE /api/tenants/{id}   - Delete tenant
```

### Swagger UI
- **Swagger UI**: `http://localhost:8000/swagger-ui/`
- **OpenAPI Spec**: `http://localhost:8000/api-docs/openapi.json`

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
│  Sales  │  Purchase  │  HR  │  Accounting  │  Assets      │
├─────────┴───────────┴──────┴──────────────┴────────────────┤
│                   Extended Modules                           │
│  Projects  │  Manufacturing  │  BOM  │  QC  │  Shop Floor  │
├───────────┴─────────────────┴───────┴──────┴───────────────┤
│                         CRM                                  │
│     Leads  │  Opportunities  │  Campaigns  │  Tickets        │
└─────────────────────────────────────────────────────────────┘
```

### Project Structure
```
turerp/
├── src/
│   ├── api/              # HTTP handlers (Actix-web)
│   ├── config/           # Configuration
│   ├── domain/           # Domain modules (business logic)
│   │   ├── auth/         # Authentication
│   │   ├── user/         # User management
│   │   ├── tenant/       # Tenant management
│   │   ├── cari/         # Cari accounts
│   │   ├── product/      # Product management
│   │   ├── stock/        # Stock management
│   │   ├── invoice/      # Invoice management
│   │   ├── sales/        # Sales module
│   │   ├── purchase/     # Purchase module
│   │   ├── hr/           # HR module
│   │   ├── accounting/   # Accounting module
│   │   ├── project/      # Project management
│   │   ├── manufacturing/# Manufacturing module
│   │   └── crm/          # CRM module
│   ├── error/            # Error handling
│   ├── middleware/       # HTTP middleware
│   ├── utils/            # Utility functions
│   ├── lib.rs            # Library entry point
│   └── main.rs           # Application entry point
├── tests/                # Integration tests
├── docs/                 # Module documentation
└── Cargo.toml            # Dependencies
```

## Testing

```bash
# All tests
cargo test

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

| Variable | Description | Default |
|----------|-------------|---------|
| `TURERP_DATABASE_URL` | PostgreSQL connection string | Required |
| `TURERP_JWT_SECRET` | JWT signing key | Required |
| `TURERP_SERVER_HOST` | Server host | `0.0.0.0` |
| `TURERP_SERVER_PORT` | Server port | `8000` |
| `TURERP_DB_MAX_CONNECTIONS` | Max DB connections | `10` |
| `TURERP_JWT_ACCESS_EXPIRATION` | Access token duration (seconds) | `3600` |
| `TURERP_JWT_REFRESH_EXPIRATION` | Refresh token duration (seconds) | `604800` |
| `TURERP_TRUSTED_PROXIES` | Comma-separated trusted proxy IPs | (none) |
| `TURERP_RATE_LIMIT_RPM` | Rate limit requests per minute | `10` |
| `TURERP_RATE_LIMIT_BURST` | Rate limit burst size | `3` |
| `TURERP_METRICS_ENABLED` | Enable Prometheus metrics | `true` |
| `TURERP_METRICS_PATH` | Metrics endpoint path | `/metrics` |
| `RUST_LOG` | Log level | `info` |

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: amazing feature'`)
4. Push the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Documentation

- [Module Documentation](docs/README.md)
- [Implementation Plan](IMPLEMENTATION_PLAN.md)
- [API Documentation](http://localhost:8000/swagger-ui/)

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

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

**Turerp Team** - Built with Rust for modern ERP solutions.