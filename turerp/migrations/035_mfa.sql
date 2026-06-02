-- MFA (Multi-Factor Authentication) tables

-- MFA settings per user
CREATE TABLE IF NOT EXISTS mfa_settings (
    user_id BIGINT NOT NULL,
    tenant_id BIGINT NOT NULL,
    totp_secret TEXT,
    mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    backup_codes TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    method TEXT NOT NULL DEFAULT 'none',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    PRIMARY KEY (user_id, tenant_id)
);

-- MFA challenges for verification during login
CREATE TABLE IF NOT EXISTS mfa_challenges (
    user_id BIGINT NOT NULL,
    tenant_id BIGINT NOT NULL,
    code TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    attempts INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, tenant_id, code)
);

-- Index for faster challenge lookups
CREATE INDEX IF NOT EXISTS idx_mfa_challenges_expires
    ON mfa_challenges (expires_at);

-- Index for tenant-scoped MFA lookups
CREATE INDEX IF NOT EXISTS idx_mfa_settings_tenant
    ON mfa_settings (tenant_id);
