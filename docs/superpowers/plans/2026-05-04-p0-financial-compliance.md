# P0: Financial Compliance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Turkey-specific financial compliance: e-Fatura (UBL-TR), e-Defter (GIB), Tax Engine + KVB, Chart of Accounts skeleton.

**Architecture:** Domain-First DDD — each module follows existing pattern (model → repository trait → service → postgres_repository → API). Shared GIB gateway trait in common/gov.rs. Tax calculator modules per tax type.

**Tech Stack:** Rust, Actix-web, SQLx, rust_decimal, chrono, utoipa, serde, async-trait

---

## Task 1: Chart of Accounts — Model & Repository

**Files:**
- Create: `src/domain/chart_of_accounts/mod.rs`
- Create: `src/domain/chart_of_accounts/model.rs`
- Create: `src/domain/chart_of_accounts/repository.rs`
- Modify: `src/domain/mod.rs`

- [ ] **Step 1: Create domain module structure**

```bash
mkdir -p src/domain/chart_of_accounts
```

- [ ] **Step 2: Write model.rs with all types**

```rust
//! Chart of Accounts domain models (TEK skeleton)

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::common::SoftDeletable;

/// TEK main account groups
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum AccountGroup {
    DonenVarliklar,                  // 100 - Dönen Varlıklar
    DuranVarliklar,                  // 200 - Duran Varlıklar
    KisaVadeliYabanciKaynaklar,      // 300
    UzunVadeliYabanciKaynaklar,      // 400
    OzKaynaklar,                     // 500
    GelirTablosu,                    // 600-799
    GiderTablosu,                    // 600-799 (separated by sub-group)
}

impl std::fmt::Display for AccountGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountGroup::DonenVarliklar => write!(f, "DonenVarliklar"),
            AccountGroup::DuranVarliklar => write!(f, "DuranVarliklar"),
            AccountGroup::KisaVadeliYabanciKaynaklar => write!(f, "KisaVadeliYabanciKaynaklar"),
            AccountGroup::UzunVadeliYabanciKaynaklar => write!(f, "UzunVadeliYabanciKaynaklar"),
            AccountGroup::OzKaynaklar => write!(f, "OzKaynaklar"),
            AccountGroup::GelirTablosu => write!(f, "GelirTablosu"),
            AccountGroup::GiderTablosu => write!(f, "GiderTablosu"),
        }
    }
}

impl std::str::FromStr for AccountGroup {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DonenVarliklar" => Ok(Self::DonenVarliklar),
            "DuranVarliklar" => Ok(Self::DuranVarliklar),
            "KisaVadeliYabanciKaynaklar" => Ok(Self::KisaVadeliYabanciKaynaklar),
            "UzunVadeliYabanciKaynaklar" => Ok(Self::UzunVadeliYabanciKaynaklar),
            "OzKaynaklar" => Ok(Self::OzKaynaklar),
            "GelirTablosu" => Ok(Self::GelirTablosu),
            "GiderTablosu" => Ok(Self::GiderTablosu),
            _ => Err(format!("Unknown account group: {}", s)),
        }
    }
}

/// Chart of Account entry (TEK hierarchy)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChartAccount {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub group: AccountGroup,
    pub parent_code: Option<String>,
    pub level: u8,
    pub account_type: crate::domain::accounting::model::AccountType,
    pub is_active: bool,
    pub balance: Decimal,
    pub allow_posting: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl SoftDeletable for ChartAccount {
    fn is_deleted(&self) -> bool { self.deleted_at.is_some() }
    fn deleted_at(&self) -> Option<DateTime<Utc>> { self.deleted_at }
    fn deleted_by(&self) -> Option<i64> { self.deleted_by }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChartAccountResponse {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub group: AccountGroup,
    pub parent_code: Option<String>,
    pub level: u8,
    pub account_type: crate::domain::accounting::model::AccountType,
    pub is_active: bool,
    pub balance: Decimal,
    pub allow_posting: bool,
}

impl From<ChartAccount> for ChartAccountResponse {
    fn from(a: ChartAccount) -> Self {
        Self {
            id: a.id, code: a.code, name: a.name, group: a.group,
            parent_code: a.parent_code, level: a.level, account_type: a.account_type,
            is_active: a.is_active, balance: a.balance, allow_posting: a.allow_posting,
        }
    }
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateChartAccount {
    pub code: String,
    pub name: String,
    pub group: AccountGroup,
    pub parent_code: Option<String>,
    pub account_type: crate::domain::accounting::model::AccountType,
    pub allow_posting: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdateChartAccount {
    pub name: Option<String>,
    pub group: Option<AccountGroup>,
    pub is_active: Option<bool>,
    pub allow_posting: Option<bool>,
}

/// Tree node for hierarchical display
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AccountTreeNode {
    pub code: String,
    pub name: String,
    pub group: AccountGroup,
    pub balance: Decimal,
    pub children: Vec<AccountTreeNode>,
}

/// Trial balance entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrialBalanceEntry {
    pub account_code: String,
    pub account_name: String,
    pub debit_balance: Decimal,
    pub credit_balance: Decimal,
}
```

