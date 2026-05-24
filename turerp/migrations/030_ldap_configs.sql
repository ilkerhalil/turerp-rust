CREATE TABLE IF NOT EXISTS ldap_configs (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL UNIQUE REFERENCES tenants(id) ON DELETE CASCADE,
    ldap_url VARCHAR(500) NOT NULL,
    bind_dn VARCHAR(500) NOT NULL,
    bind_password_encrypted TEXT NOT NULL,
    base_dn VARCHAR(500) NOT NULL,
    user_filter VARCHAR(500) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_ldap_configs_tenant_id ON ldap_configs(tenant_id);

CREATE TRIGGER update_ldap_configs_updated_at
    BEFORE UPDATE ON ldap_configs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
