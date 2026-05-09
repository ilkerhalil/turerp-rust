-- Multi-company support migration for Turerp ERP
-- Creates companies table and adds company_id to existing business tables

-- ============================================================================
-- COMPANIES TABLE
-- ============================================================================
CREATE TABLE IF NOT EXISTS companies (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    code VARCHAR(50) NOT NULL,
    name VARCHAR(200) NOT NULL,
    tax_number VARCHAR(20),
    address VARCHAR(500),
    city VARCHAR(100),
    country VARCHAR(100),
    currency VARCHAR(10) NOT NULL DEFAULT 'TRY',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    deleted_by BIGINT REFERENCES users(id)
);

CREATE UNIQUE INDEX idx_companies_code_tenant ON companies(code, tenant_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_companies_tenant_id ON companies(tenant_id);
CREATE INDEX idx_companies_tenant_active ON companies(tenant_id, is_active) WHERE deleted_at IS NULL;

CREATE TRIGGER update_companies_updated_at
    BEFORE UPDATE ON companies
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- ADD company_id TO EXISTING TABLES (default 1 for backward compatibility)
-- ============================================================================

ALTER TABLE cari ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_cari_company_id ON cari(company_id);

ALTER TABLE products ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_products_company_id ON products(company_id);

ALTER TABLE stock_movements ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_stock_movements_company_id ON stock_movements(company_id);

ALTER TABLE invoices ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_invoices_company_id ON invoices(company_id);

ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_sales_orders_company_id ON sales_orders(company_id);

ALTER TABLE purchase_orders ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_purchase_orders_company_id ON purchase_orders(company_id);

ALTER TABLE journal_entries ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_journal_entries_company_id ON journal_entries(company_id);

ALTER TABLE payments ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_payments_company_id ON payments(company_id);

ALTER TABLE assets ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_assets_company_id ON assets(company_id);

ALTER TABLE employees ADD COLUMN IF NOT EXISTS company_id BIGINT NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_employees_company_id ON employees(company_id);
