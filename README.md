# Turerp ERP

[![CI](https://github.com/ilkerhalil/turerp-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/ilkerhalil/turerp-rust/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-99%20passing-brightgreen)]()

**Multi-tenant SaaS ERP sistemi** - Rust, Actix-web ve SQLx ile geliştirilmiştir.

## Proje Yapısı

```
turerp-rust/
├── turerp/                 # Ana uygulama (Rust crate)
│   ├── src/                # Kaynak kod
│   ├── tests/              # Entegrasyon testleri
│   ├── docs/               # Modül dokümantasyonu
│   └── Cargo.toml          # Bağımlılıklar
├── docs/                   # Proje dokümantasyonu
│   └── modules/            # Modül detayları
├── .github/                # GitHub Actions CI/CD
├── AGENTS.md               # AI agent konfigürasyonu
└── IMPLEMENTATION_PLAN.md  # Implementasyon planı
```

## Hızlı Başlangıç

```bash
# Projeyi klonla
git clone https://github.com/ilkerhalil/turerp-rust.git
cd turerp-rust/turerp

# Environment variables ayarla
export TURERP_DATABASE_URL="postgres://postgres:postgres@localhost:5432/turerp"
export TURERP_JWT_SECRET="your-secret-key-change-in-production"

# Build ve çalıştır
cargo build
cargo run
```

## Modüller

| Kategori | Modüller |
|----------|----------|
| **Core** | Auth, Tenant, User, Cari, Product, Stock, Invoice |
| **Business** | Sales, Purchase, HR, Accounting |
| **Extended** | Project, Manufacturing, BOM, Quality Control, CRM |

## Teknoloji Stack

- **Backend**: Rust, Actix-web 4
- **Database**: PostgreSQL + SQLx
- **Auth**: JWT + bcrypt
- **API Docs**: utoipa (OpenAPI/Swagger)
- **Architecture**: Domain-Driven Design

## Dokümantasyon

- [Modül Dokümantasyonu](docs/README.md)
- [Implementasyon Planı](IMPLEMENTATION_PLAN.md)
- [API Dokümantasyonu](http://localhost:8000/swagger-ui/)

## Detaylı README

Ana uygulama için detaylı README: [turerp/README.md](turerp/README.md)

## Lisans

MIT License