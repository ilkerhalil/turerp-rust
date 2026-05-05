-- API Keys table for programmatic access
CREATE TABLE IF NOT EXISTS api_keys (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    key_hash VARCHAR(255) NOT NULL,
    key_prefix VARCHAR(20) NOT NULL,
    scopes TEXT[] NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

-- Fast lookup by hash for authentication
CREATE UNIQUE INDEX idx_api_keys_hash ON api_keys(key_hash);

-- Tenant-scoped listings
CREATE INDEX idx_api_keys_tenant ON api_keys(tenant_id, created_at DESC);

-- Active keys per tenant for quick filtering
CREATE INDEX idx_api_keys_tenant_active ON api_keys(tenant_id, is_active) WHERE is_active = true;