- [ ] **Step 3: Write repository trait**

```rust
//! Chart of Accounts repository trait

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use crate::domain::chart_of_accounts::model::{
    AccountGroup, ChartAccount, ChartAccountResponse, CreateChartAccount, UpdateChartAccount,
};
use crate::common::pagination::{PaginatedResult, PaginationParams};

#[async_trait]
pub trait ChartAccountRepository: Send + Sync {
    async fn create(&self, account: CreateChartAccount, tenant_id: i64) -> Result<ChartAccount, String>;
    async fn find_by_code(&self, code: &str, tenant_id: i64) -> Result<Option<ChartAccount>, String>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<ChartAccount>, String>;
    async fn find_all(&self, tenant_id: i64, group: Option<AccountGroup>, params: PaginationParams) -> Result<PaginatedResult<ChartAccount>, String>;
    async fn find_children(&self, parent_code: &str, tenant_id: i64) -> Result<Vec<ChartAccount>, String>;
    async fn update(&self, id: i64, tenant_id: i64, update: UpdateChartAccount) -> Result<ChartAccount, String>;
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), String>;
    async fn update_balance(&self, id: i64, tenant_id: i64, balance: rust_decimal::Decimal) -> Result<(), String>;
}

pub type BoxChartAccountRepository = Box<dyn ChartAccountRepository>;
```

- [ ] **Step 4: Write mod.rs**

```rust
//! Chart of Accounts domain module (TEK skeleton)

pub mod model;
pub mod repository;
pub mod service;

pub use model::*;
pub use repository::{ChartAccountRepository, BoxChartAccountRepository};
pub use service::ChartOfAccountsService;
```

- [ ] **Step 5: Register in domain/mod.rs**

Add `pub mod chart_of_accounts;` and corresponding re-exports to `src/domain/mod.rs`.

- [ ] **Step 6: Run cargo check**

Run: `cargo check`
Expected: Compiles with warnings about unused service module

- [ ] **Step 7: Commit**

```bash
git add src/domain/chart_of_accounts/ src/domain/mod.rs
git commit -m "feat(chart-of-accounts): add model, repository trait, and domain registration"
```

---

## Task 2: Chart of Accounts — Service & In-Memory Repository

**Files:**
- Create: `src/domain/chart_of_accounts/service.rs`
- Modify: `src/domain/chart_of_accounts/repository.rs` (add InMemory impl)

- [ ] **Step 1: Write InMemoryChartAccountRepository in repository.rs**

Implement `ChartAccountRepository` with `parking_lot::Mutex<HashMap>` following the same pattern as `InMemoryCariRepository`. Use auto-incrementing `AtomicI64` for IDs.

- [ ] **Step 2: Write service.rs**

```rust
//! Chart of Accounts service

use async_trait::async_trait;
use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::chart_of_accounts::model::*;
use crate::domain::chart_of_accounts::repository::ChartAccountRepository;

pub struct ChartOfAccountsService {
    repo: std::sync::Arc<parking_lot::Mutex<dyn ChartAccountRepository + Send + Sync>>,
}

impl ChartOfAccountsService {
    pub fn new(repo: Box<dyn ChartAccountRepository + Send + Sync>) -> Self {
        Self { repo: std::sync::Arc::new(parking_lot::Mutex::new(*repo)) }
    }
}
```

