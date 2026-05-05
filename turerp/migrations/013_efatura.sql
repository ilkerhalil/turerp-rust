-- e-Fatura tables
CREATE TABLE IF NOT EXISTS efatura (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    invoice_id BIGINT,
    uuid VARCHAR(36) NOT NULL,
    document_number VARCHAR(50) NOT NULL,
    issue_date DATE NOT NULL,
    profile_id VARCHAR(20) NOT NULL,
    -- Sender
    sender_vkn_tckn VARCHAR(11) NOT NULL,
    sender_name VARCHAR(255) NOT NULL,
    sender_tax_office VARCHAR(255) NOT NULL,
    sender_street VARCHAR(500) NOT NULL,
    sender_district VARCHAR(100),
    sender_city VARCHAR(100) NOT NULL,
    sender_country VARCHAR(100),
    sender_postal_code VARCHAR(20),
    sender_email VARCHAR(255),
    sender_phone VARCHAR(50),
    sender_register_number VARCHAR(50),
    sender_mersis_number VARCHAR(50),
    -- Receiver
    receiver_vkn_tckn VARCHAR(11) NOT NULL,
    receiver_name VARCHAR(255) NOT NULL,
    receiver_tax_office VARCHAR(255) NOT NULL,
    receiver_street VARCHAR(500) NOT NULL,
    receiver_district VARCHAR(100),
    receiver_city VARCHAR(100) NOT NULL,
    receiver_country VARCHAR(100),
    receiver_postal_code VARCHAR(20),
    receiver_email VARCHAR(255),
    receiver_phone VARCHAR(50),
    receiver_register_number VARCHAR(50),
    receiver_mersis_number VARCHAR(50),
    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'Draft',
    response_code VARCHAR(20),
    response_desc TEXT,
    xml_content TEXT,
    -- JSONB columns for structured data
    lines JSONB NOT NULL DEFAULT '[]'::jsonb,
    tax_totals JSONB NOT NULL DEFAULT '[]'::jsonb,
    legal_monetary_total JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_efatura_tenant ON efatura(tenant_id);
CREATE INDEX idx_efatura_tenant_status ON efatura(tenant_id, status);
CREATE UNIQUE INDEX idx_efatura_uuid ON efatura(tenant_id, uuid) WHERE deleted_at IS NULL;
CREATE INDEX idx_efatura_invoice ON efatura(tenant_id, invoice_id) WHERE deleted_at IS NULL;

COMMENT ON TABLE efatura IS 'e-Fatura documents (UBL-TR)';