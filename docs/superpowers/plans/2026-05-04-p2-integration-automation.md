# P2: Integration & Automation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add integration and automation: Webhook system, Bank reconciliation, CSV/EDI import, 3rd-party ERP connectors, Real email sending.

**Architecture:** New `webhook` domain module for event-driven notifications. Bank reconciliation extends accounting. CSV import via common/import trait. Email via SMTP integration using existing NotificationService trait.

**Tech Stack:** Rust, Actix-web, SQLx, reqwest (SMTP/HTTP), csv crate, rust_decimal

---

## Task 1: Webhook System — Model & Repository

**Files:**
- Create: `src/domain/webhook/mod.rs`
- Create: `src/domain/webhook/model.rs`
- Create: `src/domain/webhook/repository.rs`
- Modify: `src/domain/mod.rs`

- [ ] **Step 1: Write webhook models**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Webhook {
    pub id: i64,
    pub tenant_id: i64,
    pub url: String,
    pub secret: String,           // HMAC signing key
    pub events: Vec<String>,      // ["invoice.created", "payment.received"]
    pub is_active: bool,
    pub retry_count: u32,
    pub timeout_ms: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookDelivery {
    pub id: i64,
    pub tenant_id: i64,
    pub webhook_id: i64,
    pub event: String,
    pub payload: String,
    pub status: DeliveryStatus,
    pub response_code: Option<u16>,
    pub response_body: Option<String>,
    pub attempts: u32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

pub enum DeliveryStatus {
    Pending, Sent, Failed, Retrying, Abandoned,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateWebhook {
    pub url: String,
    pub events: Vec<String>,
    pub retry_count: Option<u32>,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookDeliveryResponse {
    pub id: i64,
    pub webhook_id: i64,
    pub event: String,
    pub status: String,
    pub attempts: u32,
    pub created_at: String,
}
```

- [ ] **Step 2: Write WebhookRepository trait**

Methods: `create`, `find_by_id`, `find_all_active`, `find_by_event`, `update`, `soft_delete`. Delivery: `create_delivery`, `mark_sent`, `mark_failed`, `find_pending_retries`.

- [ ] **Step 3: Write InMemoryWebhookRepository**

- [ ] **Step 4: Register in domain/mod.rs**

- [ ] **Step 5: Run cargo check + commit**

```bash
git add src/domain/webhook/ src/domain/mod.rs
git commit -m "feat(webhook): add model and repository for webhook system"
```

---

## Task 2: Webhook System — Service & Event Dispatch

**Files:**
- Create: `src/domain/webhook/service.rs`
- Create: `src/domain/webhook/dispatcher.rs`
- Modify: `src/common/events.rs` (add webhook dispatch hook)

- [ ] **Step 1: Write WebhookService**

```rust
pub struct WebhookService {
    repo: Arc<dyn WebhookRepository>,
    http_client: reqwest::Client,
}

impl WebhookService {
    pub async fn register(&self, webhook: CreateWebhook, tenant_id: i64) -> Result<Webhook, String>;
    pub async fn dispatch_event(&self, event: &str, payload: &serde_json::Value, tenant_id: i64) -> Result<Vec<WebhookDelivery>, String>;
    pub async fn retry_failed(&self) -> Result<u64, String>;
    pub async fn list_deliveries(&self, webhook_id: i64, tenant_id: i64) -> Result<Vec<WebhookDeliveryResponse>, String>;
}
```

- [ ] **Step 2: Write dispatcher with HMAC signing**

Sign payloads with `HMAC-SHA256` using webhook secret. Add signature header: `X-Turerp-Signature`. Handle retries with exponential backoff.

- [ ] **Step 3: Integrate with EventBus**

Add a `WebhookSubscriber` that listens to `DomainEvent`s and dispatches matching webhooks. Register in AppState alongside existing subscribers.

- [ ] **Step 4: Write unit tests**

Test: webhook creation, HMAC signing, event matching, retry logic.

- [ ] **Step 5: Run tests + commit**

```bash
git add src/domain/webhook/
git commit -m "feat(webhook): add service with HMAC signing and event dispatch"
```

---

## Task 3: Webhook System — API Endpoints & PostgreSQL

**Files:**
- Create: `src/api/v1/webhooks.rs`
- Create: `src/domain/webhook/postgres_repository.rs`
- Create: `migrations/015_webhooks.sql`
- Modify: `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`

- [ ] **Step 1: Write API endpoints**

```
POST /api/v1/webhooks                     — Register webhook
GET  /api/v1/webhooks                     — List webhooks
GET  /api/v1/webhooks/{id}                — Get webhook
PUT  /api/v1/webhooks/{id}                — Update webhook
DELETE /api/v1/webhooks/{id}              — Delete webhook
POST /api/v1/webhooks/{id}/test           — Test webhook
GET  /api/v1/webhooks/{id}/deliveries     — List deliveries
POST /api/v1/webhooks/retry-failed        — Retry failed deliveries
```

- [ ] **Step 2: Write migration + PostgreSQL repo**

- [ ] **Step 3: Wire into AppState**

- [ ] **Step 4: Run cargo check + cargo test + commit**

```bash
git add src/api/v1/webhooks.rs src/domain/webhook/postgres_repository.rs migrations/015_webhooks.sql src/api/ src/main.rs
git commit -m "feat(webhook): add REST API, PostgreSQL repo, and test endpoint"
```

---

## Task 4: Bank Reconciliation

**Files:**
- Create: `src/domain/bank_reconciliation/mod.rs`
- Create: `src/domain/bank_reconciliation/model.rs`
- Create: `src/domain/bank_reconciliation/repository.rs`
- Create: `src/domain/bank_reconciliation/service.rs`
- Create: `src/api/v1/bank_reconciliation.rs`
- Modify: `src/domain/mod.rs`, `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`

- [ ] **Step 1: Write bank reconciliation models**

```rust
pub struct BankStatement {
    pub id: i64,
    pub tenant_id: i64,
    pub bank_account: String,
    pub iban: Option<String>,
    pub statement_date: chrono::NaiveDate,
    pub opening_balance: Decimal,
    pub closing_balance: Decimal,
    pub currency: String,
    pub lines: Vec<BankStatementLine>,
    pub status: ReconciliationStatus,
}

pub struct BankStatementLine {
    pub id: i64,
    pub statement_id: i64,
    pub transaction_date: chrono::NaiveDate,
    pub description: String,
    pub reference: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub balance: Decimal,
    pub matched_journal_entry_id: Option<i64>,
}

pub enum ReconciliationStatus {
    Imported, InProgress, Matched, Completed,
}

pub struct ReconciliationMatch {
    pub statement_line_id: i64,
    pub journal_entry_id: i64,
    pub match_type: MatchType,
    pub confidence: f64,
}

pub enum MatchType {
    Exact,    // Amount + date exact match
    Fuzzy,    // Amount match, date within 3 days
    Manual,   // User-matched
}
```

- [ ] **Step 2: Write BankReconciliationService**

`import_statement`, `auto_match`, `manual_match`, `complete_reconciliation`, `get_unmatched_lines`.

- [ ] **Step 3: Write API endpoints**

```
POST /api/v1/bank-reconciliation/statements       — Import bank statement
GET  /api/v1/bank-reconciliation/statements         — List statements
GET  /api/v1/bank-reconciliation/statements/{id}    — Get statement
POST /api/v1/bank-reconciliation/statements/{id}/auto-match — Auto-match
POST /api/v1/bank-reconciliation/statements/{id}/manual-match — Manual match
POST /api/v1/bank-reconciliation/statements/{id}/complete — Complete reconciliation
GET  /api/v1/bank-reconciliation/statements/{id}/unmatched — Unmatched lines
```

- [ ] **Step 4: Wire and test + commit**

```bash
git add src/domain/bank_reconciliation/ src/api/v1/bank_reconciliation.rs src/api/ src/main.rs src/domain/mod.rs
git commit -m "feat(bank-reconciliation): add bank statement import and matching"
```

---

## Task 5: CSV/EDI Import System

**Files:**
- Create: `src/common/import_export/mod.rs`
- Create: `src/common/import_export/csv_import.rs`
- Create: `src/common/import_export/csv_export.rs`
- Create: `src/api/v1/import.rs`
- Modify: `src/common/mod.rs`, `src/api/v1/mod.rs`, `src/api/mod.rs`, `src/main.rs`

- [ ] **Step 1: Write common import/export module**

```rust
pub trait DataImporter: Send + Sync {
    fn supported_entity(&self) -> &str;
    fn import_csv(&self, csv_content: &str, tenant_id: i64) -> Result<ImportResult, String>;
}

pub struct ImportResult {
    pub total_rows: u64,
    pub imported: u64,
    pub skipped: u64,
    pub errors: Vec<ImportError>,
}

pub struct ImportError {
    pub row: u64,
    pub field: String,
    pub message: String,
}

pub trait DataExporter: Send + Sync {
    fn supported_entity(&self) -> &str;
    fn export_csv(&self, tenant_id: i64, filters: Option<serde_json::Value>) -> Result<String, String>;
}
```

- [ ] **Step 2: Write CSV importers for key entities**

`CariCsvImporter`, `ProductCsvImporter`, `InvoiceCsvImporter`. Parse CSV with headers, validate each row, batch insert.

- [ ] **Step 3: Write CSV exporters**

Export any entity list to CSV format using `serde_csv` or manual formatting.

- [ ] **Step 4: Add API endpoints**

```
POST /api/v1/import/csv             — Import CSV (multipart, entity_type param)
GET  /api/v1/export/csv             — Export CSV (entity_type, filters)
POST /api/v1/import/csv/validate    — Validate CSV without importing
```

- [ ] **Step 5: Wire and test + commit**

```bash
git add src/common/import_export/ src/api/v1/import.rs src/common/mod.rs src/api/ src/main.rs
git commit -m "feat(import-export): add CSV import/export for cari, product, invoice"
```

---

## Task 6: Real Email Sending (SMTP Integration)

**Files:**
- Create: `src/common/notifications/smtp.rs`
- Modify: `src/common/notifications/mod.rs`
- Modify: `src/common/mod.rs` (add SmtpNotificationService export)

- [ ] **Step 1: Write SmtpNotificationService**

```rust
pub struct SmtpNotificationService {
    config: SmtpConfig,
    inner: InMemoryNotificationService,  // delegates in-app notifications
}

pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub from_name: String,
    pub use_tls: bool,
}
```

Implement `NotificationService` trait. For `Email` channel: use `lettre` crate to send via SMTP. For `InApp` channel: delegate to InMemoryNotificationService. For `Sms` channel: log placeholder.

- [ ] **Step 2: Add lettre dependency to Cargo.toml**

```toml
lettre = { version = "0.11", features = ["tokio1-rustls-tls", "smtp-transport", "builder"] }
```

- [ ] **Step 3: Add SMTP config to Config struct**

`TURERP_SMTP_HOST`, `TURERP_SMTP_PORT`, `TURERP_SMTP_USER`, `TURERP_SMTP_PASS`, `TURERP_SMTP_FROM`.

- [ ] **Step 4: Wire into AppState**

If SMTP config is present, use `SmtpNotificationService`. Otherwise fall back to `InMemoryNotificationService`.

- [ ] **Step 5: Write unit tests (with mock SMTP)**

- [ ] **Step 6: Run cargo check + cargo test + commit**

```bash
git add src/common/notifications/smtp.rs src/common/notifications/mod.rs Cargo.toml src/config.rs
git commit -m "feat(notifications): add SMTP email sending via lettre crate"
```

---

## Task 7: 3rd Party ERP Connector Framework

**Files:**
- Create: `src/common/connectors/mod.rs`
- Create: `src/common/connectors/erp_connector.rs`
- Create: `src/common/connectors/logo_connector.rs`
- Create: `src/api/v1/connectors.rs`
- Modify: `src/common/mod.rs`, `src/api/v1/mod.rs`

- [ ] **Step 1: Write ERP connector trait**

```rust
#[async_trait]
pub trait ErpConnector: Send + Sync {
    fn name(&self) -> &str;
    async fn test_connection(&self) -> Result<bool, String>;
    async fn sync_cari(&self, tenant_id: i64) -> Result<SyncResult, String>;
    async fn sync_products(&self, tenant_id: i64) -> Result<SyncResult, String>;
    async fn sync_invoices(&self, tenant_id: i64) -> Result<SyncResult, String>;
    async fn get_sync_status(&self, tenant_id: i64) -> Result<SyncStatus, String>;
}

pub struct SyncResult {
    pub created: u64,
    pub updated: u64,
    pub failed: u64,
    pub errors: Vec<String>,
}

pub struct SyncStatus {
    pub last_sync: Option<DateTime<Utc>>,
    pub status: String,
    pub entities_synced: Vec<String>,
}
```

- [ ] **Step 2: Write Logo ERP connector (stub)**

Logo connector implementing `ErpConnector`. Stub methods that return placeholder data — actual REST API calls depend on Logo configuration.

- [ ] **Step 3: Add API endpoints**

```
GET  /api/v1/connectors                    — List available connectors
POST /api/v1/connectors/{name}/test        — Test connector connection
POST /api/v1/connectors/{name}/sync        — Run sync
GET  /api/v1/connectors/{name}/status      — Get sync status
```

- [ ] **Step 4: Wire and test + commit**

```bash
git add src/common/connectors/ src/api/v1/connectors.rs src/common/mod.rs src/api/ src/main.rs
git commit -m "feat(connectors): add 3rd-party ERP connector framework with Logo stub"
```

---

## Summary

| Task | Feature | New Endpoints |
|------|---------|---------------|
| 1-3 | Webhook System | 8 |
| 4 | Bank Reconciliation | 7 |
| 5 | CSV Import/Export | 3 |
| 6 | SMTP Email | 0 (extends existing) |
| 7 | ERP Connectors | 4 |
| **Total** | | **22** |