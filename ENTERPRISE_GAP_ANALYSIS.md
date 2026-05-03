# Turerp ERP - Enterprise Architecture Gap Analysis

> **Tarih:** 2026-05-03
> **Değerlendirme:** Mevcut Production Score ~8.7/10 → Hedef: 9.5+/10

---

## Executive Summary

Mevcut sistem **DDoS koruması, multi-tenancy, audit logging ve temel RBAC** konusunda iyi durumda. Ancak bir **enterprise SaaS ERP** için kritik olan altyapı katmanlarında ciddi eksiklikler var. Bu analiz, sıfırdan kurulması gereken 12 büyük modülü ve 40+ alt özelliği önceliklendirilmiş şekilde listeliyor.

---

## Kritik Eksiklikler (P0 - Üretime Çıkmadan Önce)

### 1. 🏗️ Event-Driven Architecture & Domain Events
**Durum:** ❌ Hiç yok

**Neden Kritik:**
- Billing, Inventory, Accounting arasında **transactional consistency** sağlanmıyor
- Invoice oluşturulduğunda Stock otomatik düşmüyor (manual/synchronous)
- Audit log'lar request düzeyinde; **business event** düzeyinde değil
- External system integration (e-Fatura, e-Defter, banka entegrasyonları) imkansız

**Ne Kurulmalı:**
```
┌─────────────┐    ┌──────────────┐    ┌─────────────────────────────┐
│   Domain    │───▶│   Outbox     │───▶│  Event Bus (Redis Streams)   │
│  Service    │    │   Pattern    │    │  or Apache Kafka/RabbitMQ   │
└─────────────┘    └──────────────┘    └─────────────────────────────┘
                                              │
                    ┌─────────────────────────┼─────────────────────────┐
                    ▼                         ▼                         ▼
              ┌──────────┐            ┌──────────┐            ┌──────────┐
              │  Stock     │            │ Accounting│            │  Email   │
              │ Consumer   │            │ Consumer  │            │ Service  │
              └──────────┘            └──────────┘            └──────────┘
```

**Görevler:**
- [ ] `EventBus` trait (InMemory + Redis Streams)
- [ ] `OutboxPattern` — Events tablosu + background publisher
- [ ] Domain Events: `InvoiceCreated`, `PaymentReceived`, `StockMoved`, `EmployeeHired`
- [ ] Event subscribers: `InvoiceCreated → StockDecrement`, `InvoiceCreated → AccountingEntry`
- [ ] Dead Letter Queue (DLQ) mekanizması

---

### 2. 🗂️ Soft Delete & Data Lifecycle Management
**Durum:** ❌ Hard delete varsayılan

**Neden Kritik:**
- ERP'de **hiçbir veri asla gerçekten silinmemeli**
- Müşteri cari kaydı silindiğinde 5 yıllık fatura geçmişi kaybolabilir
- GDPR/regulatory compliance için `deleted_at` zorunlu
- Yanlışlıkla silinen veriyi geri getirme imkansız

**Ne Kurulmalı:**
```sql
-- Her tabloya eklenecek:
ALTER TABLE cari ADD COLUMN deleted_at TIMESTAMPTZ;
ALTER TABLE cari ADD COLUMN deleted_by BIGINT REFERENCES users(id);

CREATE INDEX idx_cari_deleted_at ON cari(deleted_at) WHERE deleted_at IS NULL;
```

**Tüm domain'lerde:**
- [ ] `deleted_at` / `deleted_by` kolonları
- [ ] Repository `find_all` → `WHERE deleted_at IS NULL`
- [ ] Admin-only "Silinenleri Gör" endpoint'i
- [ ] Bulk restore mekanizması

---

