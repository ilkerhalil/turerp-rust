# Turerp ERP

[![CI](https://github.com/ilkerhalil/turerp-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/ilkerhalil/turerp-rust/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Modern, çok kiracılı (multi-tenant) SaaS ERP sistemi** - Rust, Actix-web ve SQLx ile geliştirilmiştir.

## Özellikler

### 🏢 Core Modüller
| Modül | Açıklama |
|-------|----------|
| **Auth** | JWT tabanlı kimlik doğrulama, bcrypt şifre hashing, token refresh |
| **Tenant** | Subdomain bazlı multi-tenant mimari, tenant izolasyonu |
| **User** | Kullanıcı yönetimi, roller (Admin, User, Viewer) |
| **Cari** | Müşteri/Tedarikçi hesapları, kredi limiti yönetimi |
| **Product** | Ürün katalogu, kategoriler, birim, barkod desteği |
| **Stock** | Depo yönetimi, stok hareketleri, değerleme |
| **Invoice** | Fatura oluşturma, ödeme takibi, vergi hesaplamaları |

### 💼 İş Modülleri
| Modül | Açıklama |
|-------|----------|
| **Sales** | Satış siparişleri, teklifler, fiyatlandırma |
| **Purchase** | Satın alma siparişleri, mal kabul, tedarikçi yönetimi |
| **HR** | Personel yönetimi, bordro, izin takibi |
| **Accounting** | Hesap planı, yevmiye kayıtları, mizan |
| **Assets** | Demirbaşlar, amortisman hesaplama, bakım takibi |

### 📊 Gelişmiş Modüller
| Modül | Açıklama |
|-------|----------|
| **Projects** | Proje yönetimi, WBS, proje maliyetleri, karlılık analizi |
| **Manufacturing** | İş emirleri, rota, üretim takibi |
| **BOM** | Reçete yönetimi, malzeme ihtiyacı hesaplama |
| **Quality Control** | Kalite kontroller, uygunsuzluk raporları (NCR) |
| **CRM** | Potansiyel müşteri, fırsat, kampanya, destek talepleri |

## Teknoloji Stack

| Katman | Teknoloji | Versiyon |
|--------|-----------|----------|
| **Backend** | Rust | 1.70+ |
| **Web Framework** | Actix-web | 4.x |
| **Database** | PostgreSQL | 14+ |
| **ORM** | SQLx (runtime queries) | 0.8 |
| **Auth** | JWT + bcrypt | - |
| **Validation** | validator | 0.16 |
| **Rate Limiting** | governor | 0.8 |
| **Synchronization** | parking_lot (Mutex) | 0.12 |
| **API Docs** | utoipa (OpenAPI/Swagger) | 4.x |
| **Logging** | tracing | 0.1 |

## Hızlı Başlangıç

### Gereksinimler
- Rust 1.70+
- PostgreSQL 14+
- (Opsiyonel) Docker & Docker Compose

### Docker ile Çalıştırma

```bash
cd turerp
docker-compose up -d
# API: http://localhost:8080
# Swagger UI: http://localhost:8080/swagger-ui/
```

### Geliştirme Ortamı

```bash
# Repository'yi klonla
git clone https://github.com/ilkerhalil/turerp-rust.git
cd turerp-rust/turerp

# Build ve çalıştır (in-memory storage - development)
cargo run

# PostgreSQL ile çalıştır (production)
export TURERP_DATABASE_URL="postgres://postgres:postgres@localhost:5432/turerp"
export TURERP_JWT_SECRET="your-secret-key-change-in-production"
cargo run --features postgres

# Testleri çalıştır
cargo test

# PostgreSQL testleri
cargo test --features postgres
```

### Storage Options

| Mode | Command | Use Case |
|------|---------|----------|
| **In-Memory** | `cargo run` | Development, testing |
| **PostgreSQL** | `cargo run --features postgres` | Production |

**Not**: In-memory mod tüm verileri RAM'de tutar. Sunucu yeniden başlatıldığında veriler kaybolur. Production için PostgreSQL kullanın.

### Pre-commit & Pre-push Hooks (Lefthook)

Bu proje, CI başarısızlıklarını önlemek için lefthook kullanır. Her commit ve push işlemlerinde otomatik kontroller çalışır.

**Kurulum:**
```bash
# Lefthook'u kur (tek seferlik)
cargo install lefthook

# Git hook'ları aktifleştir
lefthook install
```

**Çalışan Kontroller:**

| Hook | Komutlar | Açıklama |
|------|----------|----------|
| `pre-commit` | `cargo fmt --check` | Kod formatı kontrolü |
| `pre-commit` | `cargo clippy -- -D warnings` | Lint hataları |
| `pre-push` | `cargo test` | Tüm testler |
| `pre-push` | `cargo audit` | Güvenlik denetimi |
| `commit-msg` | Conventional commits | Commit mesaj formatı |

**Commit Mesaj Formatı:**
```
type(scope): description

# Örnekler:
feat: add rate limiting middleware
fix: auth token validation bug
docs: update README
ci: add lefthook configuration
```

**Types:** feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert

## Proje Yapısı

```
turerp-rust/
├── turerp/                    # Ana uygulama (Rust crate)
│   ├── src/
│   │   ├── api/               # HTTP handlers (Actix-web)
│   │   ├── config/           # Konfigürasyon
│   │   ├── db/                # Database layer (PostgreSQL)
│   │   │   ├── mod.rs
│   │   │   └── pool.rs       # Connection pool, migrations
│   │   ├── domain/           # Domain modülleri (business logic)
│   │   │   ├── auth/          # Kimlik doğrulama
│   │   │   ├── user/         # Kullanıcı yönetimi
│   │   │   │   ├── mod.rs
│   │   │   │   ├── model.rs
│   │   │   │   ├── repository.rs      # InMemory impl
│   │   │   │   ├── postgres_repository.rs  # PostgreSQL impl
│   │   │   │   └── service.rs
│   │   │   ├── tenant/        # Tenant yönetimi
│   │   │   │   └── postgres_repository.rs
│   │   │   ├── cari/          # Cari hesaplar
│   │   │   │   └── postgres_repository.rs
│   │   │   ├── product/       # Ürün yönetimi
│   │   │   ├── stock/         # Stok yönetimi
│   │   │   ├── invoice/       # Fatura yönetimi
│   │   │   ├── sales/         # Satış modülü
│   │   │   ├── purchase/      # Satın alma modülü
│   │   │   ├── hr/            # İK modülü
│   │   │   ├── accounting/    # Muhasebe modülü
│   │   │   ├── assets/        # Demirbaş modülü (NEW)
│   │   │   ├── project/       # Proje yönetimi
│   │   │   ├── manufacturing/ # Üretim modülü
│   │   │   └── crm/           # CRM modülü
│   │   ├── common/
│   │   │   └── pagination.rs  # Sayfalama yardımcıları (NEW)
│   │   ├── middleware/       # HTTP middleware
│   │   │   ├── auth.rs        # JWT authentication
│   │   │   ├── rate_limit.rs  # Rate limiting (governor)
│   │   │   └── request_id.rs  # Request ID tracing
│   │   ├── utils/             # Yardımcı fonksiyonlar
│   │   │   ├── jwt.rs         # JWT utilities
│   │   │   ├── password.rs    # Password utilities
│   │   │   └── encryption.rs  # AES-256-GCM encryption (NEW)
│   │   ├── error/             # Hata yönetimi
│   │   ├── middleware/       # HTTP middleware
│   │   │   ├── auth.rs        # JWT authentication
│   │   │   ├── rate_limit.rs  # Rate limiting (governor)
│   │   │   └── request_id.rs  # Request ID tracing
│   │   ├── utils/             # Yardımcı fonksiyonlar
│   │   ├── lib.rs             # Library entry point
│   │   └── main.rs            # Application entry point
│   ├── migrations/
│   │   └── 001_initial_schema.sql  # Database schema
│   ├── tests/                 # Entegrasyon testleri
│   └── Cargo.toml             # Bağımlılıklar
├── docs/                      # Proje dokümantasyonu
│   └── modules/               # Modül detayları
├── .github/                   # GitHub Actions CI/CD
├── AGENTS.md                  # AI agent konfigürasyonu
├── IMPLEMENTATION_PLAN.md     # Implementasyon planı
└── lefthook.yml               # Pre-commit/pre-push hooks
```

## API Endpoints

### Kimlik Doğrulama (Public - JWT gerekmez)
```
POST /api/auth/register   - Yeni kullanıcı kaydı
POST /api/auth/login      - Giriş (JWT token döner)
POST /api/auth/refresh    - Token yenileme
```

### Kimlik Doğrulama (Protected - JWT gerekir)
```
GET  /api/auth/me         - Aktif kullanıcı bilgisi 🔒
```

### Kullanıcılar (Protected - JWT gerekir)
```
GET    /api/users         - Kullanıcı listesi 🔒
POST   /api/users         - Kullanıcı oluştur 🔒
GET    /api/users/{id}    - Kullanıcı detayı 🔒
PUT    /api/users/{id}    - Kullanıcı güncelle 🔒
DELETE /api/users/{id}    - Kullanıcı sil 🔒
```

🔒 = JWT Bearer token gerekir

### Swagger UI
- **Swagger UI**: `http://localhost:8000/swagger-ui/`
- **OpenAPI Spec**: `http://localhost:8000/api-docs/openapi.json`

**Not**: Swagger UI'da "Authorize" butonuna tıklayarak Bearer token girebilirsiniz.

## Mimari

### Multi-Tenant Akışı
```
İstek → Subdomain Tespiti → Tenant Lookup → DB Routing → API Yanıtı
   ↓
JWT Token → Kullanıcı Doğrulama → Rol Bazlı Erişim
```

### Modül Bağımlılıkları
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

## Test

```bash
# Tüm testler (239 test)
cargo test

# Security testleri
cargo test --test security_test

# Belirli modül testleri
cargo test --lib domain::cari

# Test coverage
cargo tarpaulin --out Html
```

## CI/CD

GitHub Actions ile otomatik:
- ✅ Format kontrolü (`cargo fmt --check`)
- ✅ Clippy linting (`cargo clippy`)
- ✅ Test çalıştırma (`cargo test`)
- ✅ Security audit (`cargo audit`)

## Environment Variables

### Zorunlu Değişkenler

| Değişken | Açıklama |
|----------|---------|
| `TURERP_DATABASE_URL` | PostgreSQL connection string (örn: `postgres://user:pass@host:5432/db`) |
| `TURERP_JWT_SECRET` | JWT imzalama anahtarı (production'da min 32 karakter, güvenli rastgele string) |

### Opsiyonel Değişkenler

| Değişken | Açıklama | Varsayılan |
|----------|---------|------------|
| `TURERP_ENV` | Ortam (`development` / `production`) | `development` |
| `TURERP_SERVER_HOST` | Sunucu host | `0.0.0.0` |
| `TURERP_SERVER_PORT` | Sunucu portu | `8000` |
| `TURERP_DB_MAX_CONNECTIONS` | Max DB bağlantısı | `10` |
| `TURERP_DB_MIN_CONNECTIONS` | Min DB bağlantısı | `5` |
| `TURERP_JWT_ACCESS_EXPIRATION` | Access token süresi (saniye) | `3600` (1 saat) |
| `TURERP_JWT_REFRESH_EXPIRATION` | Refresh token süresi (saniye) | `604800` (7 gün) |
| `TURERP_CORS_ORIGINS` | İzin verilen origins (virgülle ayrılmış) | `*` |
| `TURERP_CORS_METHODS` | İzin verilen HTTP metodları | `GET,POST,PUT,DELETE,OPTIONS` |
| `TURERP_CORS_HEADERS` | İzin verilen headerlar | `Content-Type,Authorization` |
| `TURERP_CORS_CREDENTIALS` | CORS credentials | `true` |
| `RUST_LOG` | Log seviyesi | `info` |

## Güvenlik

### OWASP Top 10 Koruması

Sistem OWASP Top 10 güvenlik açıklarına karşı test edilmiştir:
- ✅ **SQL Injection Prevention** - Parametreli sorgular
- ✅ **JWT Token Security** - Token doğrulama ve manipülasyon koruması
- ✅ **Authentication Security** - Güçlü şifre politikaları, rate limiting
- ✅ **Authorization** - Rol bazlı erişim kontrolü
- ✅ **Input Validation** - Tüm girişler doğrulanıyor
- ✅ **HTTP Method Security** - İzin verilmeyen metodlar reddediliyor

### JWT Kimlik Doğrulama

Tüm API endpoint'leri (auth hariç) JWT Bearer token gerektirir:

```bash
# Token al
curl -X POST http://localhost:8000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"password"}'

# Authenticated istek
curl http://localhost:8000/api/users \
  -H "Authorization: Bearer <access_token>"
```

### Rate Limiting

Auth endpoint'leri rate limiting ile korunur:
- **Limit**: 10 request/dakika (per IP)
- **Burst**: 3 request

### Şifre Gereksinimleri

Şifreler aşağıdaki kriterleri karşılamalıdır:
- Minimum 12 karakter
- En az 1 büyük harf
- En az 1 küçük harf
- En az 1 rakam
- En az 1 özel karakter

### Production Uyarıları

Production ortamında (`TURERP_ENV=production`):
- JWT secret minimum 32 karakter olmalı
- JWT secret "dev", "test", "password" gibi zayıf patternler içermemeli
- CORS wildcard (`*`) kullanılmaması önerilir

### Tenant İzolasyonu

Her tenant kendi veritabanına sahip ve JWT token'dan gelen `tenant_id` ile izole edilmiştir. Kullanıcılar sadece kendi tenant'larının verilerine erişebilir.

## Katkıda Bulunma

1. Fork yapın
2. Feature branch oluşturun (`git checkout -b feature/amazing-feature`)
3. Değişikliklerinizi commit edin (`git commit -m 'feat: amazing feature'`)
4. Branch'i push edin (`git push origin feature/amazing-feature`)
5. Pull Request açın

## Dokümantasyon

- [Modül Dokümantasyonu](docs/README.md) - Tüm modüllerin detaylı açıklaması
- [Implementasyon Planı](IMPLEMENTATION_PLAN.md) - Geliştirme road map
- [Detaylı README](turerp/README.md) - Ana uygulama dokümantasyonu

## Lisans

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

**Turerp Team** - Modern ERP çözümleri için Rust ile geliştirilmiştir.