Implement: `create_account`, `get_account`, `list_accounts`, `update_account`, `delete_account`, `get_tree`, `get_children`, `recalculate_balance`, `get_trial_balance`.

- [ ] **Step 3: Write unit tests for service**

Test: create account, hierarchical listing, balance recalculation, soft delete.

- [ ] **Step 4: Run tests**

Run: `cargo test chart_of_accounts`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add src/domain/chart_of_accounts/
git commit -m "feat(chart-of-accounts): add service and in-memory repository"
```

---

## Task 3: Chart of Accounts — API Endpoints & Wiring

**Files:**
- Create: `src/api/v1/chart_of_accounts.rs`
- Modify: `src/api/v1/mod.rs`
- Modify: `src/api/mod.rs` (add OpenAPI paths/schemas)
- Modify: `src/main.rs` (add service to AppState + route)
- Modify: `src/lib.rs` / `src/app.rs` (add service to AppState)

- [ ] **Step 1: Create API endpoints file**

9 endpoints: POST create, GET list, GET by code, PUT update, DELETE, GET tree, GET children, POST recalculate-balance, GET trial-balance. Follow existing pattern from `api/v1/accounting.rs`.

- [ ] **Step 2: Register in api/v1/mod.rs**

Add `pub mod chart_of_accounts;` and `pub use chart_of_accounts::configure as chart_of_accounts_configure;`

- [ ] **Step 3: Add OpenAPI paths and schemas to api/mod.rs**

- [ ] **Step 4: Add ChartOfAccountsService to AppState and main.rs**

- [ ] **Step 5: Run cargo check**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 6: Run all tests**

Run: `cargo test`
Expected: All existing + new tests pass

- [ ] **Step 7: Commit**

```bash
git add src/api/v1/chart_of_accounts.rs src/api/v1/mod.rs src/api/mod.rs src/main.rs
git commit -m "feat(chart-of-accounts): add REST API endpoints and wire into AppState"
```

---

## Task 4: Chart of Accounts — PostgreSQL Repository & Migration

**Files:**
- Create: `src/domain/chart_of_accounts/postgres_repository.rs`
- Create: `migrations/009_chart_of_accounts.sql`
- Modify: `src/domain/chart_of_accounts/mod.rs`

- [ ] **Step 1: Write migration SQL**

```sql
CREATE TABLE chart_accounts (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    code VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    group_name VARCHAR(50) NOT NULL,
    parent_code VARCHAR(50),
    level SMALLINT NOT NULL DEFAULT 1,
    account_type VARCHAR(30) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    balance DECIMAL(19,4) NOT NULL DEFAULT 0,
    allow_posting BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    deleted_by BIGINT
);

CREATE UNIQUE INDEX idx_chart_accounts_tenant_code ON chart_accounts (tenant_id, code) WHERE deleted_at IS NULL;
CREATE INDEX idx_chart_accounts_tenant_group ON chart_accounts (tenant_id, group_name) WHERE deleted_at IS NULL;
CREATE INDEX idx_chart_accounts_tenant_parent ON chart_accounts (tenant_id, parent_code) WHERE deleted_at IS NULL;
CREATE INDEX idx_chart_accounts_tenant_created ON chart_accounts (tenant_id, created_at DESC);
```

- [ ] **Step 2: Write PostgresChartAccountRepository**

Implement all `ChartAccountRepository` trait methods using sqlx queries. Follow the pattern from `domain/accounting/postgres_repository.rs`.

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add src/domain/chart_of_accounts/postgres_repository.rs migrations/009_chart_of_accounts.sql
git commit -m "feat(chart-of-accounts): add PostgreSQL repository and migration"
```

---

## Task 5: Tax Engine — Model & Repository

**Files:**
- Create: `src/domain/tax/mod.rs`
- Create: `src/domain/tax/model.rs`
- Create: `src/domain/tax/repository.rs`
- Modify: `src/domain/mod.rs`

- [ ] **Step 1: Create domain module structure**

