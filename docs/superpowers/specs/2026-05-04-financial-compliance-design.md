# Financial Compliance (P0) Design

**Date:** 2026-05-04
**Scope:** e-Fatura, e-Defter, Tax Engine + KVB, Chart of Accounts
**Priority:** P0 — Türkiye ERP pazarı için olmazsa olmaz

---

## 1. Overview

Turerp'a 4 yeni domain modülü eklenerek Türkiye finansal uyumluluk gereksinimleri karşılanacak. Her modül mevcut DDD pattern'ini (model → repository trait → service → postgres_repository → API) takip eder.

### Modules

| Module | Domain | Purpose |
|--------|--------|---------|
| e-Fatura | `efatura` | UBL-TR formatında e-Fatura oluşturma, imzalama, GIB gönderimi |
| e-Defter | `edefter` | GIB formatında yevmiye/büyük defter XML, berat imzalama, Saklayıcı gönderimi |
| Tax Engine | `tax` | Vergi oranı yönetimi, hesaplama motoru, KVB dönemi takibi |
| Chart of Accounts | `chart_of_accounts` | TEK uyumlu hesap planı iskeleti, hiyerarşi, bakiye |

---

## 2. Architecture

```
domain/
  efatura/
    model.rs
    repository.rs
    service.rs
    postgres_repository.rs
    ubl/
      mod.rs
      mapper.rs          # Invoice → UBL-TR XML dönüşüm
      validator.rs       # XSD doğrulama
      templates.rs       # UBL-TR XML şablonları

  edefter/
    model.rs
    repository.rs
    service.rs
    postgres_repository.rs
    gib/
      mod.rs
      yevmiye.rs         # Yevmiye defteri XML üretici
      buyuk_defter.rs    # Büyük defter XML üretici
      berat.rs           # Berat imzalama

  tax/
    model.rs
    repository.rs
    service.rs
    postgres_repository.rs
    calculator/
      mod.rs
      kdv.rs
      oiv.rs
      stopaj.rs
      bsmv.rs
      damga.rs

  chart_of_accounts/
    model.rs
    repository.rs
    service.rs
    postgres_repository.rs

common/
  gov.rs               # GibGateway trait (e-Fatura + e-Defter operatör bağlantısı)
```

---

## 3. e-Fatura (UBL-TR)

### 3.1 Data Model

```rust
enum EFaturaStatus {
    Draft,
    Signed,
    Sent,
    Accepted,
    Rejected,
    Cancelled,
    Error,
}

enum EFaturaProfile {
    TemelFatura,
    Ihracat,
    YolcuBeleni,
    OzelMatbuFatura,
}

struct EFatura {
    id: i64,
    tenant_id: i64,
    invoice_id: Option<i64>,
    uuid: String,
    document_number: String,
    issue_date: DateTime<Utc>,
    profile_id: EFaturaProfile,
    sender: PartyInfo,
    receiver: PartyInfo,
    lines: Vec<EFaturaLine>,
    tax_totals: Vec<TaxSubtotal>,
    legal_monetary_total: MonetaryTotal,
    status: EFaturaStatus,
    response_code: Option<String>,
    response_desc: Option<String>,
    xml_content: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

struct PartyInfo {
    vkn_tckn: String,
    name: String,
    tax_office: String,
    address: AddressInfo,
    email: Option<String>,
    phone: Option<String>,
    register_number: Option<String>,
    mersis_number: Option<String>,
}
```

### 3.2 Service

```rust
trait EFaturaService: Send + Sync {
    async fn create_from_invoice(invoice_id: i64, tenant_id: i64) -> Result<EFatura>;
    async fn generate_ubl(fatura_id: i64) -> Result<String>;
    async fn validate_ubl(xml: &str) -> Result<ValidationResult>;
    async fn send_to_gib(fatura_id: i64) -> Result<SendResult>;
    async fn check_status(fatura_id: i64) -> Result<EFaturaStatus>;
    async fn process_response(response: GibResponse) -> Result<()>;
    async fn cancel_efatura(fatura_id: i64, reason: String) -> Result<()>;
}
```

