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
- [x] Database routing per tenant (TenantPoolRegistry: caches per tenant but uses same shared pool for all tenants; not true per-tenant DB isolation)

### 1.4 Users Module
- [x] User management within tenant
- [x] Role assignment (Admin, User, Viewer defined but Viewer role has no differentiated authorization logic)
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
- [x] Inspections (model types only - no API endpoints)
- [x] Non-conformance reports (NCR) (model types only - no API endpoints)

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
- [x] OpenAPI/Swagger UI (partial: ~13 paths documented out of ~170 handlers; most v1 endpoints not yet in spec)
- [x] API versioning (/api/v1/ prefix)
- [x] Legacy `/api/auth/*` and `/api/users/*` routes exist in codebase but are NOT wired into the production router (dead code; only configured in integration tests)
- [x] Rate limiting (governor crate)

### 9.2 Testing & Security
- [x] Unit tests (250 passing)
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
- [x] Add #[must_use] attributes to important return types (only 1 applied: `TokenPair::generate_tokens` in jwt.rs)
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

## Current Status: Phase 20 - Complete ✅

### Completed Modules
| Module | Status | Notes |
|--------|--------|-------|
| Auth | ✅ Complete | JWT, bcrypt, rate limiting, OpenAPI, role-based auth |
| Tenant | ✅ Complete | Subdomain routing, PostgreSQL repo, TenantConfig CRUD API (5 endpoints) |
| User | ✅ Complete | CRUD + roles + validation tests + admin auth |
| Cari | ✅ Complete | Customer/Vendor + PostgreSQL repo, Decimal, Soft Delete |
| Product | ✅ Complete | Categories, units, variants, Decimal |
| Stock | ✅ Complete | Warehouses, movements, Decimal |
| Invoice | ✅ Complete | Payments, status, Decimal |
| Sales | ✅ Complete | Orders, quotations, Decimal |
| Purchase | ✅ Complete | Orders, goods receipt, purchase requests, Decimal |
| HR | ✅ Complete | Employees, attendance, leave, Decimal |
| Accounting | ✅ Complete | Journal entries, trial balance, Decimal |
| Assets | ✅ Complete | Fixed assets, depreciation, maintenance |
| Project | ✅ Complete | WBS, costs, profitability, Decimal |
| Manufacturing | ✅ Complete | Work orders, BOM, routing, Decimal, QC inspections & NCR API endpoints |
| CRM | ✅ Complete | Leads, opportunities, tickets, Decimal |
| Custom Fields | ✅ Complete | Dynamic field definitions per module, JSONB values, validation |
| Feature Flags | ✅ Complete | CRUD, tenant-specific, API v1, admin auth |
| Product Variants | ✅ Complete | CRUD API v1 with AdminUser checks for create/update/delete |
| Purchase Requests | ✅ Complete | CRUD, approval workflow, API v1, state machine, pagination |
| Audit | ✅ Complete | Request audit trail, batch persistence, admin query API |
| Tax Engine | ✅ Complete | Tax types (KDV, OIV, Stopaj, BSMV, Damga), calculator modules, effective rates, periods |
| Chart of Accounts | ✅ Complete | Hierarchical accounts (parent-child), account types, PostgreSQL repo, REST API |
| e-Fatura | ✅ Complete | UBL-TR compliant model, XML validator, status workflow, PostgreSQL repo, REST API |
| e-Defter | ✅ Complete | Yevmiye/Büyük Defter/Defteri Kebir, berat signing, entry management, REST API |
| Webhooks | ✅ Complete | Event subscriptions, delivery tracking, retry logic, REST API |

### Infrastructure & Operations
| Feature | Status | Notes |
|---------|--------|-------|
| Centralized Error Handling | ✅ Complete | `map_sqlx_error` with PG error codes (23505, 23503) |
| Trusted Proxy Config | ✅ Complete | `TURERP_TRUSTED_PROXIES` for rate limiting behind LBs |
| Composite DB Indexes | ✅ Complete | `tenant_id + created_at DESC` on all multi-tenant tables |
| Health Checks | ✅ Complete | `/health/live` (liveness), `/health/ready` (readiness + DB) |
| Prometheus Metrics | ✅ Complete | `http_requests_total`, `http_request_duration_seconds`, `/metrics` |
| Pagination | ✅ Complete | All list endpoints return `PaginatedResult<T>` (20+ paginated endpoints across all modules) |
| Custom Fields | ✅ Complete | Dynamic field definitions per module, JSONB values, validation, admin-only API |
| Audit Log API | ✅ Complete | `GET /api/v1/audit-logs` with filtering + pagination |
| Notifications API | ✅ Complete | 6 endpoints: send, in-app list, unread count, mark read, mark all read, retry |
| Reports API | ✅ Complete | 4 endpoints: generate, list, download, delete (PDF/Excel/XML/CSV/JSON) |
| Events API | ✅ Complete | 5 endpoints: publish, process outbox, pending events, dead letters, retry |
| Search API | ✅ Complete | 4 endpoints: search, index, remove, reindex (full-text with fuzzy matching) |

### Test Coverage
- **489 tests passing** (424 unit + 38 integration + 27 security)
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
- ✅ Secure public path matching (exact match + prefix for directory-like paths; note: non-directory paths like `/health` also match their subpaths e.g. `/health/*`)
- ✅ Encryption key memory security (`zeroize` crate present but no explicit zeroize-on-drop implementation; `generate_key()` returns plain `[u8; 32]`)
- ✅ Decimal precision for financial values (all modules)
- ✅ Required tenant_id in registration (no default tenant exposure)
- ✅ Thread-safe in-memory repositories (single mutex pattern)
- ✅ #[must_use] attributes (1 applied: `TokenPair::generate_tokens`)
- ✅ Forbidden (403) error type for authorization failures
- ✅ AdminUser extractor role comparison (lowercase "admin" fix)
- ✅ Centralized DB error handling with PG error code detection
- ⚠️ Default admin credentials (dev only, warning in migrations)

---

## Known Issues & Technical Debt

### API Layer
| Issue | Severity | Description |
|-------|----------|-------------|
| OpenAPI coverage | Low | All 187 v1 handlers are registered in the `ApiDoc` OpenAPI schema. Swagger UI is available at `/swagger-ui/`. |
| Legacy route drift | Medium | `/api/auth/*` and `/api/users/*` legacy modules exist but are never configured in `main.rs` router; integration tests wire them manually, causing test/production divergence |
| Viewer role unused | Low | `Role::Viewer` is defined but no authorization logic differentiates it from `User` |

### Infrastructure
| Issue | Severity | Description |
|-------|----------|-------------|
| TenantPoolRegistry | Low | Caches a connection pool per tenant ID but all pools point to the same database; true per-tenant DB isolation not implemented |
| Zeroize on drop | Low | `zeroize` crate is present but no struct implements `Zeroize`/`Drop` for keys; `generate_key()` returns a plain `[u8; 32]` array |
| #[must_use] coverage | Low | Only 1 attribute applied in entire codebase (jwt.rs) |