```bash
mkdir -p src/domain/tax/calculator
```

- [ ] **Step 2: Write model.rs**

Define: `TaxType` (KDV, OIV, BSMV, Damga, Stopaj, KV, GV), `TaxRate`, `TaxCalculationResult`, `TaxPeriodStatus`, `TaxPeriod`, `TaxPeriodDetail`, `CreateTaxRate`, `UpdateTaxRate`, `CreateTaxPeriod`. All with `ToSchema` derives.

- [ ] **Step 3: Write repository trait**

`TaxRateRepository` (CRUD for rates), `TaxPeriodRepository` (CRUD for KVB periods + details). Both with `Box<dyn>` type aliases.

- [ ] **Step 4: Write mod.rs and register in domain/mod.rs**

- [ ] **Step 5: Run cargo check**

- [ ] **Step 6: Commit**

```bash
git add src/domain/tax/ src/domain/mod.rs
git commit -m "feat(tax): add model and repository trait for tax engine"
```

---

## Task 6: Tax Engine — Calculator Modules

**Files:**
- Create: `src/domain/tax/calculator/mod.rs`
- Create: `src/domain/tax/calculator/kdv.rs`
- Create: `src/domain/tax/calculator/oiv.rs`
- Create: `src/domain/tax/calculator/stopaj.rs`
- Create: `src/domain/tax/calculator/bsmv.rs`
- Create: `src/domain/tax/calculator/damga.rs`

- [ ] **Step 1: Write calculator/mod.rs with TaxCalculator trait**

```rust
pub trait TaxCalculator: Send + Sync {
    fn tax_type(&self) -> TaxType;
    fn calculate(&self, base_amount: Decimal, rate: Decimal, inclusive: bool) -> TaxCalculationResult;
}
```

- [ ] **Step 2: Write each calculator module**

Each implements `TaxCalculator`:
- `kdv.rs` — handles inclusive/exclusive KDV, exemption categories
- `oiv.rs` — %7.5 base rate with categories
- `stopaj.rs` — %15, %10, %0 rates by income type
- `bsmv.rs` — %5 base rate
- `damga.rs` — binde 9.48 (BindeIslem) rate

- [ ] **Step 3: Write unit tests for each calculator**

Test inclusive/exclusive calculation, rounding, edge cases.

- [ ] **Step 4: Run tests**

Run: `cargo test tax::calculator`
Expected: All calculator tests pass

- [ ] **Step 5: Commit**

```bash
git add src/domain/tax/calculator/
git commit -m "feat(tax): add calculator modules for KDV, OIV, stopaj, BSMV, damga"
```

---

## Task 7: Tax Engine — Service, In-Memory Repo & API

**Files:**
- Create: `src/domain/tax/service.rs`
- Modify: `src/domain/tax/repository.rs` (add InMemory impls)
- Create: `src/api/v1/tax.rs`
- Modify: `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`, `src/app.rs`

- [ ] **Step 1: Write InMemoryTaxRateRepository and InMemoryTaxPeriodRepository**

- [ ] **Step 2: Write TaxService with all methods**

Rate CRUD, effective rate lookup (date-based), `calculate_tax`, `calculate_invoice_taxes`, KVB period CRUD, `calculate_period`, `file_period`.

- [ ] **Step 3: Create API endpoints (12 endpoints)**

- [ ] **Step 4: Wire into AppState, OpenAPI, main.rs**

- [ ] **Step 5: Run cargo check + cargo test**

- [ ] **Step 6: Commit**

```bash
git add src/domain/tax/ src/api/v1/tax.rs src/api/ src/main.rs
git commit -m "feat(tax): add service, in-memory repo, and REST API endpoints"
```

---

## Task 8: Tax Engine — PostgreSQL Repository & Migration

**Files:**
- Create: `src/domain/tax/postgres_repository.rs`
- Create: `migrations/010_tax_engine.sql`
- Modify: `src/domain/tax/mod.rs`

- [ ] **Step 1: Write migration SQL**

Tables: `tax_rates` (with date range for effective_from/to), `tax_periods`, `tax_period_details`. Indexes: tenant+tax_type+effective dates, tenant+period year/month.

