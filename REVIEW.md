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
13. ~~`get_outstanding_invoices` — tum tabloyu RAM'e yukleyip filtreliyor~~ — **Cozuldu (#93)** — `find_outstanding()` repo metodu SQL'e itildi
14. ~~`get_overdue_invoices` — ayni~~ — **Cozuldu (#93)** — `find_overdue()` repo metodu SQL'e itildi
15. ~~`search_invoices` — LIMIT yok~~ — **Yanlis bulgu**, service layer'da LIMIT 100 var, SQL'de LIMIT/OFFSET parametrik

### Mimari (4)
16. ~~`domain/mod.rs` God Module~~ — **Kismen Cozuldu (#93)** — 113 re-export kaldırıldı, 47'ye indi, tam temizlik icin #20'deki eksik domain'ler eklendikten sonra
17. ~~Portal servisler concrete coupling~~ — **Cozuldu (#93)** — `CustomerPortal` + `VendorPortal` trait'leri eklendi
18. `postgres` feature flag compile-time — runtime storage switch gerekli
19. Vault token plain `String` — `secrecy::SecretString` kullanilmali

### Kod Kalitesi (6)
20. ~~`main.rs` duplicate bootstrap~~ — **Cozuldu (#93)** — `macro_rules! build_app_core` ile birleştirildi
21. ~~Duplicate `MessageResponse` — hem `users.rs` hem `common/mod.rs`~~ — **Cozuldu (#108)** — `crate::common::MessageResponse` kullaniliyor, users/tenant/ldap'den lokal tanimlar kaldirildi
22. ~~Startup `.expect()` panics~~ — **Cozuldu (#95)** — `encryption_key_bytes()` ve `create_app_state()` `Result` donuyor
23. ~~173x handler boilerplate — her handler'de `match service.await` tekrari~~ — **Cozuldu (#97)** — `json_resp!` macro ile 111+ handler refactor edildi, net -689 satir
24. ~~`#[allow(unused_imports)]` suppression~~ — **Cozuldu (#100)** — kalan suppression yok
25. ~~`#[allow(dead_code)]` suppressions~~ — **Cozuldu (#100)** — gereksiz suppressions kaldırıldı, sadece 3 yerde DB mapping icin korundu

### Gozlemlenebilirlik (3)
26. Zero `#[tracing::instrument]` — DB query'ler, business logic gorunmez
27. ~~37 domain'de integration test yok~~ — **Cozuldu** — 36 yeni `*_crud_test.rs` dosyasi yazildi, toplam 1921+ test geciyor
28. PostgreSQL path hic test edilmiyor — tum testler in-memory

---

## Medium Bulgular (38) — Onemli ama Acil Degil

### Guvenlik
- ~~Brute-force in-memory (multi-instance calismaz)~~ — **Cozuldu (#103)** — Migration 029 `login_attempts` tablosu, `AuthService` PostgreSQL tabanli kilitleme
- ~~Refresh token revoke edilemiyor~~ — **Cozuldu (#103)** — `RevokedTokenStore` async trait, `InMemoryRevokedTokenStore`, SHA-256 hash ile token revoke, `POST /api/v1/auth/logout` endpoint
- ~~Hardcoded fallback encryption key~~ — **Cozuldu (#103)** — `Config::default()` artik hardcoded key icermiyor, validation bos string ve eski default key'i reddediyor
- ~~CORS `*` + `allow_credentials: true`~~ — **Cozuldu (#94)** — wildcard origin ile credentials zorla `false`

### Performans
- `update_preferences` N+1 bulk upsert
- `SELECT *` document repo'larda (genis tablolar)
- ~~`subdomain.clone()` gereksiz allocation~~ — **Cozuldu (#104)** — `tenant/postgres_repository.rs` ve `tenant/repository.rs`'te 3 clone kaldırıldı, String move yapıldı
- ~~`Vec::new()` yerine `with_capacity`~~ — **Cozuldu (#104)** — 24 yerde `Vec::with_capacity()` eklendi, 18 dosya

### Kod Kalitesi
- Giant `create_in_memory_services!` macro (1000+ satir)
- ~~`api/mod.rs` 70+ manual re-export~~ — **Cozuldu (#104)** — 58 individual re-export gruplandi, `v1/mod.rs` ara katman kaldırıldı, net -79 satır
- `TenantMiddleware` `AuthUser`'a erisiyor (coupling)
- ~~`RateLimitMiddleware` duplicate IP extraction~~ — **Cozuldu (#109)** — `is_loopback`, `is_in_trusted_proxies`, ve `extract_client_ip` `common/ip_utils.rs`'te ortaklasildi, rate_limit.rs + ip_whitelist.rs'ten ~60 satir kaldirildi
- ~~`SearchQuery` her domain'de yeniden implemente~~ — **Cozuldu (#108)** — `PaginatedSearchQuery` `common/pagination.rs`'te ortaklasildi, cari/invoice/products'tan lokal tanimlar kaldirildi
- ~~`jwt.rs` `Unauthorized` yerine `InvalidToken`~~ — **Cozuldu (#104)** — 3 yerde `ApiError::Unauthorized` → `ApiError::InvalidToken`
- ~~`block_on` sync setup'ta~~ — **Cozuldu (#104)** — `background_evaluator.rs`'teki test `#[tokio::test]` async yapıldı, `lib.rs`'teki unavoidable `block_on` açıklama eklendi

### Mimari
- ~~Eksiz PostgreSQL repo'lar (barcode, ip_whitelist, earchive, portal servisler)~~ — **Cozuldu (#102)** — `PostgresBarcodeRepository`, `PostgresIpWhitelistRepository`, `PostgresEarchiveRepository`, `PostgresPortalUserRepository`, `PostgresSupportTicketRepository`, `PostgresVendorUserRepository`, `PostgresDeliveryNoteRepository` implemente edildi, migration 028 eklendi, `lib.rs` wiring tamamlandi
- URL naming tutarsiz (`/cari` singular, `/invoices` plural)
- Search endpoint'ler `?q=` query param olmali
- `encryption_key_bytes()` `.expect()` panic
- `tenant_database_url()` naive string replace
- IP Whitelist JWT'den sonra
- Audit logging auth'dan once
- ~~Idempotency in-memory (scale-out calismaz)~~ — **Cozuldu** — `RedisIdempotencyStore` eklendi, async trait, main.rs'te Redis enabled ise otomatik inject
- `InterCompanyService` `common/`da ama 4 domain'e bagli
- `QualityControlService` yanlis state'te
- `SGK Payroll` concrete `HrService`'e bagli
- `AppState` 60+ `.app_data()` tekrari

### Gozlemlenebilirlik
- Duplicate logging (actix Logger + TracingMiddleware)
- Domain log'lari string interpolation (structured field yok)
- DB error log'larinda tenant_id/user_id context yok
- ~~P99 gauge gercek percentile degil~~ — **Cozuldu (#106)** — Yanlis `gauge!().set(elapsed)` kaldırıldı, `http_request_duration_seconds` histogram üzerinden `compute_percentiles()` ile gercek P99 hesaplanıyor
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
- ~~Duplicate `MessageResponse`~~ — **Cozuldu (#108)** — `crate::common::MessageResponse` kullaniliyor
- ~~`SearchQuery` local vs common~~ — **Cozuldu (#108)** — `PaginatedSearchQuery` `common/pagination.rs`'te ortaklasildi

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
14. [x] ~~173x handler boilerplate generic hale getir~~ — **#97** — `json_resp!` macro ile 111+ handler refactor edildi
15. [x] ~~Postgres feature flag runtime'a cevir~~ — **#98** — `#[cfg(feature = "postgres")]` kaldırıldı, `create_app_state_unified()` runtime seçim yapıyor
16. [x] domain/mod.rs re-export'lari daralt — **#93** — 113 re-export kaldırıldı
17. [x] Portal servisler trait-based hale getir — **#93** — `CustomerPortal` + `VendorPortal` trait'leri eklendi
18. [x] 37 domain icin integration test basla — 36 yeni test dosyasi, tum testler geciyor
19. [x] `#[tracing::instrument]` ekle — **#93** — 16 annotation eklendi (invoice, bank, cari, auth)

### Faz 3: Medium (1-2 hafta)
20. [x] Eksiz PostgreSQL repo'lar implemente et — **Cozuldu (#102)** — 7 repo + migration 028 + lib.rs wiring
21. [x] URL naming standardize et — **Cozuldu (#105)** — `/cari` → `/caris`, 10 utoipa annotation + 7 actix route + 6 test dosyasi
22. [ ] Search endpoint'ler `?q=` query param yap
23. [x] ~~`encryption_key_bytes()` Result donsun~~ — **Cozuldu (#100)** — `Result<[u8; 32], ApiError>` donuyor, `.expect()` yok
24. [x] ~~`tenant_database_url()` parse et~~ — **Cozuldu (#100)** — `url` crate ile proper parsing, query params korunuyor, Result donuyor
25. [x] ~~Idempotency Redis/Postgres backend yap~~ — **Cozuldu** — `RedisIdempotencyStore` implemente edildi, `IdempotencyStore` async trait'e cevrildi, main.rs'te `config.redis.enabled == true` ise Redis backend otomatik inject, yoksa in-memory fallback
26. [x] Audit logging auth'dan sonra tasila — **Cozuldu (#105)** — `JwtAuthMiddleware` → `AuditLoggingMiddleware` sirasi degisti, audit log'lar artik auth claim'leri ile
27. [x] Structured logging (field syntax) — **Cozuldu (#105)** — 13 log donusumu, 4 domain (invoice, bank, cari postgres repo + auth mod.rs)
28. [ ] P99 gauge histogram yap

### Faz 4: Low (2-3 gun)
29. [x] ~~Dead code temizligi~~ — **Cozuldu (#100)** — 202+ satir dead code kaldirildi, 20+ dosya, clippy/format/test/API temiz
30. [ ] Doc comment'lari tamamla
31. [x] README badge AGPL — **Yanlis bulgu**, zaten AGPL-3.0 badge mevcut
32. [ ] `tests/integration/` dizinini kaldir veya doldur

---

*Rapor 5 paralel AI agent ile uretilmistir. Bulgular manuel review onerilir.*