### Code Quality
| Issue | Severity | Description |
|-------|----------|-------------|
| **Migration 004 table/column name mismatches** | **Fixed** | Migration `004_composite_indexes.sql` now uses correct table names (`opportunities` not `crm_opportunities`, `tickets` not `support_tickets`) and correctly skips `stock_movements` for `tenant_id` index. |
| **Zeroize dependency** | **Fixed** | `zeroize = "1.8"` is used in `utils/encryption.rs` — `generate_key()` returns `Zeroizing<[u8; 32]>` which is scrubbed from memory on drop. The crate is correctly used. |
| **Service transaction atomicity** | **Fixed** | `InvoiceService::create_invoice` and `create_payment` already had compensating rollback on failure. `SalesService::convert_quotation_to_order` now rolls back order + lines if quotation link fails. All three methods are safe from partial failure. |
| **IDOR: Repository find_by_id/delete missing tenant_id** | **Fixed** | All domain repositories now require `tenant_id` in `find_by_id`, `delete`, `soft_delete`, `restore`, `destroy` methods. PostgreSQL queries use `WHERE id = $1 AND tenant_id = $2`. In-memory repos filter by `tenant_id`. All 339 tests pass. |
| **Quotation line validation gap** | **Fixed** | `CreateQuotationLine` now validates `unit_price >= Decimal::ZERO` (same as `CreateSalesOrderLine`). |
| Test/production route divergence | Medium | Integration tests register legacy routes that don't exist in production `main.rs` |

---

## Remaining Work

### Medium Priority (Completed)
| Feature | Description | Status |
|---------|-------------|--------|
| TenantConfig REST API | CRUD endpoints for per-tenant config (key-value with optional encryption) | ✅ Done — 5 endpoints added in `api/v1/tenant.rs` |
| Quality Control API | Inspection + NCR endpoints for manufacturing module | ✅ Done — 8 endpoints added in `api/v1/manufacturing.rs` |

### Medium Priority
| Feature | Description | Status |
|---------|-------------|--------|
| Performance Testing | Load testing with realistic data | Planned |
| API Response Caching | Cache frequently accessed data | Planned |

### Low Priority
| Feature | Description | Status |
|---------|-------------|--------|
| API Rate Limit Dashboard | Visual dashboard for rate limit metrics | Planned |
| Webhook System | Event-driven webhook subscriptions, delivery tracking, retry logic | ✅ Complete |

---

## Phase 13: Localization (i18n) ✅

### 13.1 Infrastructure
- [x] JSON-based translation bundles (`locales/en.json`, `locales/tr.json`)
- [x] Eager-load translations at startup via `std::sync::OnceLock`
- [x] Accept-Language header parsing with quality values (`q=0.8`)
- [x] Configurable default locale (`TURERP_DEFAULT_LOCALE`)
- [x] Simple `{placeholder}` string interpolation

### 13.2 Integration Points
- [x] `I18n` service added to `AppState` and injected as `web::Data`
- [x] `Locale` Actix extractor pulls preferred language per-request
- [x] `ErrorResponse` extended with optional `localized_error` field
- [x] `ApiError::error_response()` consumes locale context when available

### 13.3 Translation Coverage
| Domain | Keys | Status |
|--------|------|--------|
| Auth | invalid_credentials, token_expired, invalid_token | ✅ |
| Generic | not_found, unauthorized, forbidden, bad_request, conflict, validation_error, internal_error, database_error | ✅ |

### 13.4 Files Added
- `src/i18n/mod.rs` — Core localization engine
- `src/i18n/extractor.rs` — `Locale` Actix extractor
- `locales/en.json` — English translations
- `locales/tr.json` — Turkish translations

---

## Phase 14: Enterprise Infrastructure (P0 - Critical)

### 14.1 Soft Delete (All Domains) — Complete
- [x] Add `SoftDeleteMeta` struct and `SoftDeletable` trait in `common/soft_delete.rs`
- [x] Add `deleted_at` / `deleted_by` columns to all tables via migration `007_soft_delete.sql`
- [x] Cari module: model, repository, postgres_repository, service, API endpoints
- [x] Invoice module: model, repository, postgres_repository, service, API endpoints
- [x] Product module: model, repository, postgres_repository, service, API endpoints
- [x] Stock module: model, repository, postgres_repository, service, API endpoints
- [x] Sales module: model, repository, postgres_repository, service, API endpoints
- [x] Purchase module: model, repository, postgres_repository, service, API endpoints
- [x] HR module: model, repository, service, API endpoints
- [x] Accounting module: model, repository, service, API endpoints
- [x] Assets module: model, repository, service, API endpoints
- [x] Project module: model, repository, service, API endpoints
- [x] Manufacturing module: model, repository, service, API endpoints
- [x] CRM module: model, repository, service, API endpoints
- [x] Update all `find_all` queries with `WHERE deleted_at IS NULL`
- [x] Add admin-only "list deleted" and "restore" endpoints
- [x] Add `destroy()` (hard delete) for super-admin

### 14.2 Event-Driven Architecture (Outbox Pattern) — Complete
- [x] `EventBus` trait with `InMemoryEventBus` (Redis Streams backend pluggable via trait)
- [x] `OutboxEvent` model with `EventStatus` (Pending/Published/Failed/DeadLettered)
- [x] Domain Events: `InvoiceCreated`, `PaymentReceived`, `StockMoved`, `EmployeeHired`, `SalesOrderCreated`, `PurchaseOrderApproved`, `Custom`
- [x] Event subscribers: `StockDecrementSubscriber` (InvoiceCreated→StockDecrement), `AccountingEntrySubscriber` (InvoiceCreated/PaymentReceived→AccountingEntry)
- [x] Dead Letter Queue (DLQ) with `DeadLetterEntry`, max retries, and retry-from-DLQ
- [x] Batch processing via `process_outbox()`, batch publish, and subscriber dispatch
- [x] Integrated into AppState with `BoxEventBus`

### 14.3 Background Job Scheduler — Complete
- [x] `JobScheduler` trait with `InMemoryJobScheduler` (PostgreSQL backend pluggable via trait)
- [x] Job types: `CalculateDepreciation`, `RunPayroll`, `SendReminders`, `ArchiveLogs`, `GenerateReport`, `Custom`
- [x] Priority queue (Low/Normal/High/Critical) with priority-based `next_pending()`
- [x] Retry mechanism with exponential backoff (2^attempts seconds)
- [x] Job scheduling (`scheduled_at` for future execution)
- [x] Cleanup of old completed/failed/cancelled jobs
- [x] REST API endpoints (`/api/v1/jobs/*`) for full job lifecycle management
- [x] Integrated into AppState with `BoxJobScheduler`

### 14.4 Notification Service (Email/SMS) — Complete
- [x] `NotificationService` trait with `InMemoryNotificationService` (SMTP/SendGrid/SES pluggable via trait)
- [x] Email template engine with string interpolation (5 built-in templates: invoice_created, payment_received, employee_hired, stock_low, password_reset)
- [x] Custom template registration via `register_template()`
- [x] Notification channels: Email, SMS, InApp
- [x] Notification priorities: Low, Normal, High, Urgent
- [x] In-app notification bell: `get_in_app_notifications()`, `mark_as_read()`, `mark_all_as_read()`, `unread_count()`
- [x] Tenant-isolated in-app notifications
- [x] Async notification queue with retry support
- [x] Integrated into AppState with `BoxNotificationService`

### 14.5 Idempotency Keys — Complete
- [x] `IdempotencyKey` header middleware
- [x] In-memory response cache with 24h TTL, trait-based store for future Redis backend
- [x] Per-endpoint idempotency support via middleware chain