- [ ] **Step 2: Write PostgresTaxRateRepository and PostgresTaxPeriodRepository**

- [ ] **Step 3: Run cargo check**

- [ ] **Step 4: Commit**

```bash
git add src/domain/tax/postgres_repository.rs migrations/010_tax_engine.sql
git commit -m "feat(tax): add PostgreSQL repository and migration"
```

---

## Task 9: e-Fatura — Model, Repository & GIB Gateway

**Files:**
- Create: `src/domain/efatura/mod.rs`
- Create: `src/domain/efatura/model.rs`
- Create: `src/domain/efatura/repository.rs`
- Create: `src/common/gov.rs`
- Modify: `src/common/mod.rs`
- Modify: `src/domain/mod.rs`

- [ ] **Step 1: Create domain module + ubl sub-module structure**

```bash
mkdir -p src/domain/efatura/ubl
```

- [ ] **Step 2: Write common/gov.rs with GibGateway trait**

```rust
use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GibSendResult {
    pub success: bool,
    pub uuid: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GibStatusResult {
    pub status: String,
    pub response_code: Option<String>,
    pub response_desc: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GibIncomingInvoice {
    pub uuid: String,
    pub sender_vkn: String,
    pub receiver_vkn: String,
    pub issue_date: chrono::DateTime<chrono::Utc>,
    pub xml_content: String,
}

#[async_trait]
pub trait GibGateway: Send + Sync {
    async fn send_invoice(&self, xml: &str, profile: &str) -> Result<GibSendResult, String>;
    async fn check_status(&self, uuid: &str) -> Result<GibStatusResult, String>;
    async fn get_incoming(&self, since: chrono::DateTime<chrono::Utc>, tenant_id: i64) -> Result<Vec<GibIncomingInvoice>, String>;
    async fn cancel(&self, uuid: &str, reason: &str) -> Result<(), String>;
}

pub type BoxGibGateway = Box<dyn GibGateway>;

/// In-memory GIB gateway for testing
pub struct InMemoryGibGateway {
    sent: parking_lot::Mutex<Vec<(String, String, GibSendResult)>>,
}

impl InMemoryGibGateway {
    pub fn new() -> Self { Self { sent: parking_lot::Mutex::new(Vec::new()) } }
}

#[async_trait]
impl GibGateway for InMemoryGibGateway {
    async fn send_invoice(&self, xml: &str, profile: &str) -> Result<GibSendResult, String> {
        let uuid = uuid::Uuid::new_v4().to_string();
        let result = GibSendResult { success: true, uuid: Some(uuid), error_code: None, error_message: None };
        self.sent.lock().push((xml.to_string(), profile.to_string(), result.clone()));
        Ok(result)
    }
    async fn check_status(&self, _uuid: &str) -> Result<GibStatusResult, String> {
        Ok(GibStatusResult { status: "Accepted".to_string(), response_code: None, response_desc: None })
    }
    async fn get_incoming(&self, _since: chrono::DateTime<chrono::Utc>, _tenant_id: i64) -> Result<Vec<GibIncomingInvoice>, String> {
        Ok(Vec::new())
    }
    async fn cancel(&self, _uuid: &str, _reason: &str) -> Result<(), String> { Ok(()) }
}
```

- [ ] **Step 3: Write efatura model.rs**

Define: `EFaturaStatus`, `EFaturaProfile`, `EFatura`, `EFaturaLine`, `PartyInfo`, `AddressInfo`, `TaxSubtotal`, `MonetaryTotal`, `CreateEFatura`, `ValidationResult`.

- [ ] **Step 4: Write repository trait + InMemory impl**

- [ ] **Step 5: Register in domain/mod.rs and common/mod.rs**

- [ ] **Step 6: Run cargo check**

- [ ] **Step 7: Commit**

```bash
git add src/domain/efatura/ src/common/gov.rs src/domain/mod.rs src/common/mod.rs
git commit -m "feat(efatura): add model, repository, and GIB gateway trait"
```

---

## Task 10: e-Fatura — UBL-TR Mapper & Validator