### 3.3 API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/v1/efatura | Admin | Invoice'dan e-Fatura oluştur |
| POST | /api/v1/efatura/{id}/send | Admin | GIB'e gönder |
| GET | /api/v1/efatura | Auth | Listele (paginated) |
| GET | /api/v1/efatura/{id} | Auth | Detay |
| GET | /api/v1/efatura/{id}/xml | Auth | UBL-TR XML indir |
| POST | /api/v1/efatura/{id}/cancel | Admin | İptal et |
| GET | /api/v1/efatura/status/{uuid} | Auth | GIB durum sorgula |

### 3.4 GIB Gateway

```rust
trait GibGateway: Send + Sync {
    async fn send_invoice(xml: &str, profile: EFaturaProfile) -> Result<GibSendResult>;
    async fn check_status(uuid: &str) -> Result<GibStatusResult>;
    async fn get_incoming(since: DateTime<Utc>) -> Result<Vec<GibIncomingInvoice>>;
    async fn cancel(uuid: &str, reason: &str) -> Result<()>;
}
```

Implementations: `InMemoryGibGateway` (test), `ParaSoftGibGateway` (production).

---

## 4. e-Defter (GIB)

### 4.1 Data Model

```rust
enum EDefterStatus {
    Draft,
    Signed,
    Sent,
    Accepted,
    Rejected,
    Cancelled,
}

enum LedgerType {
    YevmiyeDefteri,
    BuyukDefter,
    KebirDefter,
}

struct LedgerPeriod {
    id: i64,
    tenant_id: i64,
    year: i32,
    month: u32,
    period_type: LedgerType,
    status: EDefterStatus,
    berat_signed_at: Option<DateTime<Utc>>,
    sent_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

struct YevmiyeEntry {
    id: i64,
    period_id: i64,
    entry_number: i64,
    entry_date: Date,
    explanation: String,
    debit_total: Decimal,
    credit_total: Decimal,
    lines: Vec<YevmiyeLine>,
}

struct YevmiyeLine {
    account_code: String,
    account_name: String,
    debit: Decimal,
    credit: Decimal,
    explanation: String,
}

struct BeratInfo {
    period_id: i64,
    serial_number: String,
    sign_time: DateTime<Utc>,
    signer: String,
    digest_value: String,
    signature_value: String,
}
```

### 4.2 Service

```rust
trait EDefterService: Send + Sync {
    async fn create_period(year: i32, month: u32, tenant_id: i64) -> Result<LedgerPeriod>;
    async fn populate_from_accounting(period_id: i64) -> Result<()>;
    async fn validate_balance(period_id: i64) -> Result<BalanceCheckResult>;
    async fn generate_yevmiye_xml(period_id: i64) -> Result<String>;
    async fn generate_buyuk_defter_xml(period_id: i64) -> Result<String>;
    async fn sign_berat(period_id: i64) -> Result<BeratInfo>;
    async fn send_to_saklayici(period_id: i64) -> Result<SendResult>;
    async fn check_status(period_id: i64) -> Result<EDefterStatus>;
}
```

### 4.3 API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/v1/edefter/periods | Admin | Dönem oluştur |
| GET | /api/v1/edefter/periods | Auth | Dönemleri listele |
| POST | /api/v1/edefter/periods/{id}/populate | Admin | Accounting'den doldur |
| POST | /api/v1/edefter/periods/{id}/validate | Admin | Denge kontrolü |
| POST | /api/v1/edefter/periods/{id}/yevmiye-xml | Admin | Yevmiye XML üret |
| POST | /api/v1/edefter/periods/{id}/buyuk-defter-xml | Admin | Büyük defter XML üret |
| POST | /api/v1/edefter/periods/{id}/sign | Admin | Berat imzala |
| POST | /api/v1/edefter/periods/{id}/send | Admin | Saklayıcı'ya gönder |
| GET | /api/v1/edefter/periods/{id}/status | Auth | Durum sorgula |

---

## 5. Tax Engine + KVB

### 5.1 Data Model