### 14.6 API Key Authentication — Complete
- [x] `ApiKey` model with scopes and expiry
- [x] `ApiKeyService` with CRUD operations and authentication
- [x] `ApiKeyAuth` extractor for X-API-Key header validation
- [x] Scoped permissions: `["cari:read", "cari:write", "invoice:read", "invoice:write", "stock:read", "stock:write", "sales:read", "sales:write", "product:read", "product:write", "report:read", "all"]`
- [x] Admin CRUD API endpoints (`/api/v1/api-keys`)
- [x] `ApiKeyScope` enum with Display/FromStr for string serialization
- [x] SHA-256 key hashing with prefix extraction for identification
- [x] In-memory repository with tenant isolation

### 14.7 File Upload & Document Management — Complete
- [x] `FileStorage` trait with `LocalFileStorage` (S3/MinIO backends pluggable via trait)
- [x] Presigned URL support (interface defined, local backend returns error)
- [x] File metadata tracking with tenant isolation, soft delete, checksum
- [x] File upload/download/delete/list/storage_used operations
- [x] Tenant-isolated storage directories
- [x] Integrated into `common/mod.rs` with `BoxFileStorage` type alias

### 14.8 Full-Text Search — Complete (InMemory backend)
- [x] `SearchService` trait with async search/index/remove/reindex
- [x] `SearchDocument` model with entity_type, id, tenant_id, title, description
- [x] `SearchQuery` with entity type filtering, min score, result limit
- [x] `InMemorySearchService` with fuzzy matching (word overlap, prefix match, substring)
- [x] Tenant isolation in search
- [x] Turkish locale support (case-insensitive, Unicode-aware)
- [x] PostgreSQL `pg_trgm` + `unaccent` ready (trait-based, pluggable)

### 14.9 Redis Caching Layer — Complete (InMemory backend)
- [x] `CacheService` trait with async get/set/delete/exists/clear_namespace/get_or_set
- [x] `InMemoryCacheService` with TTL support, eviction, and namespace isolation
- [x] Tenant config, feature flags, user perms caching namespaces
- [x] Cache key helper and common namespace constants
- [x] Redis backend placeholder (trait-based, pluggable when Redis is added)

### 14.10 Report Engine (PDF/Excel/XML) — Complete
- [x] `ReportEngine` trait with `InMemoryReportEngine` (production PDF/Excel backends pluggable via trait)
- [x] `ReportFormat` enum: Pdf, Excel, Xml, Csv, Json
- [x] `ReportType` enum: Invoice, TrialBalance, BalanceSheet, IncomeStatement, PayrollSummary, StockSummary, SalesReport, PurchaseReport, AgingReport, EDefter, Custom
- [x] Invoice PDF generation (placeholder structure, pluggable with wkhtmltopdf/genpdf)
- [x] Excel export for accounting/HR (placeholder structure, pluggable with rust_xlsxwriter)
- [x] e-Defter UBL-TR XML format (GenericAccountingPacket with period/tenant)
- [x] `ReportRequest` with JSON parameters, locale support
- [x] `GeneratedReport` with binary data, content type, filename
- [x] `ReportMeta` lightweight metadata for listing without binary payload
- [x] Tenant-isolated report storage, CRUD operations
- [x] Integrated into AppState with `BoxReportEngine`

### 14.11 Distributed Tracing (OpenTelemetry) — Complete
- [x] `TracingService` trait with `InMemoryTracingService` (Jaeger/Tempo backends pluggable via trait + `tracing-opentelemetry`)
- [x] `TraceSpan` model: trace_id, span_id, parent_span_id, operation_name, status, attributes, service_name
- [x] W3C Trace Context propagation (`traceparent` header: extract + inject)
- [x] `TraceContext` for cross-service span propagation
- [x] `TraceQuery` with tenant, operation, trace_id, duration, status filters
- [x] Span lifecycle: start_span → end_span with Ok/Error/Unset status
- [x] Duration calculation (`duration_ms()`)
- [x] Tenant isolation in trace search
- [x] Integrated into AppState with `BoxTracingService`

### 14.12 Database Read Replicas — Complete
- [x] `DbRouter` trait with `InMemoryDbRouter` (sqlx pool-based backends pluggable via trait)
- [x] `QueryType` classification: Read vs Write
- [x] Write queries always routed to master
- [x] Read queries routed to healthy replicas (round-robin)
- [x] Fallback to master when no healthy replicas available
- [x] `ReadAfterWriteMode`: Session (read-after-write), Eventual (replica-ok), Strict (always-master)
- [x] Session write tracking with `mark_session_write()` / `clear_session()`
- [x] `ReplicaNode` health tracking: Healthy/Unhealthy/Unknown with last check timestamp
- [x] Replication lag tracking per replica
- [x] `RouterStats`: total reads/writes, reads_to_master, reads_to_replica, active/total replicas
- [x] Weighted replica selection support
- [x] Integrated into AppState with `BoxDbRouter`

---

## Phase 15: Custom Fields (Dynamic Attributes) — Complete

### 15.1 Custom Field Definitions — Complete
- [x] `CustomFieldModule` enum (Cari, Invoice, Stock, Sales, Purchase, Hr, Accounting, Project, Manufacturing, Crm, Asset, Product)
- [x] `CustomFieldType` enum (String, Number, Date, Boolean, Select)
- [x] `CustomFieldDefinition` entity with `SoftDeletable` support
- [x] `CreateCustomFieldDefinition` / `UpdateCustomFieldDefinition` DTOs with validation
- [x] `CustomFieldDefinitionResponse` response DTO (excludes deleted_at/deleted_by)
- [x] `CustomFieldRepository` trait with CRUD, `find_by_module`, `field_name_exists`, `soft_delete`
- [x] `InMemoryCustomFieldRepository` implementation with `parking_lot::Mutex<HashMap>`
- [x] `PostgresCustomFieldRepository` implementation with sqlx (migration `008_custom_fields.sql`)
- [x] `CustomFieldService` with create, get_by_id, list_by_module, list_all, update, soft_delete
- [x] `validate_entity_custom_fields()` — validates JSONB values against definitions (type check, required check, Select options check)
- [x] Standalone `validate_custom_fields()` function for use outside service context
- [x] Partial unique index: `UNIQUE(tenant_id, module, field_name) WHERE deleted_at IS NULL`
- [x] Admin-only REST API: `POST/GET/PUT/DELETE /api/v1/custom-fields`, `GET /api/v1/custom-fields?module={module}`
- [x] Integrated into AppState, main.rs, domain/mod.rs, api/mod.rs
- [x] 14 unit tests (model, repository, service, validation)

---

## Phase 16: Tax Engine — Complete ✅

### 16.1 Tax Model & Domain
- [x] `TaxType` enum: KDV, OIV, Stopaj, BSMV, Damga, OTV, MTV, GecikmeFaizi, Diger
- [x] `TaxRate` model with effective date range (`effective_from`, `effective_to`)
- [x] `TaxPeriod` model with status (Open, Closed, Filed, Late)
- [x] `TaxPeriodDetail` for per-transaction tax entries
- [x] `CreateTaxRate` / `UpdateTaxRate` DTOs with validation

