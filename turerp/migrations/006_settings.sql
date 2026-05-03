-- Settings / Configuration Management table

CREATE TABLE IF NOT EXISTS settings (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    key VARCHAR(255) NOT NULL,
    value JSONB NOT NULL,
    default_value JSONB,
    data_type VARCHAR(50) NOT NULL DEFAULT 'string',
    group_name VARCHAR(50) NOT NULL DEFAULT 'general',
    description TEXT NOT NULL DEFAULT '',
    is_sensitive BOOLEAN NOT NULL DEFAULT FALSE,
    is_editable BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(tenant_id, key)
);

-- Indexes for tenant-scoped lookups
CREATE INDEX IF NOT EXISTS idx_settings_tenant_group ON settings(tenant_id, group_name);
CREATE INDEX IF NOT EXISTS idx_settings_tenant_key ON settings(tenant_id, key);
