# Uctan Uca Kod Inceleme Raporu

**Tarih:** 2026-05-16
**Proje:** turerp-rust
**Kapsam:** 412 kaynak dosya, ~158K satir kod
**Reviewer'lar:** Guvenlik, Performans, Kod Kalitesi, Mimari, Gozlemlenebilirlik (5 paralel agent)

---

## Executive Summary

| Alan | Critical | High | Medium | Low |
|------|----------|------|--------|-----|
| **Guvenlik** | 1 | 3 | 5 | 2 |
| **Performans** | 4 | 12 | 12 | 6 |
| **Kod Kalitesi** | 0 | 6 | 7 | 3 |
| **Mimari** | 2 | 4 | 8 | 4 |
| **Gozlemlenebilirlik** | 1 | 3 | 6 | 5 |
| **Toplam (benzersiz)** | **8** (3 acik) | **28** (16 acik) | **38** | **20** |

> **Teknik Borc Tahmini:** ~80-120 saat (2-3 hafta, 1 senior developer)

---

## Critical Bulgular (8) — Acil Eylem Gerekli

| # | Kategori | Bulgu | Dosya | Risk |
|---|----------|-------|-------|------|
| 1 | Guvenlik | ~~IP Whitelist IP format validation eksik~~ — **Cozuldu (#94)** | `middleware/ip_whitelist.rs:157` | `std::net::IpAddr::parse()` validation eklendi |
| 2 | Gozlemlenebilirlik | ~~TracingMiddleware RequestId'den ONCE — `request_id` bos string loglaniyor~~ — **Yanlis bulgu** | `main.rs:402-403` | Mevcut siralama dogru; RequestId → Tracing |
| 3 | Mimari | ~~Rate Limit JWT Auth'den SONRA~~ — **Yanlis bulgu** | `main.rs:381` | Mevcut siralama dogru; RateLimit en dista |
| 4 | Performans | `std::fs::create_dir_all` async icinde — tokio worker bloklaniyor | `file_storage.rs:154` | File upload stall | **Cozuldu (#91)** |
| 5 | Performans | `std::fs::write` async icinde — buyuk upload'lar thread bloklar | `file_storage.rs:159` | DoS | **Cozuldu (#91)** |
| 6 | Performans | `std::fs::read` async icinde — download stall | `file_storage.rs:194` | DoS | **Cozuldu (#91)** |
| 7 | Performans | Unbounded invoice `search()` LIMIT yok — OOM | `invoice/postgres_repo.rs:637` | Memory exhaustion | **Cozuldu (#91)** |
| 8 | Guvenlik | Unbounded multipart file upload — size limit yok | `files.rs:36` | Memory DoS | **Cozuldu (#91)** |

---

## High Bulgular (28) — Kisa Vadede Kapanmali

### Guvenlik (3)
1. Login `tenant_id` default = 1 — sistem tenant'ina brute-force (`auth.rs:65`)
2. ~~`/metrics` ve `/swagger-ui` auth'siz~~ — **Yanlis bulgu**, zaten auth arkanda (`AuthUser` extractor + `JwtAuthMiddleware`)
3. ~~Runtime regex derleme loop icinde — reconciliation super-linear yavaslar (`bank/service.rs:566`)~~ **Cozuldu (#91)** — `LazyLock<Regex>` ile compile-time derleme

### Performans (12)
4. ~~N+1: `get_payments_by_cari` — 1 + N query (`invoice/service.rs:230`)~~ **Cozuldu (#91)** — `find_by_invoices()` batch query
5. ~~N+1: `auto_reconcile` — 1 + 4N query (`bank/service.rs:363`)~~ **Cozuldu (#91, #92)** — `buffer_unordered(10)` ile paralel + hata propagate
6. ~~LIMIT eksik: `find_by_tenant` — tum tenant invoices RAM'e yukleniyor~~ **Cozuldu (#91)** — `LIMIT 1000` eklendi
7. ~~LIMIT eksik: `find_by_cari` — tum cari invoices RAM'e~~ **Cozuldu (#91)** — `LIMIT 1000` + `tenant_id` izolasyonu (#92)
8. ~~LIMIT eksik: `find_by_status` — tum status invoices RAM'e~~ **Cozuldu (#91)** — `LIMIT 1000` eklendi
9. ~~LIMIT eksik: `find_deleted` (invoice)~~ **Cozuldu (#91)** — `LIMIT 1000` eklendi
10. ~~LIMIT eksik: `find_by_user` (notification)~~ **Cozuldu (#91)** — `LIMIT 1000` eklendi
11. ~~LIMIT eksik: `find_deleted` (document)~~ **Cozuldu (#91)** — `LIMIT 1000` eklendi
12. ~~LIMIT eksik: `list_versions` (document)~~ **Cozuldu (#91)** — `LIMIT 1000` eklendi
13. `get_outstanding_invoices` — tum tabloyu RAM'e yukleyip filtreliyor
14. `get_overdue_invoices` — ayni
15. ~~`search_invoices` — LIMIT yok~~ — **Yanlis bulgu**, service layer'da LIMIT 100 var, SQL'de LIMIT/OFFSET parametrik

### Mimari (4)
16. `domain/mod.rs` God Module — her subdomain'in internal'larini re-export ediyor
17. Portal servisler concrete coupling — `CustomerPortalService` direkt `Arc<CariService>`
18. `postgres` feature flag compile-time — runtime storage switch gerekli
19. Vault token plain `String` — `secrecy::SecretString` kullanilmali

### Kod Kalitesi (6)
20. `main.rs` duplicate bootstrap — postgres/non-postgres 110+ satir tekrar
21. Duplicate `MessageResponse` — hem `users.rs` hem `common/mod.rs`
22. ~~Startup `.expect()` panics~~ — **Cozuldu (#95)** — `encryption_key_bytes()` ve `create_app_state()` `Result` donuyor
23. 173x handler boilerplate — her handler'de `match service.await` tekrari
24. `#[allow(unused_imports)]` suppression — notifications.rs
25. `#[allow(dead_code)]` — 15+ postgres repo'da

### Gozlemlenebilirlik (3)
26. Zero `#[tracing::instrument]` — DB query'ler, business logic gorunmez
27. 37 domain'de integration test yok — invoice, cari, stock, hr test'siz
28. PostgreSQL path hic test edilmiyor — tum testler in-memory

---

## Medium Bulgular (38) — Onemli ama Acil Degil

### Guvenlik
- Brute-force in-memory (multi-instance calismaz)
- Refresh token revoke edilemiyor
- Hardcoded fallback encryption key
- ~~CORS `*` + `allow_credentials: true`~~ — **Cozuldu (#94)** — wildcard origin ile credentials zorla `false`

### Performans
- `update_preferences` N+1 bulk upsert
- `SELECT *` document repo'larda (genis tablolar)
- `subdomain.clone()` gereksiz allocation
- `Vec::new()` yerine `with_capacity`

### Kod Kalitesi
- Giant `create_in_memory_services!` macro (1000+ satir)
- `api/mod.rs` 70+ manual re-export
- `TenantMiddleware` `AuthUser`'a erisiyor (coupling)
- `RateLimitMiddleware` duplicate IP extraction
- `SearchQuery` her domain'de yeniden implemente
- `jwt.rs` `Unauthorized` yerine `InvalidToken`
- `block_on` sync setup'ta

### Mimari
- Eksiz PostgreSQL repo'lar (barcode, ip_whitelist, earchive, portal servisler)
- URL naming tutarsiz (`/cari` singular, `/invoices` plural)
- Search endpoint'ler `?q=` query param olmali
- `encryption_key_bytes()` `.expect()` panic
- `tenant_database_url()` naive string replace
- IP Whitelist JWT'den sonra
- Audit logging auth'dan once
- Idempotency in-memory (scale-out calismaz)
- `InterCompanyService` `common/`da ama 4 domain'e bagli
- `QualityControlService` yanlis state'te
- `SGK Payroll` concrete `HrService`'e bagli
- `AppState` 60+ `.app_data()` tekrari

### Gozlemlenebilirlik
- Duplicate logging (actix Logger + TracingMiddleware)
- Domain log'lari string interpolation (structured field yok)
- DB error log'larinda tenant_id/user_id context yok
- P99 gauge gercek percentile degil
- Metrics test global OnceLock'e bagimli
- README MIT badge ama Cargo.toml AGPL-3.0

---

## Low Bulgular (20) — Nice to Have

- `tests/integration/` bos dizin
- `println!` forecasting test'lerinde
- `eprintln!` OTLP init hatalarinda
- TracingMiddleware'de tenant_id/user_id yok
- Missing `///` docs public handler'larda
- `unwrap()` GraphQL test'lerinde
- `std::time::Instant` async context'te
- `__TestFileVisibility` artifact prod modulde
- Restore/destroy HTTP method RPC-style
- `SecretsConfig::default()` side effect'li
- Duplicate `MessageResponse`
- `SearchQuery` local vs common

---

## Dogrulanan Guvenlik Ozellikleri

| Alan | Durum |
|------|-------|
| SQL Injection (parametrik query) | Guvenli |
| Tenant Isolation (her query tenant_id filtresi) | Guvenli |
| Password Hashing (bcrypt DEFAULT_COST) | Guvenli |
| API Key Hashing (SHA-256, 189-bit entropy) | Guvenli |
| Encryption at Rest (AES-256-GCM) | Guvenli |
| JWT Validation (exp/aud/iss) | Guvenli |
| Input Validation (validator::Validate) | Guvenli |
| Soft Delete Macro (impl_soft_deletable!) | Guvenli |
| OpenAPI Coverage (724/724 handler) | Tam |
| Test Independence (fresh AppState) | Tam |

---

## Onem Sirasina Gore Eylem Plani

### Faz 1: Critical (1-2 gun)
1. [x] IP Whitelist IP format validation — **#94** — `std::net::IpAddr::parse()` eklendi
2. [x] `std::fs` -> `tokio::fs` file_storage.rs'te — **#91**
3. [x] File upload size limit ekle (50MB) — **#91**
4. [x] Invoice `search()` ve `find_by_tenant` LIMIT ekle — **#91**
5. [x] ~~TracingMiddleware RequestId'den SONRA tasila~~ — **Yanlis bulgu**, mevcut siralama dogru
6. [x] ~~RateLimitMiddleware en disa tasila~~ — **Yanlis bulgu**, zaten en dista

### Faz 2: High (1 hafta)
7. [x] N+1 query'ler JOIN'e cevir (payments, reconcile) — **#91, #92**
8. [x] `get_outstanding/overdue` filtre SQL'e it — **#93** — `find_outstanding`/`find_overdue` repo metodlari
9. [x] Runtime regex pre-compile (bank rules) — **#91**
10. [x] Login default tenant_id kaldir — **#93** — legacy login `unwrap_or(1)` kaldırıldı
11. [x] ~~`/metrics` ve `/swagger-ui` auth altina al~~ — **Yanlis bulgu**, zaten `AuthUser` + `JwtAuthMiddleware` ile korunuyor
12. [x] Vault token `SecretString` — **#93** — `secrecy::SecretString` kullanılıyor
13. [x] `main.rs` duplicate bootstrap coz — **#93** — `macro_rules! build_app_core` ile birleştirildi
14. [ ] 173x handler boilerplate generic hale getir
15. [ ] Postgres feature flag runtime'a cevir
16. [x] domain/mod.rs re-export'lari daralt — **#93** — 113 re-export kaldırıldı
17. [x] Portal servisler trait-based hale getir — **#93** — `CustomerPortal` + `VendorPortal` trait'leri eklendi
18. [ ] 37 domain icin integration test basla
19. [x] `#[tracing::instrument]` ekle — **#93** — 16 annotation eklendi (invoice, bank, cari, auth)

### Faz 3: Medium (1-2 hafta)
20. [ ] Eksiz PostgreSQL repo'lar implemente et
21. [ ] URL naming standardize et
22. [ ] Search endpoint'ler `?q=` query param yap
23. [ ] `encryption_key_bytes()` Result donsun
24. [ ] `tenant_database_url()` parse et
25. [ ] Idempotency Redis/Postgres backend yap
26. [ ] Audit logging auth'dan sonra tasila
27. [ ] Structured logging (field syntax)
28. [ ] P99 gauge histogram yap

### Faz 4: Low (2-3 gun)
29. [ ] Dead code temizligi
30. [ ] Doc comment'lari tamamla
31. [x] README badge AGPL — **Yanlis bulgu**, zaten AGPL-3.0 badge mevcut
32. [ ] `tests/integration/` dizinini kaldir veya doldur

---

*Rapor 5 paralel AI agent ile uretilmistir. Bulgular manuel review onerilir.*