### 16.2 Tax Calculator Modules
- [x] `KdvCalculator` — KDV (VAT) computation with configurable rate
- [x] `OivCalculator` — OIV (Special Consumption Tax) computation
- [x] `StopajCalculator` — Stopaj (Withholding tax) computation
- [x] `BsmvCalculator` — BSMV (Banking & Insurance Tax) computation
- [x] `DamgaCalculator` — Damga (Stamp duty) computation
- [x] `TaxCalculator` trait for uniform interface across all tax types
- [x] Calculator factory (`TaxCalculatorFactory`) by `TaxType`

### 16.3 Tax Repository & Service
- [x] `TaxRateRepository` trait with CRUD, effective-rate lookup
- [x] `TaxPeriodRepository` trait with detail line support
- [x] `InMemoryTaxRateRepository` with effective-date filtering logic
- [x] `InMemoryTaxPeriodRepository` with period totals and detail tracking
- [x] `TaxService` with `find_effective_rate()`, period management, detail aggregation

### 16.4 Tax REST API (12 endpoints)
- `GET /api/v1/tax/rates` — List tax rates (paginated, optional type filter)
- `POST /api/v1/tax/rates` — Create tax rate (admin only)
- `GET /api/v1/tax/rates/{id}` — Get tax rate by ID
- `PUT /api/v1/tax/rates/{id}` — Update tax rate (admin only)
- `DELETE /api/v1/tax/rates/{id}` — Delete tax rate (admin only)
- `GET /api/v1/tax/rates/effective/{type}` — Find effective rate for date
- `GET /api/v1/tax/periods` — List tax periods (paginated, optional type filter)
- `POST /api/v1/tax/periods` — Create tax period (admin only)
- `GET /api/v1/tax/periods/{id}` — Get tax period with totals
- `PUT /api/v1/tax/periods/{id}` — Update period status (admin only)
- `POST /api/v1/tax/periods/{id}/details` — Add detail line (admin only)
- `GET /api/v1/tax/periods/{id}/details` — Get detail lines

### 16.5 Tax Tests
- [x] Tax rate CRUD tests
- [x] Effective rate lookup tests (date range, type filtering)
- [x] Tax period CRUD and detail tests

---

## Phase 17: Chart of Accounts Enhancement — Complete ✅

### 17.1 Domain Model
- [x] `ChartAccount` model with hierarchical account codes, parent-child relationships
- [x] `AccountType` enum: Asset, Liability, Equity, Revenue, Expense
- [x] `AccountStatus` enum: Active, Passive
- [x] Soft-delete support via `SoftDeletable` trait

### 17.2 Repository & Service
- [x] `ChartAccountRepository` trait with CRUD, find by type, find by parent
- [x] `PostgresChartAccountRepository` with tenant-isolated queries
- [x] `ChartAccountService` with business logic

### 17.3 REST API (9 endpoints)
- `GET /api/v1/chart-of-accounts` — List accounts (paginated)
- `POST /api/v1/chart-of-accounts` — Create account (admin only)
- `GET /api/v1/chart-of-accounts/{id}` — Get account by ID
- `PUT /api/v1/chart-of-accounts/{id}` — Update account (admin only)
- `DELETE /api/v1/chart-of-accounts/{id}` — Soft delete (admin only)
- `GET /api/v1/chart-of-accounts/type/{type}` — Accounts by type
- `GET /api/v1/chart-of-accounts/{id}/children` — Child accounts
- `POST /api/v1/chart-of-accounts/{id}/restore` — Restore soft-deleted
- `DELETE /api/v1/chart-of-accounts/{id}/destroy` — Hard delete (super-admin)

### 17.4 Database Migration
- [x] `chart_of_accounts` table with `tenant_id`, `parent_id`, `code`, `name`, `type`, `status`, `balance`, soft-delete columns
- [x] Composite index: `tenant_id + code` (unique per tenant)

---

## Phase 18: e-Fatura (e-Invoice) — Complete ✅

### 18.1 Domain Model
- [x] `EFatura` model with UBL-TR compliance fields
- [x] `EFaturaStatus` enum: Draft, Sent, Accepted, Rejected, Cancelled
- [x] `EFaturaProfile` enum: TemelFatura, TicariFatura, IhracatFatura, YolcuFatura
- [x] `PartyInfo` / `AddressInfo` for sender/receiver
- [x] `MonetaryTotal` for legal monetary totals
- [x] `EFaturaLine` for invoice line items

### 18.2 UBL-TR Support
- [x] UBL Invoice XML structure types
- [x] XML validator (`ubl/validator.rs`) with schema validation
- [x] XML generation helper types

### 18.3 Repository & Service
- [x] `EFaturaRepository` trait with CRUD, find by UUID, find by invoice ID, status update, XML update
- [x] `InMemoryEFaturaRepository` with full CRUD, status transitions, soft-delete
- [x] `EFaturaService` with business logic

### 18.4 REST API (7 endpoints)
- `POST /api/v1/e-fatura` — Create e-Fatura document (admin only)
- `GET /api/v1/e-fatura` — List e-Fatura documents (paginated, status filter)
- `GET /api/v1/e-fatura/{id}` — Get e-Fatura by ID
- `GET /api/v1/e-fatura/uuid/{uuid}` — Get by UUID
- `GET /api/v1/e-fatura/invoice/{invoice_id}` — Get by invoice ID
- `PUT /api/v1/e-fatura/{id}/status` — Update status (admin only)
- `PUT /api/v1/e-fatura/{id}/xml` — Update XML content (admin only)

### 18.5 Tests
- [x] e-Fatura CRUD tests
- [x] Find by UUID / invoice ID tests
- [x] Status update and XML update tests

---

## Phase 19: e-Defter (e-Ledger) — Complete ✅

### 19.1 Domain Model
- [x] `LedgerPeriod` model with year, month, period type, status
- [x] `LedgerType` enum: YevmiyeDefteri, BuyukDefter, DefteriKebir
- [x] `EDefterStatus` enum: Draft, Signed, Sent, Rejected
- [x] `YevmiyeEntry` / `YevmiyeLine` for journal entries
- [x] `BeratInfo` for digital signature metadata

### 19.2 Repository & Service
- [x] `EDefterRepository` trait with period CRUD, entry management, berat operations
- [x] `InMemoryEDefterRepository` with full CRUD, entry aggregation, berat storage
- [x] `EDefterService` with business logic

### 19.3 REST API (10 endpoints)
- `POST /api/v1/e-defter/periods` — Create ledger period (admin only)
- `GET /api/v1/e-defter/periods` — List periods (paginated, year/type filters)
- `GET /api/v1/e-defter/periods/{id}` — Get period by ID
- `PUT /api/v1/e-defter/periods/{id}/status` — Update period status (admin only)
- `POST /api/v1/e-defter/entries` — Add Yevmiye entry (admin only)
- `GET /api/v1/e-defter/entries/{period_id}` — Get entries for period
- `PUT /api/v1/e-defter/periods/{id}/berat` — Update berat info (admin only)
- `GET /api/v1/e-defter/periods/{id}/berat` — Get berat info

### 19.4 Tests
- [x] Period CRUD tests
- [x] Entry add/find tests
- [x] Berat CRUD tests
- [x] Nonexistent period error handling

---

## Phase 20: Webhook System — Complete ✅

