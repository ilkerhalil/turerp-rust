-- Tax Engine tables
CREATE TABLE IF NOT EXISTS tax_rates (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    tax_type VARCHAR(20) NOT NULL,
    rate NUMERIC(10,4) NOT NULL,
    effective_from DATE NOT NULL,
    effective_to DATE,
    category VARCHAR(100),
    description TEXT NOT NULL DEFAULT '',
    is_default BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_tax_rates_tenant_type ON tax_rates(tenant_id, tax_type);
CREATE INDEX idx_tax_rates_tenant_effective ON tax_rates(tenant_id, effective_from, effective_to);
CREATE UNIQUE INDEX idx_tax_rates_tenant_type_unique ON tax_rates(tenant_id, tax_type, effective_from) WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS tax_periods (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    tax_type VARCHAR(20) NOT NULL,
    period_year INT NOT NULL,
    period_month INT NOT NULL CHECK (period_month BETWEEN 1 AND 12),
    total_base NUMERIC(18,2) NOT NULL DEFAULT 0,
    total_tax NUMERIC(18,2) NOT NULL DEFAULT 0,
    total_deduction NUMERIC(18,2) NOT NULL DEFAULT 0,
    net_tax NUMERIC(18,2) NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'Open',
    filed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tax_periods_tenant ON tax_periods(tenant_id);
CREATE INDEX idx_tax_periods_tenant_type ON tax_periods(tenant_id, tax_type);
CREATE UNIQUE INDEX idx_tax_periods_unique ON tax_periods(tenant_id, tax_type, period_year, period_month);

CREATE TABLE IF NOT EXISTS tax_period_details (
    id BIGSERIAL PRIMARY KEY,
    period_id BIGINT NOT NULL REFERENCES tax_periods(id),
    transaction_date DATE NOT NULL,
    transaction_type VARCHAR(50) NOT NULL,
    base_amount NUMERIC(18,2) NOT NULL,
    tax_rate NUMERIC(10,4) NOT NULL,
    tax_amount NUMERIC(18,2) NOT NULL,
    deduction_amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    reference_id BIGINT
);

CREATE INDEX idx_tax_period_details_period ON tax_period_details(period_id);