### 3. ⏰ Scheduled Jobs / Background Workers
**Durum:** ❌ Hiç yok (API endpoint'ler tetiklenmeli)

**Neden Kritik:**
- **Depreciation** (Amortisman): Her ay 1'inde otomatik hesaplanmalı
- **Payroll**: Her ay sonu otomatik çalışmalı
- **Invoice reminder**: Vadesi gelen faturalara otomatik email
- **Stock reorder**: Minimum seviyenin altına düşen ürünler için otomatik bildirim
- **Backup/Archive**: Eski audit log'ları archive'e taşıma

**Ne Kurulmalı:**
```rust
#[derive(Clone)]
pub struct JobScheduler {
    redis_conn: redis::aio::Connection,
}

impl JobScheduler {
    pub async fn schedule(&self, job: Job, cron: &str) -> Result<JobId>;
    pub async fn enqueue(&self, job: Job) -> Result<JobId>;
    pub async fn retry(&self, job_id: JobId, delay: Duration) -> Result<()>;
}

pub enum Job {
    CalculateMonthlyDepreciation { tenant_id: i64, month: u8, year: u16 },
    RunPayroll { tenant_id: i64, period: PayrollPeriod },
    SendInvoiceReminders { days_before_due: i64 },
    ArchiveOldAuditLogs { older_than_days: u32 },
    GenerateReports { tenant_id: i64, report_type: ReportType },
}
```

**Teknoloji:** `tokio-cron-scheduler` veya `fang` (PostgreSQL-based job queue)

---

### 4. 📧 Notification Service (Email/SMS/Push)
**Durum:** ❌ Hiç yok

**Neden Kritik:**
- Fatura oluşturulduğunda müşteriye email gitmiyor
- Şifre reset mekanizması yok
- MFA (SMS/TOTP) yok
- Sistem uyarıları (disk full, DB down) yöneticiye gitmiyor

**Ne Kurulmalı:**
- [ ] `NotificationService` trait
- [ ] Email template engine (Handlebars)
- [ ] SMTP integration (configurable: SendGrid/AWS SES/own SMTP)
- [ ] SMS provider abstraction (Twilio, iletisim.gov.tr)
- [ ] Notification queue (async send)
- [ ] In-app notification bell (read/unread)
- [ ] Notification preferences per user

---

### 5. 🔄 Idempotency Keys
**Durum:** ❌ Hiç yok

**Neden Kritik:**
- Network timeout nedeniyle aynı fatura 2 kez oluşabilir
- Payment gateway'e çift çekim
- Retry mekanizmaları olmadan distributed system olmaz

**Ne Kurulmalı:**
```rust
#[derive(Debug, Clone)]
pub struct IdempotencyKey(String);

// Middleware veya handler seviyesinde:
async fn create_invoice(
    idempotency_key: Option<Header<"Idempotency-Key">>, // RFC 7232
    ...
) -> Result<HttpResponse> {
    // Redis'de key→response cache (24h TTL)
    if let Some(cached) = idempotency_store.get(&key).await? {
        return Ok(cached);
    }
    let response = service.create_invoice(...).await?;
    idempotency_store.set(&key, &response, Duration::hours(24)).await?;
    Ok(response)
}
```

---

### 6. 🛡️ API Key Authentication (Service-to-Service)
**Durum:** ❌ Sadece User JWT var

**Neden Kritik:**
- IoT cihazları, mobil app'ler, 3rd party integratörler JWT kullanmamalı
- API Key = Scoped + Rotatable + Auditable
- Rate limiting per API key (tier-based)

**Ne Kurulmalı:**
```rust
pub struct ApiKey {
    pub id: String,           // pk_abc123
    pub tenant_id: i64,
    pub name: String,         // "Production Warehouse Scanner"
    pub scopes: Vec<Scope>,   // ["stock:read", "stock:write"]
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}
```

---

### 7. 📊 Report Engine & Export (PDF/Excel)
**Durum:** ❌ Sadece JSON response

**Neden Kritik:**
- Fatura PDF'i oluşturulamıyor
- Excel export yok (muhasebeciler kullanamaz)
- TRA/Tax reporting formatları (e-Defter, e-Fatura, e-Arşiv)

**Ne Kurulmalı:**
```rust
pub enum ExportFormat {
    Pdf,
    Excel,
    Csv,
    Xml,  // e-Defter/e-Fatura UBL-TR
    Json,
}

trait ReportEngine {
    async fn generate(&self, report: ReportSpec, format: ExportFormat) -> Result<Vec<u8>>;
}
```

**Kütüphaneler:**
- PDF: `printpdf` veya `headless_chrome` (HTML→PDF)
- Excel: `calamine` (read) + `xlsxwriter` (Rust yok, `rust_xlsxwriter`)
- Charts: `plotters`

---

### 8. 🔍 Full-Text Search
**Durum:** ❌ PostgreSQL `ILIKE` ile sınırlı

**Neden Kritik:**
- Cari araması 100K+ kayıtta yavaşlayacak (`%name%` index kullanamaz)
- Ürün barkod/batch/lot araması
- Fuzzy search: "Mehmet Yılmaz" → "Mehmet Yilmaz" eşleşmeli
- "İstanbul" araması "istanbul" bulmalı (Turkish locale)

**Ne Kurulmalı:**
```sql
-- PostgreSQL için:
CREATE EXTENSION IF NOT EXISTS pg_trgm;  -- trigram similarity
CREATE EXTENSION IF NOT EXISTS unaccent;   -- accent-insensitive

-- VEYA
CREATE INDEX idx_cari_search ON cari USING gin(to_tsvector('turkish', name || ' ' || code));
```

**Alternatif:** Meilisearch/Typesense container (ciddi hacimlerde)

---

## Yüksek Öncelik (P1 - Release Sonrası Hemen)

### 9. 💾 Redis Caching Layer
**Durum:** ❌ Her request DB'ye gidiyor

**Etki:** Tenant config, user permissions, product catalog gibi sık okunan veriler cache'lenmeli

```
Read Request → Cache (Redis) → Hit? → Evet: dön
                              └── Hayır: DB → Cache'e yaz → dön
Cache invalidation: Write işlemi sonrası ilgili key'leri sil
```

**Kullanım alanları:**
- [ ] Tenant config (`tenant:{id}:settings`)
- [ ] Feature flags (`tenant:{id}:feature:{name}`)
- [ ] User permissions (`user:{id}:perms`)
- [ ] Product catalog listesi
- [ ] Cari listesi (short cache TTL)

---

### 10. 📨 Change Data Capture (CDC) / Database Triggers
**Durum:** ❌ Uygulama katmanında manuel

**Ne Kurulmalı:**
- PostgreSQL `LISTEN/NOTIFY` ile real-time event streaming
- VEYA `debezium` (binlog-based CDC) → Kafka → Consumers
- Masraf merkezleri arası anlık senkronizasyon
- Reporting DB'ye asynchronous replication

---

### 11. 🌐 Distributed Tracing (OpenTelemetry)
**Durum:** ❌ Sadece `X-Request-ID` var

**Etki:** Bir invoice oluşturma isteği nerede yavaşlıyor görülemiyor.

**Ne Kurulmalı:**
```rust
// Middleware'de:
.with(tracing_opentelemetry::layer())

// Her span'da:
info_span!("invoice.create", invoice.id = %id, tenant.id = %tid)
```

**Stack:** `tracing-opentelemetry` + `jaeger`/`tempo` + `grafana`

---

### 12. 💾 File Upload & Document Management
**Durum:** ❌ Dosya sistemi tamamen yok

**Neden:**
- Fatura PDF'leri
- Personel evrakları (kimlik, diploma)
- Ürün görselleri
- E-imza'lı belgeler

```rust
trait FileStorage {
    async fn upload(&self, tenant_id: i64, file: UploadFile) -> Result<FileUrl>;
    async fn get_presigned_url(&self, file_id: &str, expiry: Duration) -> Result<Url>;
    async fn delete(&self, file_id: &str) -> Result<()>;
}

// Implementations:
// - Local filesystem (dev)
// - S3 / MinIO (prod)
// - Azure Blob / GCS (cloud)
```

---

## Orta Öncelik (P2 - Scale Planı)

### 13. 🔐 Secrets Management (HashiCorp Vault)
**Durum:** ❌ Env var'lar config'de plaintext

**Risk:** `TURERP_JWT_SECRET` environment variable'da, container inspect edilebilir.

**Çözüm:**
```rust
// Vault integration:
let jwt_secret = vault.read_secret("turerp/jwt").await?;
let db_password = vault.read_dynamic_credential("database/creds/erp").await?; // auto-rotate
```

---

### 14. 🏗️ Database Read Replicas
**Durum:** ❌ Single pool

**Etki:** Raporlama sorguları master DB'yi yoruyor.

```rust
pub struct DbRouter {
    master: PgPool,   // writes
    replicas: Vec<PgPool>, // reads (round-robin)
}

// Repository trait'ına hint:
async fn find_all(&self) -> Result<Vec<T>>;   // → replica
async fn create(&self, ...) -> Result<T>;     // → master
```

---

### 15. 🔄 Circuit Breaker & Resilience Patterns
**Durum:** ❌ Hayır (3rd party servis yok ama gelecekte olacak)

**Uygulama:** `tokio-circuit-breaker` veya `backoff` crate'leri ile:
- External API çağrılarında (e-Fatura gateway)
- Email servisinde
- Payment gateway'lerinde

---

### 16. 📈 Advanced Observability (SLI/SLO)
**Durum:** ❌ Sadece Prometheus counter'lar

**Ne Eklenmeli:**
- P99 latency histogram'ları per endpoint
- Error rate alerting thresholds
- Business metrics: "Invoice creation latency", "Payment success rate"
- Grafana dashboards (infrastructure + application + business)
- Alertmanager → PagerDuty/Opsgenie integration

---

## Düşük Öncelik (P3 - Nice-to-Have)

| # | Modül | Durum | Not |
|---|-------|-------|-----|
| 17 | GraphQL API | ❌ | Frontend'in tek endpoint'ten tüm veriyi alması. `async-graphql` |
| 18 | gRPC Service Interface | ❌ | Microservice boundary (günümüzde gerek yok, monolitisiniz) |
| 19 | Multi-region Deployment | ❌ | Active-active replication |
| 20 | Data Warehousing | ❌ | ClickHouse/BigQuery'e analytical event stream |
| 21 | AI/ML Features | ❌ | Sales forecasting, fraud detection, auto-reorder suggestions |
| 22 | Blockchain Ledger (e-ledger) | ❌ | e-Defter için hash-chain (Kanunen zorunlu TR'de!) |

---

## Özet: Öncelik Matrisi

```
                    Etki (Impact)
              Düşük ◄─────────► Yüksek
          ┌─────────────────────────────┐
    Kolay │ P3: GraphQL    │  P1: Redis  │
          │ P3: gRPC       │  P1: CDC    │
          │ P3: Multi-region│ P1: Tracing │
    Zor   ├─────────────────────────────┤
          │ P2: Read Replicas│ P0: Events │
          │ P2: Secrets      │  P0: Soft  │
          │ P2: Circuit      │     Delete │
    Zor   │       Breaker    │  P0: Jobs  │
          └─────────────────────────────┘
```

## Önerilen Sıralama (Roadmap)

```
Sprint N+1:   Soft Delete (tüm domain'lere) + Idempotency Keys
Sprint N+2:   Event Bus (Outbox) + Notification Service (Email)
Sprint N+3:   Background Jobs (Scheduler) + Redis Cache
Sprint N+4:   API Keys + File Upload (S3)
Sprint N+5:   Full-Text Search + Report Engine (PDF/Excel)
Sprint N+6:   OpenTelemetry Tracing + Advanced Monitoring
```

## Sonuç

Mevcut sistem **MVP→Production** geçişini tamamlamış ama **Enterprise SaaS** olmak için **12 kritik altyapı modülü** daha gerekiyor. En kritik 3:

1. **Event-Driven Architecture** — Sistemler konuşsun
2. **Soft Delete** — Veri asla kaybolmasın
3. **Background Jobs** — Zamana bağlı işler otomatikleşsin

Bu 3'ü kurulduğunda architecture score ~9.2/10'a çıkar.