### 20.1 Domain Model
- [x] `WebhookSubscription` model with URL, events, secret, headers
- [x] `WebhookEvent` enum: InvoiceCreated, PaymentReceived, StockMoved, EmployeeHired, SalesOrderCreated, PurchaseOrderApproved, Custom
- [x] `WebhookDelivery` model with status, response, retries
- [x] `DeliveryStatus` enum: Pending, Delivered, Failed, RetryScheduled
- [x] Soft-delete support

### 20.2 Repository & Service
- [x] `WebhookRepository` trait with CRUD, find by event, delivery tracking
- [x] `InMemoryWebhookRepository` with full CRUD, delivery history, retry logic
- [x] `WebhookService` with event dispatch, delivery tracking, retry scheduling

### 20.3 REST API (8 endpoints)
- `GET /api/v1/webhooks` — List subscriptions (paginated)
- `POST /api/v1/webhooks` — Create subscription (admin only)
- `GET /api/v1/webhooks/{id}` — Get subscription by ID
- `PUT /api/v1/webhooks/{id}` — Update subscription (admin only)
- `DELETE /api/v1/webhooks/{id}` — Delete subscription (admin only)
- `GET /api/v1/webhooks/{id}/deliveries` — Delivery history
- `POST /api/v1/webhooks/{id}/retry` — Retry failed delivery (admin only)
- `GET /api/v1/webhooks/events` — List available event types

### 20.4 Tests
- [x] Subscription CRUD tests
- [x] Delivery tracking tests
- [x] Retry logic tests

---

## Security Fixes

### JWT `sub` Claim Parsing
- [x] **Fixed `unwrap_or(0)` vulnerability** — `AuthClaims::user_id()` method returns `Result<i64, ApiError>`
- [x] Replaced 37 instances of `.sub.parse().unwrap_or(0)` across 17 API handler files
- [x] Replaced 6 verbose `.map_err(...)` patterns with `.user_id()?` for consistency
- [x] Prevents malformed JWT tokens from silently acting as user ID 0

---

## Dependencies (Cargo.toml)

```toml
# Web framework
actix-web = "4"
actix-rt = "2"
actix-cors = "0.7"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio-native-tls", "postgres", "macros", "chrono", "rust_decimal"] }

# Authentication
jsonwebtoken = "9"
bcrypt = "0.15"

# Encryption
aes-gcm = "0.10"
base64 = "0.22"
rand = "0.8"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Validation
validator = { version = "0.20", features = ["derive"] }

# Configuration
config = "0.14"
dotenvy = "0.15"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Async
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# Rate limiting
governor = "0.8"
nonzero_ext = "0.3"

# Validation
regex = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Time
chrono = { version = "0.4", features = ["serde"] }

# UUID
uuid = { version = "1.0", features = ["v4", "serde"] }

# Synchronization (no poisoning)
parking_lot = "0.12"

# Secure memory zeroing
zeroize = "1.8"

# Metrics
metrics = "0.24"
metrics-exporter-prometheus = "0.16"

# Precise decimal for financial calculations
rust_decimal = { version = "1.36", features = ["serde"] }
rust_decimal_macros = "1.36"

# Mocking for tests
mockall = "0.12"

# OpenAPI / Swagger
utoipa = "4"
utoipa-swagger-ui = { version = "6", features = ["actix-web"] }
```

---

## API Endpoints Summary

### Auth (v1)
- `POST /api/v1/auth/register` - Register new user
- `POST /api/v1/auth/login` - Login (rate limited: 10 req/min)
- `POST /api/v1/auth/refresh` - Refresh token
- `GET /api/v1/auth/me` - Current user (requires auth)

### Users (v1)
- `GET /api/v1/users` - List users (requires auth, paginated)
- `POST /api/v1/users` - Create user (admin only)
- `GET /api/v1/users/{id}` - Get user (requires auth)
- `PUT /api/v1/users/{id}` - Update user (self or admin)
- `DELETE /api/v1/users/{id}` - Delete user (admin only)

### Tenants (v1)
- `GET /api/v1/tenants` - List tenants (requires auth, paginated)
- `POST /api/v1/tenants` - Create tenant (admin only)
- `GET /api/v1/tenants/{id}` - Get tenant (requires auth)
- `PUT /api/v1/tenants/{id}` - Update tenant (admin only)
- `DELETE /api/v1/tenants/{id}` - Delete tenant (admin only)

### Cari (v1)
- `GET /api/v1/cari` - List cari accounts (requires auth, paginated)
- `POST /api/v1/cari` - Create cari account (admin only)
- `GET /api/v1/cari/{id}` - Get cari account (requires auth)
- `PUT /api/v1/cari/{id}` - Update cari account (admin only)
- `DELETE /api/v1/cari/{id}` - Delete cari account (admin only)
- `GET /api/v1/cari/type/{cari_type}` - Get cari by type (requires auth, paginated)
- `GET /api/v1/cari/search` - Search cari by code/name (requires auth, paginated)

### Products (v1)
- `GET /api/v1/products` - List products (requires auth, paginated)
- `GET /api/v1/categories` - List categories (requires auth, paginated)
- `GET /api/v1/products/{product_id}/variants` - List variants (requires auth)
- `POST /api/v1/products/{product_id}/variants` - Create variant (requires auth)
- `GET /api/v1/variants/{id}` - Get variant (requires auth)
- `PUT /api/v1/variants/{id}` - Update variant (requires auth)
- `DELETE /api/v1/variants/{id}` - Delete variant (requires auth)

### Stock (v1)
- `GET /api/v1/stock/warehouses` - List warehouses (requires auth, paginated)
- `POST /api/v1/stock/warehouses` - Create warehouse (admin only)
- `GET /api/v1/stock/warehouses/{id}` - Get warehouse (requires auth)
- `PUT /api/v1/stock/warehouses/{id}` - Update warehouse (admin only)
- `DELETE /api/v1/stock/warehouses/{id}` - Delete warehouse (admin only)
- `POST /api/v1/stock/movements` - Create stock movement (admin only)
- `GET /api/v1/stock/movements/product/{product_id}` - Movements by product (requires auth)
- `GET /api/v1/stock/movements/warehouse/{warehouse_id}` - Movements by warehouse (requires auth)
- `GET /api/v1/stock/levels/product/{product_id}` - Stock levels by product (requires auth)
- `GET /api/v1/stock/levels/warehouse/{warehouse_id}` - Stock levels by warehouse (requires auth)
- `GET /api/v1/stock/summary/{product_id}` - Stock summary (requires auth)

### Invoices (v1)
- `GET /api/v1/invoices` - List invoices (requires auth, paginated)
- `POST /api/v1/invoices` - Create invoice (admin only)
- `GET /api/v1/invoices/{id}` - Get invoice (requires auth)
- `DELETE /api/v1/invoices/{id}` - Delete invoice (admin only)
- `GET /api/v1/invoices/status/{status}` - Invoices by status (requires auth, paginated)
- `GET /api/v1/invoices/outstanding` - Outstanding invoices (requires auth)
- `GET /api/v1/invoices/overdue` - Overdue invoices (requires auth)
- `PUT /api/v1/invoices/{id}/status` - Update invoice status (admin only)
- `GET /api/v1/invoices/{id}/payments` - Get payments by invoice (requires auth)
- `POST /api/v1/invoices/payments` - Add payment (admin only)