**Files:**
- Create: `src/domain/efatura/ubl/mod.rs`
- Create: `src/domain/efatura/ubl/mapper.rs`
- Create: `src/domain/efatura/ubl/validator.rs`
- Create: `src/domain/efatura/ubl/templates.rs`

- [ ] **Step 1: Write UBL-TR XML templates**

Create XML template strings for UBL-TR Invoice structure following GIB UBL-TR 2.1 specification.

- [ ] **Step 2: Write mapper (Invoice → UBL-TR XML)**

Convert `EFatura` struct to UBL-TR XML string. Handle PartyInfo, tax totals, monetary totals, line items.

- [ ] **Step 3: Write validator (basic structure validation)**

Validate required fields, VKN/TCKN format, tax ID format.

- [ ] **Step 4: Write unit tests for mapper**

Test: Invoice → XML conversion, required field validation, invalid VKN rejection.

- [ ] **Step 5: Run tests**

Run: `cargo test efatura::ubl`
Expected: All UBL tests pass

- [ ] **Step 6: Commit**

```bash
git add src/domain/efatura/ubl/
git commit -m "feat(efatura): add UBL-TR XML mapper and validator"
```

---

## Task 11: e-Fatura — Service, API & Wiring

**Files:**
- Create: `src/domain/efatura/service.rs`
- Create: `src/api/v1/efatura.rs`
- Modify: `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`, `src/app.rs`

- [ ] **Step 1: Write EFaturaService**

Methods: `create_from_invoice`, `generate_ubl`, `validate_ubl`, `send_to_gib`, `check_status`, `process_response`, `cancel_efatura`. Use `GibGateway` trait for external calls.

- [ ] **Step 2: Create API endpoints (7 endpoints)**

- [ ] **Step 3: Wire into AppState, OpenAPI, main.rs**

- [ ] **Step 4: Run cargo check + cargo test**

- [ ] **Step 5: Commit**

```bash
git add src/domain/efatura/ src/api/v1/efatura.rs src/api/ src/main.rs
git commit -m "feat(efatura): add service, REST API, and wire into AppState"
```

---

## Task 12: e-Fatura — PostgreSQL Repository & Migration

**Files:**
- Create: `src/domain/efatura/postgres_repository.rs`
- Create: `migrations/011_efatura.sql`
- Modify: `src/domain/efatura/mod.rs`

- [ ] **Step 1: Write migration SQL**

Tables: `efatura_invoices` (full EFatura fields + xml_content TEXT column), `efatura_lines`. Indexes: tenant+status, tenant+created_at, document_number unique per tenant.

- [ ] **Step 2: Write PostgresEFaturaRepository**

- [ ] **Step 3: Run cargo check**

- [ ] **Step 4: Commit**

```bash
git add src/domain/efatura/postgres_repository.rs migrations/011_efatura.sql
git commit -m "feat(efatura): add PostgreSQL repository and migration"
```

---

## Task 13: e-Defter — Model, Repository & GIB Format

**Files:**
- Create: `src/domain/edefter/mod.rs`
- Create: `src/domain/edefter/model.rs`
- Create: `src/domain/edefter/repository.rs`
- Create: `src/domain/edefter/gib/mod.rs`
- Create: `src/domain/edefter/gib/yevmiye.rs`
- Create: `src/domain/edefter/gib/buyuk_defter.rs`
- Create: `src/domain/edefter/gib/berat.rs`
- Modify: `src/domain/mod.rs`

- [ ] **Step 1: Create domain module + gib sub-module structure**

```bash
mkdir -p src/domain/edefter/gib
```

- [ ] **Step 2: Write model.rs**

Define: `EDefterStatus`, `LedgerType`, `LedgerPeriod`, `YevmiyeEntry`, `YevmiyeLine`, `BeratInfo`, `BalanceCheckResult`, `CreateLedgerPeriod`.

- [ ] **Step 3: Write repository trait + InMemory impl**

`LedgerPeriodRepository`, `YevmiyeEntryRepository`.

- [ ] **Step 4: Write GIB format generators**

- `yevmiye.rs` — Generate GIB-format Yevmiye Defteri XML from YevmiyeEntry records
- `buyuk_defter.rs` — Generate Büyük Defter XML (grouped by account code)
- `berat.rs` — Create and sign berat structure (digest + signature placeholder)

