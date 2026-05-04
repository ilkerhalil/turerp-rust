# P3: Platform & Security Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add platform and security features: RBAC, Dashboard metrics, Multi-currency, 2FA/MFA, Data backup/recovery, Data export.

**Architecture:** RBAC extends existing auth with permission-based access. Dashboard aggregates existing domain metrics. Multi-currency adds exchange rates and conversion. 2FA adds TOTP support. Backup adds snapshot endpoints.

**Tech Stack:** Rust, Actix-web, SQLx, totp-rs, rust_decimal, chrono, utoipa

---

## Task 1: RBAC — Permission Model & Repository

**Files:**
- Create: `src/domain/rbac/mod.rs`
- Create: `src/domain/rbac/model.rs`
- Create: `src/domain/rbac/repository.rs`
- Modify: `src/domain/mod.rs`

- [ ] **Step 1: Write RBAC models**

```rust
/// Fine-grained permission
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Permission {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,          // e.g. "cari:read", "invoice:write"
    pub module: String,        // e.g. "cari", "invoice"
    pub action: String,        // e.g. "read", "write", "delete"
    pub description: String,
}

/// Role with assigned permissions
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Role {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,  // Permission names
    pub is_system: bool,           // System roles can't be deleted
    pub created_at: DateTime<Utc>,
}

/// User-Role assignment
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserRole {
    pub user_id: i64,
    pub role_id: i64,
    pub tenant_id: i64,
    pub assigned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateRole {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AssignRoleRequest {
    pub user_id: i64,
    pub role_name: String,
}
```

- [ ] **Step 2: Write RbacRepository trait**

Methods: `create_permission`, `list_permissions`, `create_role`, `find_role_by_name`, `assign_role`, `revoke_role`, `get_user_permissions`, `check_permission(user_id, permission_name)`.

- [ ] **Step 3: Write InMemoryRbacRepository**

Pre-populate system roles: `admin` (all permissions), `user` (read all), `viewer` (read only), `accountant` (accounting+invoice r/w).

- [ ] **Step 4: Register in domain/mod.rs**

- [ ] **Step 5: Run cargo check + commit**

```bash
git add src/domain/rbac/ src/domain/mod.rs
git commit -m "feat(rbac): add permission model and role-based repository"
```

---

## Task 2: RBAC — Service & Permission Middleware

**Files:**
- Create: `src/domain/rbac/service.rs`
- Create: `src/middleware/permissions.rs`
- Modify: `src/middleware/mod.rs`
- Modify: `src/domain/auth/model.rs` (extend AuthClaims with permissions)

- [ ] **Step 1: Write RbacService**

```rust
pub struct RbacService {
    repo: Arc<dyn RbacRepository>,
}

impl RbacService {
    pub async fn create_role(&self, role: CreateRole, tenant_id: i64) -> Result<Role, String>;
    pub async fn assign_role(&self, user_id: i64, role_name: &str, tenant_id: i64) -> Result<(), String>;
    pub async fn check_permission(&self, user_id: i64, permission: &str, tenant_id: i64) -> Result<bool, String>;
    pub async fn get_user_permissions(&self, user_id: i64, tenant_id: i64) -> Result<Vec<String>, String>;
}
```

- [ ] **Step 2: Write RequirePermission extractor**

```rust
/// Actix extractor that checks user has specific permission
pub struct RequirePermission(pub String);

impl FromRequest for RequirePermission {
    // Extract AuthClaims, query RbacService for permission check
    // Return 403 if permission denied
}
```

- [ ] **Step 3: Extend AuthClaims with permissions field**

Add `permissions: Vec<String>` to `AuthClaims`. Populate on JWT validation from RbacService.

- [ ] **Step 4: Write unit tests**

Test: permission check, role assignment, system roles, 403 on denied permission.

- [ ] **Step 5: Run tests + commit**

```bash
git add src/domain/rbac/ src/middleware/permissions.rs src/middleware/mod.rs
git commit -m "feat(rbac): add service and RequirePermission middleware extractor"
```

---

## Task 3: RBAC — API Endpoints & PostgreSQL

**Files:**
- Create: `src/api/v1/rbac.rs`
- Create: `src/domain/rbac/postgres_repository.rs`
- Create: `migrations/016_rbac.sql`
- Modify: `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`

- [ ] **Step 1: Write API endpoints**

```
POST /api/v1/rbac/roles                    — Create role
GET  /api/v1/rbac/roles                    — List roles
GET  /api/v1/rbac/roles/{name}             — Get role
PUT  /api/v1/rbac/roles/{name}             — Update role permissions
DELETE /api/v1/rbac/roles/{name}           — Delete role (non-system only)
POST /api/v1/rbac/roles/assign             — Assign role to user
POST /api/v1/rbac/roles/revoke             — Revoke role from user
GET  /api/v1/rbac/permissions              — List all permissions
GET  /api/v1/rbac/users/{id}/permissions   — Get user permissions
```