### Sales (v1)
- `GET /api/v1/sales/orders` - List sales orders (requires auth, paginated)
- `POST /api/v1/sales/orders` - Create sales order (admin only)
- `GET /api/v1/sales/orders/{id}` - Get sales order (requires auth)
- `DELETE /api/v1/sales/orders/{id}` - Delete sales order (admin only)
- `GET /api/v1/sales/orders/status/{status}` - Orders by status (requires auth, paginated)
- `PUT /api/v1/sales/orders/{id}/status` - Update order status (admin only)
- `GET /api/v1/sales/quotations` - List quotations (requires auth, paginated)
- `POST /api/v1/sales/quotations` - Create quotation (admin only)
- `GET /api/v1/sales/quotations/{id}` - Get quotation (requires auth)
- `DELETE /api/v1/sales/quotations/{id}` - Delete quotation (admin only)
- `GET /api/v1/sales/quotations/status/{status}` - Quotations by status (requires auth, paginated)
- `PUT /api/v1/sales/quotations/{id}/status` - Update quotation status (admin only)
- `POST /api/v1/sales/quotations/{id}/convert` - Convert quotation to order (admin only)

### HR (v1)
- `GET /api/v1/hr/employees` - List employees (requires auth, paginated)
- `POST /api/v1/hr/employees` - Create employee (admin only)
- `GET /api/v1/hr/employees/{id}` - Get employee (requires auth)
- `PUT /api/v1/hr/employees/{id}/status` - Update employee status (admin only)
- `POST /api/v1/hr/employees/{id}/terminate` - Terminate employee (admin only)
- `POST /api/v1/hr/attendance` - Record attendance (admin only)
- `GET /api/v1/hr/attendance/employee/{employee_id}` - Attendance by employee (requires auth)
- `POST /api/v1/hr/leave-requests` - Create leave request (requires auth)
- `GET /api/v1/hr/leave-requests/employee/{employee_id}` - Leave requests by employee (requires auth)
- `POST /api/v1/hr/leave-requests/{id}/approve` - Approve leave request (admin only)
- `POST /api/v1/hr/leave-requests/{id}/reject` - Reject leave request (admin only)
- `GET /api/v1/hr/leave-types` - List leave types (requires auth)
- `POST /api/v1/hr/payroll/calculate` - Calculate payroll (admin only)
- `GET /api/v1/hr/payroll/employee/{employee_id}` - Payroll by employee (requires auth)
- `POST /api/v1/hr/payroll/{id}/paid` - Mark payroll as paid (admin only)

### Accounting (v1)
- `GET /api/v1/accounting/accounts` - List accounts (requires auth, paginated)
- `POST /api/v1/accounting/accounts` - Create account (admin only)
- `GET /api/v1/accounting/accounts/type/{account_type}` - Accounts by type (requires auth)
- `GET /api/v1/accounting/accounts/{id}` - Get account (requires auth)
- `GET /api/v1/accounting/journal-entries` - List journal entries (requires auth, paginated)
- `POST /api/v1/accounting/journal-entries` - Create journal entry (admin only)
- `GET /api/v1/accounting/journal-entries/{id}` - Get journal entry (requires auth)
- `POST /api/v1/accounting/journal-entries/{id}/post` - Post journal entry (admin only)
- `POST /api/v1/accounting/journal-entries/{id}/void` - Void journal entry (admin only)
- `POST /api/v1/accounting/trial-balance` - Generate trial balance (requires auth)

### Assets (v1)
- `GET /api/v1/assets` - List assets (requires auth, paginated)
- `POST /api/v1/assets` - Create asset (admin only)
- `GET /api/v1/assets/{id}` - Get asset (requires auth)
- `PUT /api/v1/assets/{id}` - Update asset (admin only)
- `DELETE /api/v1/assets/{id}` - Delete asset (admin only)
- `GET /api/v1/assets/status/{status}` - Assets by status (requires auth)
- `PUT /api/v1/assets/{id}/status` - Update asset status (admin only)
- `POST /api/v1/assets/{id}/depreciation` - Calculate depreciation (admin only)
- `POST /api/v1/assets/{id}/depreciation/record` - Record depreciation (admin only)
- `POST /api/v1/assets/{id}/dispose` - Dispose asset (admin only)
- `POST /api/v1/assets/{id}/write-off` - Write off asset (admin only)
- `POST /api/v1/assets/{id}/maintenance/start` - Start maintenance (admin only)
- `POST /api/v1/assets/{id}/maintenance/end` - End maintenance (admin only)
- `GET /api/v1/assets/{id}/maintenance-records` - Get maintenance records (requires auth)
- `POST /api/v1/assets/maintenance-records` - Create maintenance record (admin only)

### Project (v1)
- `GET /api/v1/projects` - List projects (requires auth, paginated)
- `POST /api/v1/projects` - Create project (admin only)
- `GET /api/v1/projects/{id}` - Get project (requires auth)
- `PUT /api/v1/projects/{id}/status` - Update project status (admin only)
- `GET /api/v1/projects/{project_id}/wbs` - Get WBS items (requires auth)
- `POST /api/v1/projects/wbs` - Create WBS item (admin only)
- `PUT /api/v1/projects/wbs/{id}/progress` - Update WBS progress (admin only)
- `GET /api/v1/projects/{project_id}/costs` - Get project costs (requires auth)
- `POST /api/v1/projects/costs` - Create project cost (admin only)
- `GET /api/v1/projects/{project_id}/profitability` - Get profitability (requires auth)

### Manufacturing (v1)
- `GET /api/v1/manufacturing/work-orders` - List work orders (requires auth, paginated)
- `POST /api/v1/manufacturing/work-orders` - Create work order (admin only)
- `GET /api/v1/manufacturing/work-orders/{id}` - Get work order (requires auth)
- `PUT /api/v1/manufacturing/work-orders/{id}/status` - Update work order status (admin only)
- `GET /api/v1/manufacturing/work-orders/{work_order_id}/operations` - Get operations (requires auth)
- `POST /api/v1/manufacturing/work-orders/operations` - Add operation (admin only)
- `GET /api/v1/manufacturing/work-orders/{work_order_id}/materials` - Get materials (requires auth)
- `POST /api/v1/manufacturing/work-orders/materials` - Add material (admin only)
- `POST /api/v1/manufacturing/boms` - Create BOM (admin only)
- `GET /api/v1/manufacturing/boms/{id}` - Get BOM (requires auth)
- `GET /api/v1/manufacturing/boms/product/{product_id}` - BOMs by product (requires auth)
- `POST /api/v1/manufacturing/boms/lines` - Add BOM line (admin only)
- `GET /api/v1/manufacturing/boms/{bom_id}/lines` - Get BOM lines (requires auth)
- `POST /api/v1/manufacturing/routings` - Create routing (admin only)
- `GET /api/v1/manufacturing/routings/{id}` - Get routing (requires auth)
- `GET /api/v1/manufacturing/routings/product/{product_id}` - Routings by product (requires auth)
- `POST /api/v1/manufacturing/routings/operations` - Add routing operation (admin only)
- `GET /api/v1/manufacturing/material-requirements/{product_id}` - Calculate material requirements (requires auth)