- [ ] **Step 5: Write unit tests for XML generators**

Test: valid XML structure, balance check (debit == credit), empty period handling.

- [ ] **Step 6: Run tests**

Run: `cargo test edefter`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add src/domain/edefter/ src/domain/mod.rs
git commit -m "feat(edefter): add model, repository, and GIB XML format generators"
```

---

## Task 14: e-Defter — Service, API & Wiring

**Files:**
- Create: `src/domain/edefter/service.rs`
- Create: `src/api/v1/edefter.rs`
- Modify: `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`, `src/app.rs`

- [ ] **Step 1: Write EDefterService**

Methods: `create_period`, `populate_from_accounting`, `validate_balance`, `generate_yevmiye_xml`, `generate_buyuk_defter_xml`, `sign_berat`, `send_to_saklayici`, `check_status`.

- [ ] **Step 2: Create API endpoints (9 endpoints)**

- [ ] **Step 3: Wire into AppState, OpenAPI, main.rs**

- [ ] **Step 4: Run cargo check + cargo test**

- [ ] **Step 5: Commit**

```bash
git add src/domain/edefter/ src/api/v1/edefter.rs src/api/ src/main.rs
git commit -m "feat(edefter): add service, REST API, and wire into AppState"
```

---

## Task 15: e-Defter — PostgreSQL Repository & Migration

**Files:**
- Create: `src/domain/edefter/postgres_repository.rs`
- Create: `migrations/012_edefter.sql`
- Modify: `src/domain/edefter/mod.rs`

- [ ] **Step 1: Write migration SQL**

Tables: `ledger_periods`, `yevmiye_entries`, `yevmiye_lines`, `berat_info`. Indexes: tenant+year+month, tenant+status, period_id on entries/lines.

- [ ] **Step 2: Write PostgresLedgerPeriodRepository and PostgresYevmiyeEntryRepository**

- [ ] **Step 3: Run cargo check**

- [ ] **Step 4: Commit**

```bash
git add src/domain/edefter/postgres_repository.rs migrations/012_edefter.sql
git commit -m "feat(edefter): add PostgreSQL repository and migration"
```

---

## Task 16: Cross-Module Integration & Domain Events

**Files:**
- Modify: `src/common/events.rs` (add new domain events)
- Modify: `src/domain/efatura/service.rs` (emit events)
- Modify: `src/domain/edefter/service.rs` (emit events)
- Modify: `src/domain/tax/service.rs` (emit events)

- [ ] **Step 1: Add new domain events to events.rs**

```rust
DomainEvent::EFaturaSent { tenant_id, fatura_id, uuid },
DomainEvent::EDefterPeriodCreated { tenant_id, period_id },
DomainEvent::TaxPeriodCalculated { tenant_id, period_id, tax_type, net_tax },
```

- [ ] **Step 2: Emit events from services after state changes**

- [ ] **Step 3: Wire audit logging for GIB operations**

- [ ] **Step 4: Run cargo check + cargo test**

- [ ] **Step 5: Commit**

```bash
git add src/common/events.rs src/domain/efatura/ src/domain/edefter/ src/domain/tax/
git commit -m "feat(financial-compliance): add domain events and cross-module integration"
```

---

## Summary

| Task | Module | Deliverable |
|------|--------|-------------|
| 1-3 | Chart of Accounts | Model + Service + API (9 endpoints) |
| 4 | Chart of Accounts | PostgreSQL repo + migration |
| 5-7 | Tax Engine | Model + Calculators + Service + API (12 endpoints) |
| 8 | Tax Engine | PostgreSQL repo + migration |
| 9-11 | e-Fatura | Model + UBL-TR + Service + API (7 endpoints) |
| 12 | e-Fatura | PostgreSQL repo + migration |
| 13-14 | e-Defter | Model + GIB XML + Service + API (9 endpoints) |
| 15 | e-Defter | PostgreSQL repo + migration |
| 16 | Integration | Domain events + cross-module wiring |

**Total new endpoints: 37** | **Total new migrations: 4** | **Total new domain modules: 4**