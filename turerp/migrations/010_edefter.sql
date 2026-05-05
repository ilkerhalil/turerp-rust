-- e-Defter tables
CREATE TABLE IF NOT EXISTS ledger_periods (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    year INT NOT NULL,
    month INT NOT NULL CHECK (month BETWEEN 1 AND 12),
    period_type VARCHAR(30) NOT NULL DEFAULT 'YevmiyeDefteri',
    status VARCHAR(20) NOT NULL DEFAULT 'Draft',
    berat_signed_at TIMESTAMPTZ,
    sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ledger_periods_tenant ON ledger_periods(tenant_id);
CREATE INDEX idx_ledger_periods_tenant_year ON ledger_periods(tenant_id, year);
CREATE UNIQUE INDEX idx_ledger_periods_unique ON ledger_periods(tenant_id, period_type, year, month);

CREATE TABLE IF NOT EXISTS yevmiye_entries (
    id BIGSERIAL PRIMARY KEY,
    period_id BIGINT NOT NULL REFERENCES ledger_periods(id),
    entry_number BIGINT NOT NULL,
    entry_date DATE NOT NULL,
    explanation TEXT NOT NULL,
    debit_total NUMERIC(18,2) NOT NULL DEFAULT 0,
    credit_total NUMERIC(18,2) NOT NULL DEFAULT 0,
    lines JSONB NOT NULL DEFAULT '[]'::jsonb
);

CREATE INDEX idx_yevmiye_entries_period ON yevmiye_entries(period_id);

CREATE TABLE IF NOT EXISTS berat_info (
    period_id BIGINT PRIMARY KEY REFERENCES ledger_periods(id),
    serial_number VARCHAR(100) NOT NULL,
    sign_time TIMESTAMPTZ NOT NULL,
    signer VARCHAR(255) NOT NULL,
    digest_value TEXT NOT NULL,
    signature_value TEXT NOT NULL
);

COMMENT ON TABLE ledger_periods IS 'e-Defter ledger periods';
COMMENT ON TABLE yevmiye_entries IS 'e-Defter yevmiye (journal) entries';
COMMENT ON TABLE berat_info IS 'e-Defter berat (certificate) information';