### CRM (v1)
- `GET /api/v1/crm/leads` - List leads (requires auth, paginated)
- `POST /api/v1/crm/leads` - Create lead (admin only)
- `GET /api/v1/crm/leads/{id}` - Get lead (requires auth)
- `GET /api/v1/crm/leads/status/{status}` - Leads by status (requires auth, paginated)
- `PUT /api/v1/crm/leads/{id}/status` - Update lead status (admin only)
- `POST /api/v1/crm/leads/{id}/convert` - Convert lead to customer (admin only)
- `GET /api/v1/crm/opportunities` - List opportunities (requires auth, paginated)
- `POST /api/v1/crm/opportunities` - Create opportunity (admin only)
- `GET /api/v1/crm/opportunities/{id}` - Get opportunity (requires auth)
- `GET /api/v1/crm/opportunities/status/{status}` - Opportunities by status (requires auth, paginated)
- `PUT /api/v1/crm/opportunities/{id}/status` - Update opportunity status (admin only)
- `GET /api/v1/crm/pipeline-value` - Get sales pipeline value (requires auth)
- `GET /api/v1/crm/campaigns` - List campaigns (requires auth, paginated)
- `POST /api/v1/crm/campaigns` - Create campaign (admin only)
- `GET /api/v1/crm/campaigns/{id}` - Get campaign (requires auth)
- `GET /api/v1/crm/campaigns/status/{status}` - Campaigns by status (requires auth, paginated)
- `PUT /api/v1/crm/campaigns/{id}/status` - Update campaign status (admin only)
- `GET /api/v1/crm/tickets` - List tickets (requires auth, paginated)
- `POST /api/v1/crm/tickets` - Create ticket (admin only)
- `GET /api/v1/crm/tickets/{id}` - Get ticket (requires auth)
- `GET /api/v1/crm/tickets/status/{status}` - Tickets by status (requires auth, paginated)
- `GET /api/v1/crm/tickets/open-count` - Get open tickets count (requires auth)
- `PUT /api/v1/crm/tickets/{id}/status` - Update ticket status (admin only)
- `POST /api/v1/crm/tickets/{id}/resolve` - Resolve ticket (admin only)

### Feature Flags (v1)
- `GET /api/v1/feature-flags` - List flags (auth, tenant-isolated, paginated)
- `POST /api/v1/feature-flags` - Create flag (admin only)
- `GET /api/v1/feature-flags/{id}` - Get flag (auth)
- `PUT /api/v1/feature-flags/{id}` - Update flag (admin only)
- `DELETE /api/v1/feature-flags/{id}` - Delete flag (admin only)
- `POST /api/v1/feature-flags/{id}/enable` - Enable flag (admin only)
- `POST /api/v1/feature-flags/{id}/disable` - Disable flag (admin only)
- `GET /api/v1/feature-flags/check/{name}` - Check if enabled (auth)

### Purchase Requests (v1)
- `GET /api/v1/purchase-requests` - List requests (auth, tenant-isolated, paginated, status filter)
- `POST /api/v1/purchase-requests` - Create request (auth)
- `GET /api/v1/purchase-requests/{id}` - Get request (auth)
- `PUT /api/v1/purchase-requests/{id}` - Update request (auth)
- `DELETE /api/v1/purchase-requests/{id}` - Delete request (auth)
- `POST /api/v1/purchase-requests/{id}/submit` - Submit for approval (auth)
- `POST /api/v1/purchase-requests/{id}/approve` - Approve request (admin only)
- `POST /api/v1/purchase-requests/{id}/reject` - Reject request (admin only)

### Custom Fields (v1)
- `POST /api/v1/custom-fields` - Create custom field definition (admin only)
- `GET /api/v1/custom-fields` - List custom field definitions (admin only, optional `?module={module}` filter)
- `GET /api/v1/custom-fields/{id}` - Get custom field definition (admin only)
- `PUT /api/v1/custom-fields/{id}` - Update custom field definition (admin only)
- `DELETE /api/v1/custom-fields/{id}` - Soft delete custom field definition (admin only)

### Settings (v1)
- `POST /api/v1/settings` - Create setting (admin only)
- `GET /api/v1/settings` - List settings (auth, optional `?group={group}` filter, paginated)
- `GET /api/v1/settings/{key}` - Get setting by key (auth)
- `PUT /api/v1/settings/{id}` - Update setting (admin only)
- `DELETE /api/v1/settings/{id}` - Delete setting (admin only)
- `POST /api/v1/settings/bulk` - Bulk update settings (admin only)
- `POST /api/v1/settings/seed` - Seed default settings (admin only)

### Audit Logs (v1)
- `GET /api/v1/audit-logs` - List audit logs (admin only, with filtering and pagination)

### Notifications (v1)
- `POST /api/v1/notifications/send` - Send notification (admin only)
- `GET /api/v1/notifications/in-app` - Get in-app notifications for current user (auth)
- `GET /api/v1/notifications/unread-count` - Get unread notification count (auth)
- `PUT /api/v1/notifications/{id}/read` - Mark notification as read (auth)
- `PUT /api/v1/notifications/read-all` - Mark all notifications as read (auth)
- `POST /api/v1/notifications/{id}/retry` - Retry failed notification (admin only)

### Reports (v1)
- `POST /api/v1/reports/generate` - Generate a report (admin only)
- `GET /api/v1/reports` - List generated reports (auth)
- `GET /api/v1/reports/{id}/download` - Download report file (auth)
- `DELETE /api/v1/reports/{id}` - Delete a report (admin only)

### Events / Outbox (v1)
- `POST /api/v1/events/publish` - Publish a custom domain event (admin only)
- `POST /api/v1/events/outbox/process` - Process pending outbox events (admin only)
- `GET /api/v1/events/outbox/pending` - Get pending outbox events (admin only)
- `GET /api/v1/events/dead-letters` - Get dead letter entries (admin only)
- `POST /api/v1/events/dead-letters/{id}/retry` - Retry a dead letter event (admin only)

### Search (v1)
- `GET /api/v1/search` - Full-text search across entities (auth)
- `POST /api/v1/search/index` - Index a document for search (admin only)
- `DELETE /api/v1/search/{entity_type}/{id}` - Remove document from index (admin only)
- `POST /api/v1/search/reindex` - Reindex all documents for tenant (admin only)


### Tax Engine (v1)
- `GET /api/v1/tax/rates` - List tax rates (auth, paginated, optional type filter)
- `POST /api/v1/tax/rates` - Create tax rate (admin only)
- `GET /api/v1/tax/rates/{id}` - Get tax rate by ID
- `PUT /api/v1/tax/rates/{id}` - Update tax rate (admin only)
- `DELETE /api/v1/tax/rates/{id}` - Delete tax rate (admin only)
- `GET /api/v1/tax/rates/effective/{type}` - Find effective rate for date
- `GET /api/v1/tax/periods` - List tax periods (auth, paginated, optional type filter)
- `POST /api/v1/tax/periods` - Create tax period (admin only)
- `GET /api/v1/tax/periods/{id}` - Get tax period with totals
- `PUT /api/v1/tax/periods/{id}` - Update period status (admin only)
- `POST /api/v1/tax/periods/{id}/details` - Add detail line (admin only)
- `GET /api/v1/tax/periods/{id}/details` - Get detail lines

