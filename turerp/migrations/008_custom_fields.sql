-- Custom field definitions for dynamic module attributes
-- Stores field definitions; values are stored as JSONB on each entity row.

CREATE TABLE IF NOT EXISTS custom_field_definitions (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    module VARCHAR(50) NOT NULL,
    field_name VARCHAR(100) NOT NULL,
    field_label VARCHAR(200) NOT NULL,
    field_type VARCHAR(20) NOT NULL,
    required BOOLEAN NOT NULL DEFAULT false,
    options JSONB DEFAULT '[]',
    sort_order INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    deleted_at TIMESTAMPTZ,
    deleted_by BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (tenant_id, module, field_name) WHERE deleted_at IS NULL
);

CREATE INDEX IF NOT EXISTS idx_custom_fields_tenant_module
    ON custom_field_definitions (tenant_id, module)
    WHERE deleted_at IS NULL AND is_active = true;