```rust
enum TaxType {
    KDV,
    OIV,
    BSMV,
    Damga,
    Stopaj,
    KV,
    GV,
}

struct TaxRate {
    id: i64,
    tenant_id: i64,
    tax_type: TaxType,
    rate: Decimal,
    effective_from: Date,
    effective_to: Option<Date>,
    category: Option<String>,
    description: String,
    is_default: bool,
    created_at: DateTime<Utc>,
}

struct TaxCalculationResult {
    base_amount: Decimal,
    tax_type: TaxType,
    rate: Decimal,
    tax_amount: Decimal,
    inclusive: bool,
}

enum TaxPeriodStatus {
    Open,
    Calculated,
    Filed,
    Closed,
}

struct TaxPeriod {
    id: i64,
    tenant_id: i64,
    tax_type: TaxType,
    period_year: i32,
    period_month: u32,
    total_base: Decimal,
    total_tax: Decimal,
    total_deduction: Decimal,
    net_tax: Decimal,
    status: TaxPeriodStatus,
    filed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

struct TaxPeriodDetail {
    id: i64,
    period_id: i64,
    transaction_date: Date,
    transaction_type: String,
    base_amount: Decimal,
    tax_rate: Decimal,
    tax_amount: Decimal,
    deduction_amount: Decimal,
    reference_id: Option<i64>,
}
```

### 5.2 Service

```rust
trait TaxService: Send + Sync {
    // Rate management
    async fn create_tax_rate(rate: CreateTaxRate) -> Result<TaxRate>;
    async fn get_effective_rate(tax_type: TaxType, date: Date, tenant_id: i64) -> Result<Option<TaxRate>>;
    async fn list_rates(tenant_id: i64, tax_type: Option<TaxType>) -> Result<Vec<TaxRate>>;
    async fn update_tax_rate(id: i64, update: UpdateTaxRate) -> Result<TaxRate>;

    // Calculation
    async fn calculate_tax(amount: Decimal, tax_type: TaxType, date: Date, tenant_id: i64) -> Result<TaxCalculationResult>;
    async fn calculate_invoice_taxes(invoice_id: i64, tenant_id: i64) -> Result<Vec<TaxCalculationResult>>;

    // KVB periods
    async fn create_tax_period(tax_type: TaxType, year: i32, month: u32, tenant_id: i64) -> Result<TaxPeriod>;
    async fn calculate_period(period_id: i64) -> Result<TaxPeriod>;
    async fn file_period(period_id: i64) -> Result<TaxPeriod>;
    async fn get_period_details(period_id: i64) -> Result<Vec<TaxPeriodDetail>>;
    async fn list_periods(tenant_id: i64, tax_type: Option<TaxType>) -> Result<Vec<TaxPeriod>>;
}
```

### 5.3 API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/v1/tax/rates | Admin | Vergi oranı oluştur |
| GET | /api/v1/tax/rates | Auth | Oranları listele |
| GET | /api/v1/tax/rates/{id} | Auth | Oran detayı |
| PUT | /api/v1/tax/rates/{id} | Admin | Oran güncelle |
| GET | /api/v1/tax/rates/effective | Auth | Geçerli oranı sorgula |
| POST | /api/v1/tax/calculate | Auth | Vergi hesapla |
| POST | /api/v1/tax/calculate-invoice | Auth | Fatura vergilerini hesapla |
| POST | /api/v1/tax/periods | Admin | KVB dönemi oluştur |
| GET | /api/v1/tax/periods | Auth | Dönemleri listele |
| POST | /api/v1/tax/periods/{id}/calculate | Admin | Dönemi hesapla |
| POST | /api/v1/tax/periods/{id}/file | Admin | Beyan ver |
| GET | /api/v1/tax/periods/{id}/details | Auth | Dönem detayları |

### 5.4 Tax Calculator Modules

Her vergi türü kendi calculator modülünde:
- `kdv.rs` — KDV (1%, 10%, 20%), istisna, muafiyet
- `oiv.rs` — Özel İletişim Vergisi
- `stopaj.rs` — Gelir Vergisi Stopajı (%15, %10, vb.)
- `bsmv.rs` — BSMV (%5)
- `damga.rs` — Damga Vergisi (binde 9.48)

---

## 6. Chart of Accounts (Iskelet)

### 6.1 Data Model

```rust
enum AccountGroup {
    DonenVarliklar,          // 100
    DuranVarliklar,          // 200
    KisaVadeliYabanciKaynaklar, // 300
    UzunVadeliYabanciKaynaklar, // 400
    OzKaynaklar,             // 500
    GelirTablosu,            // 600-799
    GiderTablosu,            // 600-799 (alt grup ile ayrılır)
}

struct ChartAccount {
    id: i64,
    tenant_id: i64,
    code: String,
    name: String,
    group: AccountGroup,
    parent_code: Option<String>,
    level: u8,
    account_type: AccountType,
    is_active: bool,
    balance: Decimal,
    allow_posting: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

struct AccountTreeNode {
    code: String,
    name: String,
    group: AccountGroup,
    balance: Decimal,
    children: Vec<AccountTreeNode>,
}
```