### Chart of Accounts (v1)
- `GET /api/v1/chart-of-accounts` - List accounts (auth, paginated)
- `POST /api/v1/chart-of-accounts` - Create account (admin only)
- `GET /api/v1/chart-of-accounts/{id}` - Get account by ID
- `PUT /api/v1/chart-of-accounts/{id}` - Update account (admin only)
- `DELETE /api/v1/chart-of-accounts/{id}` - Soft delete account (admin only)
- `GET /api/v1/chart-of-accounts/type/{type}` - Accounts by type
- `GET /api/v1/chart-of-accounts/{id}/children` - Child accounts
- `POST /api/v1/chart-of-accounts/{id}/restore` - Restore soft-deleted (admin only)
- `DELETE /api/v1/chart-of-accounts/{id}/destroy` - Hard delete (super-admin)

### e-Fatura (v1)
- `POST /api/v1/e-fatura` - Create e-Fatura document (admin only)
- `GET /api/v1/e-fatura` - List e-Fatura documents (auth, paginated, status filter)
- `GET /api/v1/e-fatura/{id}` - Get e-Fatura by ID
- `GET /api/v1/e-fatura/uuid/{uuid}` - Get by UUID
- `GET /api/v1/e-fatura/invoice/{invoice_id}` - Get by invoice ID
- `PUT /api/v1/e-fatura/{id}/status` - Update status (admin only)
- `PUT /api/v1/e-fatura/{id}/xml` - Update XML content (admin only)

### e-Defter (v1)
- `POST /api/v1/e-defter/periods` - Create ledger period (admin only)
- `GET /api/v1/e-defter/periods` - List periods (auth, paginated, year/type filters)
- `GET /api/v1/e-defter/periods/{id}` - Get period by ID
- `PUT /api/v1/e-defter/periods/{id}/status` - Update period status (admin only)
- `POST /api/v1/e-defter/entries` - Add Yevmiye entry (admin only)
- `GET /api/v1/e-defter/entries/{period_id}` - Get entries for period
- `PUT /api/v1/e-defter/periods/{id}/berat` - Update berat info (admin only)
- `GET /api/v1/e-defter/periods/{id}/berat` - Get berat info

### Webhooks (v1)
- `GET /api/v1/webhooks` - List subscriptions (auth, paginated)
- `POST /api/v1/webhooks` - Create subscription (admin only)
- `GET /api/v1/webhooks/{id}` - Get subscription by ID
- `PUT /api/v1/webhooks/{id}` - Update subscription (admin only)
- `DELETE /api/v1/webhooks/{id}` - Delete subscription (admin only)
- `GET /api/v1/webhooks/{id}/deliveries` - Delivery history
- `POST /api/v1/webhooks/{id}/retry` - Retry failed delivery (admin only)
- `GET /api/v1/webhooks/events` - List available event types

### Health Check
- `GET /health` - Health check endpoint (alias for readiness)
- `GET /health/live` - Liveness probe (always 200, returns version)
- `GET /health/ready` - Readiness probe (checks DB, returns version + latency + storage)

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
# Visit http://localhost:8000/swagger-ui/
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
| TURERP_DB_MAX_CONNECTIONS | Max DB connections | 10 |
| TURERP_DB_MIN_CONNECTIONS | Min DB connections | 5 |
| TURERP_JWT_ACCESS_EXPIRATION | Access token expiry (seconds) | 3600 |
| TURERP_JWT_REFRESH_EXPIRATION | Refresh token expiry (seconds) | 604800 |
| TURERP_CORS_ORIGINS | Comma-separated allowed origins | * |
| TURERP_CORS_METHODS | Comma-separated allowed methods | GET,POST,PUT,DELETE,OPTIONS |
| TURERP_CORS_HEADERS | Comma-separated allowed headers | Content-Type,Authorization |
| TURERP_CORS_CREDENTIALS | Allow credentials | true |
| TURERP_CORS_MAX_AGE | Preflight cache max age (seconds) | 3600 |
| TURERP_TRUSTED_PROXIES | Comma-separated trusted proxy IPs | (empty) |
| TURERP_RATE_LIMIT_REQUESTS_PER_MINUTE | Rate limit per minute | 10 |
| TURERP_RATE_LIMIT_BURST | Rate limit burst size | 3 |
| TURERP_METRICS_ENABLED | Enable Prometheus metrics | true |
| TURERP_METRICS_PATH | Metrics endpoint path | /metrics |
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
│   │       ├── custom_fields.rs
│   │       ├── settings.rs
│   │       ├── api_keys.rs
│   │       ├── events.rs
│   │       ├── notifications.rs
│   │       ├── reports.rs
│   │       ├── search.rs
│   │       └── audit.rs
│   ├── middleware/
│   │   ├── mod.rs        # Middleware exports
│   │   ├── auth.rs       # JWT authentication + AdminUser/AuthUser extractors
│   │   ├── rate_limit.rs # Rate limiting (with trusted proxy support)
│   │   ├── request_id.rs # Request ID tracking
│   │   ├── audit.rs      # Audit logging (channel-based batch persistence)
│   │   ├── metrics.rs    # Prometheus metrics collection
│   │   ├── tenant.rs     # Tenant context middleware
│   │   ├── idempotency.rs # Idempotency key middleware
│   │   └── api_key.rs    # API key authentication middleware
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
│   │   ├── feature/      # Feature flags domain
│   │   ├── custom_field/ # Custom field definitions domain
│   │   ├── settings/     # Configuration management domain
│   │   ├── api_key/      # API key authentication domain
│   ├── common/
│   │   ├── mod.rs        # Common exports
│   │   ├── pagination.rs # Pagination utilities (PaginatedResult, PaginationParams)
│   │   └── soft_delete.rs # Soft delete trait and types
│   ├── db/
│   │   ├── mod.rs        # DB module
│   │   ├── pool.rs       # Connection pool
│   │   ├── error.rs      # Centralized DB error handling (map_sqlx_error)
│   │   └── tenant_registry.rs # Tenant pool registry
│   ├── i18n/
│   │   ├── mod.rs        # I18n engine (OnceLock, interpolation)
│   │   └── extractor.rs  # Locale Actix extractor
│   └── utils/
│       ├── jwt.rs        # JWT utilities
│       ├── password.rs   # Password utilities
│       └── encryption.rs # AES-256-GCM encryption
├── migrations/
│   ├── 001_initial_schema.sql
│   ├── 002_add_tenant_db_name.sql
│   ├── 003_business_modules.sql
│   ├── 004_composite_indexes.sql
│   ├── 005_audit_logs.sql
│   ├── 006_settings.sql
│   ├── 007_soft_delete.sql
│   └── 008_custom_fields.sql
├── tests/
│   ├── api_integration_test.rs   # Integration tests (38 tests)
│   └── security_test.rs          # Security tests (27 tests)
├── locales/
│   ├── en.json           # English translations
│   └── tr.json           # Turkish translations
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
- [x] Switch all list endpoints (20+ across all modules) to accept `PaginationParams` and return `PaginatedResult`
- [x] PostgreSQL repos use `COUNT(*) OVER()` for efficient total count
- [x] In-memory repos implement skip/take pagination

### 12E: Audit Log Domain & API
- [x] Add `005_audit_logs.sql` with indexes for tenant+created_at, tenant+user_id, tenant+path
- [x] Create audit domain module (model, repository, service, postgres_repository)
- [x] Add `GET /api/v1/audit-logs` endpoint with filtering and pagination (admin-only)
- [x] Rewrite `AuditLoggingMiddleware` with mpsc channel for non-blocking batch persistence
- [x] Spawn background audit writer with 5s flush interval and 100-event buffer