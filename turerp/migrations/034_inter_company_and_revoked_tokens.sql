-- Migration: Inter-company tables + Revoked tokens
-- Replaces InMemory implementations with persistent PostgreSQL storage

-- ---------------------------------------------------------------------------
-- Revoked Tokens
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS revoked_tokens (
    token_hash VARCHAR(128) PRIMARY KEY,
    revoked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    tenant_id BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_revoked_tokens_tenant_id ON revoked_tokens(tenant_id);
CREATE INDEX IF NOT EXISTS idx_revoked_tokens_expires_at ON revoked_tokens(expires_at);

-- ---------------------------------------------------------------------------
-- Inter-Company Invoices
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS inter_company_invoices (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    seller_company_id BIGINT NOT NULL,
    buyer_company_id BIGINT NOT NULL,
    sales_invoice_id BIGINT NOT NULL,
    purchase_invoice_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_inter_company_invoices_tenant_id ON inter_company_invoices(tenant_id);
CREATE INDEX IF NOT EXISTS idx_inter_company_invoices_seller ON inter_company_invoices(seller_company_id);
CREATE INDEX IF NOT EXISTS idx_inter_company_invoices_buyer ON inter_company_invoices(buyer_company_id);

-- ---------------------------------------------------------------------------
-- Inter-Company Invoice Lines
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS inter_company_invoice_lines (
    invoice_id BIGINT NOT NULL REFERENCES inter_company_invoices(id) ON DELETE CASCADE,
    product_id BIGINT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    quantity NUMERIC(18, 4) NOT NULL,
    unit_price NUMERIC(18, 4) NOT NULL,
    vat_rate NUMERIC(5, 4) NOT NULL DEFAULT 0,
    PRIMARY KEY (invoice_id, product_id)
);

-- ---------------------------------------------------------------------------
-- Inter-Company Stock Transfers
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS inter_company_stock_transfers (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    from_company_id BIGINT NOT NULL,
    to_company_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    warehouse_id BIGINT NOT NULL,
    quantity NUMERIC(18, 4) NOT NULL,
    out_movement_id BIGINT NOT NULL,
    in_movement_id BIGINT NOT NULL,
    created_by BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_inter_company_transfers_tenant_id ON inter_company_stock_transfers(tenant_id);
CREATE INDEX IF NOT EXISTS idx_inter_company_transfers_from ON inter_company_stock_transfers(from_company_id);
CREATE INDEX IF NOT EXISTS idx_inter_company_transfers_to ON inter_company_stock_transfers(to_company_id);