- [ ] **Step 2: Write migration + PostgreSQL repo**

Tables: `permissions`, `roles`, `role_permissions`, `user_roles`. Indexes: tenant+role_name, tenant+user_id.

- [ ] **Step 3: Wire into AppState**

- [ ] **Step 4: Run cargo check + cargo test + commit**

```bash
git add src/api/v1/rbac.rs src/domain/rbac/postgres_repository.rs migrations/016_rbac.sql src/api/ src/main.rs
git commit -m "feat(rbac): add REST API, PostgreSQL repo, and migration"
```

---

## Task 4: Dashboard Metrics API

**Files:**
- Create: `src/domain/dashboard/mod.rs`
- Create: `src/domain/dashboard/service.rs`
- Create: `src/api/v1/dashboard.rs`
- Modify: `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`

- [ ] **Step 1: Write dashboard response models**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardMetrics {
    pub total_revenue: Decimal,
    pub total_expenses: Decimal,
    pub net_profit: Decimal,
    pub outstanding_invoices: u64,
    pub overdue_invoices: u64,
    pub open_sales_orders: u64,
    pub open_purchase_orders: u64,
    pub low_stock_products: u64,
    pub active_employees: u64,
    pub open_tickets: u64,
    pub active_projects: u64,
    pub pending_approvals: u64,
    pub recent_activities: Vec<ActivityItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActivityItem {
    pub id: i64,
    pub entity_type: String,
    pub action: String,
    pub description: String,
    pub performed_by: Option<String>,
    pub created_at: String,
}
```

- [ ] **Step 2: Write DashboardService**

Aggregates data from existing services: `InvoiceService` (revenue, outstanding, overdue), `SalesService` (open orders), `PurchaseService` (open POs), `StockService` (low stock), `HrService` (active employees), `CrmService` (open tickets), `ProjectService` (active projects).

- [ ] **Step 3: Write API endpoint**

```
GET /api/v1/dashboard                    — Get dashboard metrics
GET /api/v1/dashboard/activities         — Get recent activities (paginated)
```

- [ ] **Step 4: Wire and test + commit**

```bash
git add src/domain/dashboard/ src/api/v1/dashboard.rs src/api/ src/main.rs
git commit -m "feat(dashboard): add aggregated metrics and activity feed API"
```

---

## Task 5: Multi-Currency Support

**Files:**
- Create: `src/domain/currency/mod.rs`
- Create: `src/domain/currency/model.rs`
- Create: `src/domain/currency/repository.rs`
- Create: `src/domain/currency/service.rs`
- Create: `src/api/v1/currency.rs`
- Modify: `src/domain/mod.rs`, `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`

- [ ] **Step 1: Write currency models**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Currency {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,              // ISO 4217: "TRY", "USD", "EUR"
    pub name: String,
    pub symbol: String,
    pub is_base: bool,              // Base currency (TRY for Turkish companies)
    pub decimal_places: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExchangeRate {
    pub id: i64,
    pub tenant_id: i64,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub rate_date: chrono::NaiveDate,
    pub source: String,            // "manual", "tcmb", "ecb"
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetExchangeRate {
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub rate_date: chrono::NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CurrencyConversion {
    pub from_amount: Decimal,
    pub from_currency: String,
    pub to_currency: String,
    pub rate: Decimal,
    pub to_amount: Decimal,
    pub rate_date: chrono::NaiveDate,
}
```

- [ ] **Step 2: Write CurrencyService**

`create_currency`, `set_exchange_rate`, `get_rate(date)`, `convert(amount, from, to, date)`, `list_currencies`, `list_rates`.

- [ ] **Step 3: Write API endpoints**

```
POST /api/v1/currency/currencies          — Create currency
GET  /api/v1/currency/currencies          — List currencies
POST /api/v1/currency/rates              — Set exchange rate
GET  /api/v1/currency/rates              — List rates (date, currency filters)
POST /api/v1/currency/convert            — Convert amount
```

- [ ] **Step 4: Wire and test + commit**

```bash
git add src/domain/currency/ src/api/v1/currency.rs src/api/ src/main.rs src/domain/mod.rs
git commit -m "feat(currency): add multi-currency with exchange rates and conversion"
```

---

## Task 6: 2FA/MFA (TOTP) Support

**Files:**
- Create: `src/domain/auth/totp.rs`
- Modify: `src/domain/auth/model.rs` (add totp fields to User)
- Modify: `src/domain/auth/service.rs` (add enable/verify 2FA)
- Modify: `src/api/v1/auth.rs` (add 2FA endpoints)
- Modify: `src/middleware/auth.rs` (verify 2FA on login)

- [ ] **Step 1: Add TOTP fields to User model**

```rust
pub struct User {
    // existing fields...
    pub totp_secret: Option<String>,
    pub totp_enabled: bool,
    pub totp_verified_at: Option<DateTime<Utc>>,
}
```

- [ ] **Step 2: Add totp-rs dependency**

```toml
totp-rs = "5"
qrcode = "0.14"   # For QR code generation
```

- [ ] **Step 3: Write TOTP module**

```rust
pub fn generate_secret() -> String;
pub fn generate_qr_code(secret: &str, email: &str) -> Result<Vec<u8>, String>;
pub fn verify_totp(secret: &str, code: &str) -> bool;
pub fn generate_backup_codes() -> Vec<String>;
```

- [ ] **Step 4: Add 2FA API endpoints**

```
POST /api/v1/auth/2fa/enable             — Enable 2FA (returns QR + backup codes)
POST /api/v1/auth/2fa/verify             — Verify 2FA code during login
POST /api/v1/auth/2fa/disable            — Disable 2FA (requires password)
```

- [ ] **Step 5: Modify login flow**

If user has `totp_enabled`, login returns `2FARequired` status. Client sends code to `/2fa/verify`. On success, issue JWT.

- [ ] **Step 6: Write unit tests**

Test: secret generation, QR code, code verification, backup codes, login with 2FA.

- [ ] **Step 7: Run cargo check + cargo test + commit**

```bash
git add src/domain/auth/ src/api/v1/auth.rs src/middleware/auth.rs Cargo.toml
git commit -m "feat(auth): add TOTP-based 2FA/MFA support"
```

---

## Task 7: Data Backup & Recovery

**Files:**
- Create: `src/common/backup/mod.rs`
- Create: `src/api/v1/backup.rs`
- Modify: `src/common/mod.rs`, `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`

- [ ] **Step 1: Write backup module**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BackupInfo {
    pub id: i64,
    pub tenant_id: i64,
    pub backup_type: String,       // "full", "incremental"
    pub size_bytes: i64,
    pub tables_included: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: i64,
}

pub struct BackupService {
    pool: Option<sqlx::PgPool>,
}

impl BackupService {
    pub async fn create_backup(&self, tenant_id: i64, backup_type: &str) -> Result<BackupInfo, String>;
    pub async fn list_backups(&self, tenant_id: i64) -> Result<Vec<BackupInfo>, String>;
    pub async fn restore_backup(&self, backup_id: i64, tenant_id: i64) -> Result<(), String>;
    pub async fn delete_backup(&self, backup_id: i64, tenant_id: i64) -> Result<(), String>;
}
```

Uses `pg_dump` via shell for PostgreSQL mode. In-memory mode returns placeholder.

- [ ] **Step 2: Write API endpoints**

```
POST /api/v1/backup                     — Create backup
GET  /api/v1/backup                     — List backups
POST /api/v1/backup/{id}/restore        — Restore from backup
DELETE /api/v1/backup/{id}              — Delete backup
```

Admin-only access.

- [ ] **Step 3: Wire and test + commit**

```bash
git add src/common/backup/ src/api/v1/backup.rs src/common/mod.rs src/api/ src/main.rs
git commit -m "feat(backup): add data backup and recovery API"
```

---

## Task 8: Data Export (Table-Level)

**Files:**
- Modify: `src/common/import_export/csv_export.rs` (from P2)
- Create: `src/api/v1/export.rs`
- Modify: `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`

- [ ] **Step 1: Extend DataExporter trait for all entities**

Add exporters: `AccountingExporter`, `HrExporter`, `StockExporter`, `CrmExporter`, `CariExporter`. Each generates CSV from domain service data.

- [ ] **Step 2: Write export API endpoint**

```
GET /api/v1/export/{entity_type}         — Export entity as CSV
  entity_type: cari, products, stock, invoices, sales, purchases, hr, accounting, crm, projects
GET /api/v1/export/{entity_type}/excel   — Export as Excel (future)
```

Query params: `?format=csv&from=2024-01-01&to=2024-12-31&status=active`

- [ ] **Step 3: Wire and test + commit**

```bash
git add src/api/v1/export.rs src/common/import_export/ src/api/ src/main.rs
git commit -m "feat(export): add table-level CSV export for all entities"
```

---

## Summary

| Task | Feature | New Endpoints |
|------|---------|---------------|
| 1-3 | RBAC | 9 |
| 4 | Dashboard | 2 |
| 5 | Multi-Currency | 5 |
| 6 | 2FA/MFA | 3 |
| 7 | Backup/Recovery | 4 |
| 8 | Data Export | 2 |
| **Total** | | **25** |