### 6.2 Service

```rust
trait ChartOfAccountsService: Send + Sync {
    async fn create_account(account: CreateChartAccount, tenant_id: i64) -> Result<ChartAccount>;
    async fn get_account(code: String, tenant_id: i64) -> Result<Option<ChartAccount>>;
    async fn list_accounts(tenant_id: i64, group: Option<AccountGroup>) -> Result<Vec<ChartAccount>>;
    async fn update_account(code: String, update: UpdateChartAccount, tenant_id: i64) -> Result<ChartAccount>;
    async fn delete_account(code: String, tenant_id: i64) -> Result<()>;
    async fn get_tree(tenant_id: i64) -> Result<Vec<AccountTreeNode>>;
    async fn get_children(parent_code: String, tenant_id: i64) -> Result<Vec<ChartAccount>>;
    async fn recalculate_balance(code: String, tenant_id: i64) -> Result<Decimal>;
    async fn get_trial_balance(tenant_id: i64) -> Result<Vec<TrialBalanceEntry>>;
}
```

### 6.3 API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/v1/chart-of-accounts | Admin | Hesap oluştur |
| GET | /api/v1/chart-of-accounts | Auth | Hesapları listele |
| GET | /api/v1/chart-of-accounts/{code} | Auth | Hesap detayı |
| PUT | /api/v1/chart-of-accounts/{code} | Admin | Hesap güncelle |
| DELETE | /api/v1/chart-of-accounts/{code} | Admin | Hesap sil |
| GET | /api/v1/chart-of-accounts/tree | Auth | Hesap ağacı |
| GET | /api/v1/chart-of-accounts/{code}/children | Auth | Alt hesaplar |
| POST | /api/v1/chart-of-accounts/{code}/recalculate | Admin | Bakiye yeniden hesapla |
| GET | /api/v1/chart-of-accounts/trial-balance | Auth | Geçici mizan |

### 6.4 Relationship with Existing Accounting Domain

Mevcut `Account` modeli (code, name, account_type, sub_type) basit bir hesap tanımıdır. Yeni `ChartAccount` bunu genişletir:
- `ChartAccount` TEK hiyerarşisini (string kodlama: "100.01") ekler
- Mevcut `Account` ile paralel yaşayabilir; migration ile birleştirilir
- `Accounting` domain yevmiye kayıtları e-Defter ve ChartAccount'a beslenir

---

## 7. Cross-Module Integration

### Data Flow

```
Invoice → e-Fatura → GIB
Invoice → Tax Engine → KVB Period
Accounting → e-Defter → GIB Saklayıcı
Chart of Accounts → Accounting (hesap kodları)
Tax Engine → e-Fatura (vergi hesaplama)
```

### Shared Infrastructure

- `common/gov.rs` — `GibGateway` trait (e-Fatura + e-Defter operatör bağlantısı)
- Mevcut `EventBus` — Domain event'leri: `EFaturaSent`, `EDefterPeriodCreated`, `TaxPeriodCalculated`
- Mevcut `AuditLog` — Tüm GIB işlemleri auditlenir
- Mevcut `TenantMiddleware` — Tüm yeni modüller tenant izole

### Database Migrations

- `009_efatura.sql` — e-Fatura tabloları
- `010_edefter.sql` — e-Defter tabloları
- `011_tax_engine.sql` — Vergi oranları + KVB tabloları
- `012_chart_of_accounts.sql` — Hesap planı tabloları

---

## 8. Implementation Order

1. **Chart of Accounts** — Diğer tüm modüller hesap kodlarına bağımlı
2. **Tax Engine** — e-Fatura vergi hesaplaması ve KVB için gerekli
3. **e-Fatura** — Tax Engine ve Chart of Accounts'a bağımlı
4. **e-Defter** — Chart of Accounts ve Accounting domain'e bağımlı

---

## 9. Total New Endpoints

| Module | Endpoints |
|--------|-----------|
| e-Fatura | 7 |
| e-Defter | 9 |
| Tax Engine + KVB | 12 |
| Chart of Accounts | 9 |
| **Total** | **37** |