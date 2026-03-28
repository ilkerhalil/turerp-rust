# Turerp ERP

[![CI](https://github.com/turerp/turerp-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/turerp/turerp-rust/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-99%20passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](#lisans)

**Multi-tenant SaaS ERP sistemi** - Rust, Actix-web ve SQLx ile geliştirilmiştir.

## Özellikler

### Core Modüller
- **Kimlik Doğrulama**: JWT tabanlı auth, bcrypt şifre hashing, token refresh
- **Tenant Yönetimi**: Subdomain bazlı multi-tenant mimari
- **Kullanıcı Yönetimi**: Roller (Admin, User, Viewer), CRUD işlemleri
- **Cari**: Müşteri/Tedarikçi hesapları, kredi limiti yönetimi
- **Ürünler**: Kategori yönetimi, birim, barkod desteği
- **Stok**: Depo yönetimi, stok hareketleri, değerleme
- **Faturalar**: Fatura oluşturma, ödeme takibi, vergi hesaplamaları

### İş Modülleri
- **Satış**: Siparişler, teklifler, fiyatlandırma
- **Satın Alma**: Siparişler, mal kabul, tedarikçi yönetimi
- **İK**: Personel yönetimi, bordro, izin takibi
- **Muhasebe**: Hesap planı, yevmiye kayıtları, mizan

### Gelişmiş Modüller
- **Proje Yönetimi**: WBS, proje maliyetleri, karlılık analizi
- **Üretim**: İş emirleri, rota, BOM yönetimi
- **Kalite Kontrol**: İncelemeler, uygunsuzluk raporları (NCR)
- **CRM**: Potansiyel müşteri, fırsat, kampanya yönetimi

## Teknoloji Stack

| Katman | Teknoloji |
|--------|-----------|
| Web Framework | Actix-web 4 |
| Database | PostgreSQL + SQLx |
| Auth | JWT + bcrypt |
| Validation | validator |
| Serialization | serde |
| Error Handling | thiserror + anyhow |
| Logging | tracing |
| API Docs | utoipa (OpenAPI/Swagger) |

## Kurulum

### Gereksinimler
- Rust 1.70+
- PostgreSQL 14+
- (Opsiyonel) Docker & Docker Compose

### Hızlı Başlangıç (Docker)

```bash
cd turerp
docker-compose up -d
# API: http://localhost:8080
# Swagger UI: http://localhost:8080/swagger-ui/
```

> **Not**: Docker port 8080, local development port 8000 kullanır.

### Geliştirme Ortamı

```bash
# Rust kurulumu (rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# PostgreSQL veritabanı oluştur
createdb turerp

# Environment variables ayarla
export TURERP_DATABASE_URL="postgres://postgres:postgres@localhost:5432/turerp"
export TURERP_JWT_SECRET="your-secret-key-change-in-production"

# Repository'yi klonla
git clone https://github.com/turerp/turerp-rust.git
cd turerp-rust/turerp

# Bağımlılıkları yükle ve build et
cargo build

# Testleri çalıştır
cargo test

# Sunucuyu başlat
cargo run
# API: http://localhost:8000
# Swagger UI: http://localhost:8000/swagger-ui/
```

## API Endpoints

### Kimlik Doğrulama
```
POST /api/auth/register  - Yeni kullanıcı kaydı
POST /api/auth/login     - Giriş
POST /api/auth/refresh    - Token yenileme
GET  /api/auth/me        - Aktif kullanıcı bilgisi
```

### Kullanıcılar
```
GET    /api/users        - Kullanıcı listesi
POST   /api/users        - Kullanıcı oluştur
GET    /api/users/{id}   - Kullanıcı detayı
PUT    /api/users/{id}   - Kullanıcı güncelle
DELETE /api/users/{id}   - Kullanıcı sil
```

### Tenant
```
GET    /api/tenants        - Tenant listesi
POST   /api/tenants        - Tenant oluştur
GET    /api/tenants/{id}   - Tenant detayı
PUT    /api/tenants/{id}   - Tenant güncelle
DELETE /api/tenants/{id}   - Tenant sil
```

### Swagger UI
- **Swagger UI**: `http://localhost:8000/swagger-ui/`
- **OpenAPI Spec**: `http://localhost:8000/api-docs/openapi.json`

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
│  Sales  │  Purchase  │  HR  │  Accounting  │  Assets      │
├─────────┴───────────┴──────┴──────────────┴────────────────┤
│                   Extended Modules                           │
│  Projects  │  Manufacturing  │  BOM  │  QC  │  Shop Floor  │
├───────────┴─────────────────┴───────┴──────┴───────────────┤
│                         CRM                                  │
│     Leads  │  Opportunities  │  Campaigns  │  Tickets        │
└─────────────────────────────────────────────────────────────┘
```

### Proje Yapısı
```
turerp/
├── src/
│   ├── api/              # HTTP handlers (Actix-web)
│   ├── config/           # Konfigürasyon
│   ├── domain/           # Domain modülleri (business logic)
│   │   ├── auth/         # Kimlik doğrulama
│   │   ├── user/         # Kullanıcı yönetimi
│   │   ├── tenant/       # Tenant yönetimi
│   │   ├── cari/         # Cari hesaplar
│   │   ├── product/      # Ürün yönetimi
│   │   ├── stock/        # Stok yönetimi
│   │   ├── invoice/      # Fatura yönetimi
│   │   ├── sales/        # Satış modülü
│   │   ├── purchase/     # Satın alma modülü
│   │   ├── hr/           # İK modülü
│   │   ├── accounting/   # Muhasebe modülü
│   │   ├── project/      # Proje yönetimi
│   │   ├── manufacturing/# Üretim modülü
│   │   └── crm/          # CRM modülü
│   ├── error/            # Hata yönetimi
│   ├── middleware/       # HTTP middleware
│   ├── utils/            # Yardımcı fonksiyonlar
│   ├── lib.rs            # Library entry point
│   └── main.rs           # Application entry point
├── tests/                # Entegrasyon testleri
├── docs/                 # Modül dokümantasyonu
└── Cargo.toml            # Dependencies
```

## Test

```bash
# Tüm testler
cargo test

# Belirli modül testleri
cargo test --lib domain::cari

# Test coverage
cargo tarpaulin --out Html
```

## CI/CD

GitHub Actions ile otomatik:
- Format kontrolü (`cargo fmt --check`)
- Clippy linting (`cargo clippy`)
- Test çalıştırma (`cargo test`)
- Security audit (`cargo audit`)

## Environment Variables

| Değişken | Açıklama | Varsayılan |
|----------|---------|------------|
| `TURERP_DATABASE_URL` | PostgreSQL connection string | Zorunlu |
| `TURERP_JWT_SECRET` | JWT imzalama anahtarı | Zorunlu |
| `TURERP_SERVER_HOST` | Sunucu host | `0.0.0.0` |
| `TURERP_SERVER_PORT` | Sunucu portu | `8000` |
| `TURERP_DB_MAX_CONNECTIONS` | Max DB bağlantısı | `10` |
| `TURERP_JWT_ACCESS_EXPIRATION` | Access token süresi (saniye) | `3600` |
| `TURERP_JWT_REFRESH_EXPIRATION` | Refresh token süresi (saniye) | `604800` |
| `RUST_LOG` | Log seviyesi | `info` |

## Katkıda Bulunma

1. Fork yapın
2. Feature branch oluşturun (`git checkout -b feature/amazing-feature`)
3. Değişikliklerinizi commit edin (`git commit -m 'feat: amazing feature'`)
4. Branch'i push edin (`git push origin feature/amazing-feature`)
5. Pull Request açın

## Dokümantasyon

- [Modül Dokümantasyonu](docs/README.md)
- [Implementasyon Planı](IMPLEMENTATION_PLAN.md)
- [API Dokümantasyonu](http://localhost:8000/swagger-ui/)

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

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

**Turerp Team** - Modern ERP çözümleri için Rust ile geliştirilmiştir.