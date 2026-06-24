-- Sample data for the turerp dev/test stack (tenant 1 = "Default Tenant").
--
-- Purpose: populate the modules the hurl smoke suite (tests/hurl/) exercises
-- so list endpoints return non-empty envelopes and specific-row lookups hit
-- real rows instead of 404. This is a DEV/TEST fixture only — never run it
-- against a production database (it hardcodes tenant_id = 1 and fixture names).
--
-- Idempotent: every INSERT uses ON CONFLICT DO NOTHING keyed on the table's
-- natural unique constraint, so re-running after a partial apply or a re-seed
-- is safe and will not duplicate rows or raise constraint violations.
--
-- Apply with:
--   docker compose --env-file .env.compose exec -T db \
--     psql -U turerp -d turerp -f - < scripts/seed_sample_data.sql

BEGIN;

-- ---------------------------------------------------------------------------
-- Currencies: USD is the base currency for tenant 1. Seeding USD makes
-- GET /api/v1/currencies/USD return 200 (the hurl suite's 06_currencies
-- scenario asserts 200 once USD is seeded).
-- ---------------------------------------------------------------------------
INSERT INTO currencies (tenant_id, code, name, symbol, decimal_places, is_active, is_base)
VALUES
    (1, 'USD', 'US Dollar',       '$',  2, true, true),
    (1, 'TRY', 'Turkish Lira',    '₺',  2, true, false),
    (1, 'EUR', 'Euro',            '€',  2, true, false)
ON CONFLICT ON CONSTRAINT currencies_tenant_code_unique DO NOTHING;

-- ---------------------------------------------------------------------------
-- Exchange rates: all rates are USD-quoted and use today's date. The
-- rate_positive CHECK constraint requires rate > 0.
-- ---------------------------------------------------------------------------
INSERT INTO exchange_rates (tenant_id, from_currency, to_currency, rate, effective_date)
VALUES
    (1, 'USD', 'TRY', 32.5000,  CURRENT_DATE),
    (1, 'USD', 'EUR', 0.9200,   CURRENT_DATE),
    (1, 'EUR', 'TRY', 35.3300,  CURRENT_DATE)
ON CONFLICT ON CONSTRAINT exchange_rates_tenant_from_to_date_unique DO NOTHING;

-- ---------------------------------------------------------------------------
-- Settings: default_locale is the key the 16_settings hurl scenario looks up
-- (asserts 200 once seeded). value/default_value are JSONB.
-- ---------------------------------------------------------------------------
INSERT INTO settings (tenant_id, key, value, default_value, data_type, group_name, description, is_sensitive, is_editable)
VALUES
    (1, 'default_locale', '"en"'::jsonb,           '"en"'::jsonb,           'string', 'general',  'Default UI locale',         false, true),
    (1, 'company_name',   '"Default Tenant"'::jsonb, '"Default Tenant"'::jsonb, 'string', 'general',  'Legal company name',        false, true),
    (1, 'base_currency',  '"USD"'::jsonb,          '"USD"'::jsonb,          'string', 'financial','Base currency code',         false, true),
    (1, 'fiscal_year_start', '"01-01"'::jsonb,     '"01-01"'::jsonb,        'string', 'financial','Fiscal year start (MM-DD)',  false, true)
ON CONFLICT ON CONSTRAINT settings_tenant_id_key_key DO NOTHING;

-- ---------------------------------------------------------------------------
-- Categories: top-level (parent_id NULL).
-- ---------------------------------------------------------------------------
INSERT INTO categories (tenant_id, name, parent_id)
VALUES
    (1, 'Electronics', NULL),
    (1, 'Books',       NULL),
    (1, 'Clothing',    NULL)
ON CONFLICT (name, tenant_id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Units.
-- ---------------------------------------------------------------------------
INSERT INTO units (tenant_id, code, name, is_integer)
VALUES
    (1, 'PCS', 'Pieces',  true),
    (1, 'KG',  'Kilogram', false),
    (1, 'MTR', 'Meter',    false)
ON CONFLICT (code, tenant_id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Warehouses: real codes only. Deliberately NOT id 999999 — the 14_stock_items
-- hurl scenario asserts GET /warehouses/999999 returns 404 (non-existent row),
-- and that must stay green.
-- ---------------------------------------------------------------------------
INSERT INTO warehouses (tenant_id, code, name, address, is_active)
VALUES
    (1, 'WH01', 'Main Warehouse',   'Istanbul, TR', true),
    (1, 'WH02', 'Secondary Warehouse', 'Ankara, TR',  true)
ON CONFLICT (code, tenant_id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- HR leave types.
-- ---------------------------------------------------------------------------
INSERT INTO leave_types (tenant_id, name, description, max_days_per_year, requires_approval)
VALUES
    (1, 'Annual Leave',  'Paid yearly vacation',     14, true),
    (1, 'Sick Leave',    'Medical leave',             5, true),
    (1, 'Unpaid Leave',  'Leave without pay',         0, false)
ON CONFLICT (name, tenant_id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Employees: user_id NULL (not linked to a login user). company_id defaults
-- to 1 via the column default but set explicitly for clarity.
--
-- updated_at MUST be set: the employees table has no default and no
-- BEFORE-UPDATE trigger backfilling it, and EmployeeRow decodes it as a
-- non-null DateTime<Utc>. A NULL updated_at (e.g. from an INSERT that omits
-- it) makes sqlx::FromRow fail and GET /hr/employees return 500. The API's
-- own create() always sets updated_at = NOW(); the seed must match that.
-- ---------------------------------------------------------------------------
INSERT INTO employees (tenant_id, user_id, employee_number, first_name, last_name, email, phone, department, position, hire_date, status, salary, company_id, created_at, updated_at)
VALUES
    (1, NULL, 'EMP001', 'Ahmet',  'Yilmaz', 'ahmet.yilmaz@turerp.local',  '+905551112233', 'Engineering', 'Software Engineer', NOW(), 'Active', 75000, 1, NOW(), NOW()),
    (1, NULL, 'EMP002', 'Ayse',   'Demir',  'ayse.demir@turerp.local',   '+905554445566', 'Sales',       'Sales Manager',     NOW(), 'Active', 68000, 1, NOW(), NOW())
ON CONFLICT (email, tenant_id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Companies: one operating company. cari/products/invoices all get
-- company_id via a NOT NULL DEFAULT 1 column (migration 023), so they do
-- not strictly require this row to exist, but seeding it keeps the FK-like
-- relationship honest for any cross-module read.
-- Idempotency: the unique index is PARTIAL (WHERE deleted_at IS NULL), so
-- ON CONFLICT cannot target it portably; use a NOT EXISTS guard instead.
-- ---------------------------------------------------------------------------
INSERT INTO companies (tenant_id, code, name, tax_number, currency, is_active, created_at, updated_at)
SELECT 1, 'CO01', 'Turerp Demo A.S.', '1234567890', 'TRY', true, NOW(), NOW()
WHERE NOT EXISTS (
    SELECT 1 FROM companies WHERE tenant_id = 1 AND code = 'CO01' AND deleted_at IS NULL
);

-- ---------------------------------------------------------------------------
-- Cost centers: one cost center (general admin) and one profit center
-- (sales). type is 'cost' or 'profit'. Partial unique index -> NOT EXISTS.
-- ---------------------------------------------------------------------------
INSERT INTO cost_centers (tenant_id, code, name, type, is_active, created_at, updated_at)
SELECT 1, 'CC01', 'Genel Yonetim', 'cost', true, NOW(), NOW()
WHERE NOT EXISTS (
    SELECT 1 FROM cost_centers WHERE tenant_id = 1 AND code = 'CC01' AND deleted_at IS NULL
);
INSERT INTO cost_centers (tenant_id, code, name, type, is_active, created_at, updated_at)
SELECT 1, 'CC02', 'Satis ve Pazarlama', 'profit', true, NOW(), NOW()
WHERE NOT EXISTS (
    SELECT 1 FROM cost_centers WHERE tenant_id = 1 AND code = 'CC02' AND deleted_at IS NULL
);

-- ---------------------------------------------------------------------------
-- Tax rates: Turkish VAT (KDV) 20% as the default rate, plus two distinct
-- tax types (OIV, BSMV) for breadth.
-- IMPORTANT: the unique index is (tenant_id, tax_type, effective_from)
-- WHERE deleted_at IS NULL, so only ONE row per tax_type per effective_from
-- is allowed. We therefore use DISTINCT tax_types (not three KDV rows on the
-- same date) and guard with NOT EXISTS.
-- ---------------------------------------------------------------------------
INSERT INTO tax_rates (tenant_id, tax_type, rate, effective_from, description, is_default, created_at)
SELECT 1, 'KDV', 20.0000, CURRENT_DATE, 'Genel KDV %20', true, NOW()
WHERE NOT EXISTS (
    SELECT 1 FROM tax_rates WHERE tenant_id = 1 AND tax_type = 'KDV' AND effective_from = CURRENT_DATE AND deleted_at IS NULL
);
INSERT INTO tax_rates (tenant_id, tax_type, rate, effective_from, description, is_default, created_at)
SELECT 1, 'OIV', 0.4000, CURRENT_DATE, 'Ozel Iletisim Vergisi', false, NOW()
WHERE NOT EXISTS (
    SELECT 1 FROM tax_rates WHERE tenant_id = 1 AND tax_type = 'OIV' AND effective_from = CURRENT_DATE AND deleted_at IS NULL
);
INSERT INTO tax_rates (tenant_id, tax_type, rate, effective_from, description, is_default, created_at)
SELECT 1, 'BSMV', 0.0500, CURRENT_DATE, 'Banka ve Sigorta Muameleleri Vergisi', false, NOW()
WHERE NOT EXISTS (
    SELECT 1 FROM tax_rates WHERE tenant_id = 1 AND tax_type = 'BSMV' AND effective_from = CURRENT_DATE AND deleted_at IS NULL
);

-- ---------------------------------------------------------------------------
-- Chart of accounts: a minimal Tek Duzen Hesap Plani skeleton. group_name
-- and account_type are stored as strings and parsed in the repo with an
-- unwrap_or(default) fallback, so an invalid value never 500s — but we use
-- the canonical strings the AccountGroup/AccountType enums emit so
-- list-by-group / list-by-type filters match. updated_at has a NOT NULL
-- DEFAULT now(), so we omit it. Partial unique index (code per tenant where
-- not deleted) -> multi-row NOT EXISTS guard via an anti-join on a VALUES
-- table.
-- ---------------------------------------------------------------------------
INSERT INTO chart_accounts (tenant_id, code, name, group_name, parent_code, level, account_type, is_active, balance, allow_posting)
SELECT v.code_tenant, v.code, v.name, v.group_name, NULL::varchar, v.lvl, v.account_type, true, 0, true
FROM (VALUES
    (1::bigint, '100',  'KASA',                            'DonenVarliklar',              1, 'Asset'),
    (1::bigint, '102',  'BANKALAR',                        'DonenVarliklar',              1, 'Asset'),
    (1::bigint, '120',  'ALICILAR',                        'DonenVarliklar',              1, 'Asset'),
    (1::bigint, '153',  'TICARI MALLAR',                   'DonenVarliklar',              1, 'Asset'),
    (1::bigint, '320',  'SATICILAR',                       'KisaVadeliYabanciKaynaklar',  1, 'Liability'),
    (1::bigint, '329',  'ODENENCEK VERGILER VE FONLAR',    'KisaVadeliYabanciKaynaklar',  1, 'Liability'),
    (1::bigint, '600',  'YURTICI SATISLAR',                'GelirTablosu',                1, 'Revenue'),
    (1::bigint, '770',  'GENEL YONETIM GIDERLERI',         'GiderTablosu',                1, 'Expense')
) AS v(code_tenant, code, name, group_name, lvl, account_type)
WHERE NOT EXISTS (
    SELECT 1 FROM chart_accounts c
    WHERE c.tenant_id = 1 AND c.code = v.code AND c.deleted_at IS NULL
);

-- ---------------------------------------------------------------------------
-- Cari (customers + vendors). created_by is NOT NULL and REFERENCES
-- users(id); testuser is id 2 in the default seed, so use 2. default_currency
-- is NOT NULL DEFAULT 'TRY'. Non-partial unique (code, tenant_id) ->
-- ON CONFLICT DO NOTHING. updated_at is Option<DateTime> in CariRow so a NULL
-- is safe, but we set it explicitly for consistency.
-- ---------------------------------------------------------------------------
INSERT INTO cari (tenant_id, code, name, cari_type, tax_number, email, phone, city, country, credit_limit, current_balance, status, default_currency, company_id, created_by, created_at, updated_at)
VALUES
    (1, 'C001', 'Musteri A Ltd.',     'customer', '1111111111', 'musteri.a@turerp.local', '+905551110001', 'Istanbul', 'TR', 50000.0, 0.0, 'active', 'TRY', 1, 2, NOW(), NOW()),
    (1, 'C002', 'Musteri B A.S.',     'customer', '2222222222', 'musteri.b@turerp.local', '+905551110002', 'Ankara',   'TR', 75000.0, 0.0, 'active', 'TRY', 1, 2, NOW(), NOW()),
    (1, 'V001', 'Tedarikci X Ltd.',   'vendor',   '3333333333', 'tedarikci.x@turerp.local','+905551110003', 'Izmir',    'TR', 0.0,      0.0, 'active', 'TRY', 1, 2, NOW(), NOW())
ON CONFLICT (code, tenant_id) DO NOTHING;

-- ---------------------------------------------------------------------------
-- Products. category_id / unit_id are resolved by subquery against the
-- categories and units seeded above (NOT hardcoded ids), so this stays
-- correct regardless of the SERIAL ids those rows received.
-- updated_at is a NON-OPTION DateTime<Utc> in ProductRow (the same latent
-- decode-500 class as employees): it MUST be set, or GET /products 500s.
-- The column is nullable with no default and only a BEFORE-UPDATE trigger,
-- so an INSERT that omits it leaves NULL. We set it here AND backfill below.
-- Partial unique index (code, tenant_id WHERE deleted_at IS NULL) -> NOT EXISTS.
-- ---------------------------------------------------------------------------
INSERT INTO products (tenant_id, company_id, code, name, description, category_id, unit_id, barcode, purchase_price, sale_price, tax_rate, is_active, created_at, updated_at)
SELECT
    1, 1, v.code, v.name, v.description,
    (SELECT id FROM categories WHERE name = 'Electronics' AND tenant_id = 1),
    (SELECT id FROM units      WHERE code = 'PCS'          AND tenant_id = 1),
    v.barcode, v.purchase_price, v.sale_price, v.tax_rate, true, NOW(), NOW()
FROM (VALUES
    ('P001', 'Laptop 15"',      'Demo laptop unit',      '8690000000001', 18000.0, 25000.0, 20.0),
    ('P002', 'Wireless Mouse',  'Demo mouse unit',       '8690000000002',  250.0,   600.0,  20.0),
    ('P003', 'Mechanical Keyboard', 'Demo keyboard unit','8690000000003', 1200.0,  2200.0, 20.0)
) AS v(code, name, description, barcode, purchase_price, sale_price, tax_rate)
WHERE NOT EXISTS (SELECT 1 FROM products p WHERE p.tenant_id = 1 AND p.code = v.code AND p.deleted_at IS NULL);

-- ---------------------------------------------------------------------------
-- Invoices. INV-2026-001 is a Sent, unpaid, total>0 invoice so it shows up
-- in GET /invoices/outstanding. INV-2026-002 is fully Paid (paid_amount =
-- total_amount) so it does not. cari_id is resolved by subquery. updated_at
-- is Option<DateTime> in InvoiceRow, so NULL-safe, but set explicitly.
-- Non-partial unique (invoice_number, tenant_id) -> ON CONFLICT DO NOTHING.
-- ---------------------------------------------------------------------------
INSERT INTO invoices (tenant_id, company_id, invoice_number, invoice_type, status, cari_id, issue_date, due_date, subtotal, tax_amount, discount_amount, total_amount, paid_amount, currency, notes, created_at, updated_at)
SELECT
    1, 1, v.invoice_number, 'SalesInvoice', v.status,
    (SELECT id FROM cari WHERE code = v.cari_code AND tenant_id = 1),
    NOW(), NOW() + INTERVAL '30 days',
    v.subtotal, v.tax_amount, 0, v.total_amount, v.paid_amount, 'TRY', v.notes, NOW(), NOW()
FROM (VALUES
    ('INV-2026-001', 'Sent', 'C001', 10000.0, 2000.0, 12000.0,      0.0, 'Demo sales invoice - outstanding'),
    ('INV-2026-002', 'Paid', 'C002',  5000.0, 1000.0,  6000.0, 6000.0, 'Demo sales invoice - paid')
) AS v(invoice_number, status, cari_code, subtotal, tax_amount, total_amount, paid_amount, notes)
WHERE NOT EXISTS (SELECT 1 FROM invoices i WHERE i.tenant_id = 1 AND i.invoice_number = v.invoice_number);

-- ---------------------------------------------------------------------------
-- Invoice lines for INV-2026-001. invoice_lines has NO unique index, so a
-- naive re-run would duplicate lines. Guard each line by (invoice_number,
-- sort_order) via NOT EXISTS so re-running is a no-op. product_id is
-- resolved by subquery (nullable FK, but we point at real products).
-- ---------------------------------------------------------------------------
INSERT INTO invoice_lines (invoice_id, product_id, description, quantity, unit_price, tax_rate, discount_rate, line_total, sort_order)
SELECT i.id, p.id, 'Laptop 15" x2', 2, 5000.0, 20.0, 0, 10000.0, 1
FROM invoices i, products p
WHERE i.tenant_id = 1 AND i.invoice_number = 'INV-2026-001'
  AND p.tenant_id = 1 AND p.code = 'P001'
  AND NOT EXISTS (
      SELECT 1 FROM invoice_lines al WHERE al.invoice_id = i.id AND al.sort_order = 1
  );

INSERT INTO invoice_lines (invoice_id, product_id, description, quantity, unit_price, tax_rate, discount_rate, line_total, sort_order)
SELECT i.id, p.id, 'Mechanical Keyboard x5', 5, 400.0, 20.0, 0, 2000.0, 2
FROM invoices i, products p
WHERE i.tenant_id = 1 AND i.invoice_number = 'INV-2026-001'
  AND p.tenant_id = 1 AND p.code = 'P003'
  AND NOT EXISTS (
      SELECT 1 FROM invoice_lines al WHERE al.invoice_id = i.id AND al.sort_order = 2
  );

-- ---------------------------------------------------------------------------
-- Corrective backfill: if this script is re-run over rows a prior version
-- inserted without updated_at (nullable column, non-null struct), set them
-- now so the list endpoints decode cleanly. Idempotent and safe.
-- ---------------------------------------------------------------------------
-- Among seeded tables, employees and products have updated_at decoded by a
-- NON-OPTION DateTime<Utc> row struct on a nullable column with no default —
-- both MUST be non-NULL or the list endpoint 500s (the employees bug class).
-- currencies/cari/invoices/companies/cost_centers have nullable updated_at
-- (Option struct or no endpoint reading them) but we backfill anyway for
-- consistency. settings has DEFAULT now(); categories/units/warehouses/
-- leave_types have no updated_at column; chart_accounts has NOT NULL
-- DEFAULT now(); tax_rates has no updated_at column.
UPDATE employees    SET updated_at = NOW() WHERE updated_at IS NULL;
UPDATE products     SET updated_at = NOW() WHERE updated_at IS NULL;
UPDATE currencies   SET updated_at = NOW() WHERE updated_at IS NULL;
UPDATE cari         SET updated_at = NOW() WHERE updated_at IS NULL;
UPDATE invoices     SET updated_at = NOW() WHERE updated_at IS NULL;
UPDATE companies    SET updated_at = NOW() WHERE updated_at IS NULL;
UPDATE cost_centers SET updated_at = NOW() WHERE updated_at IS NULL;

COMMIT;