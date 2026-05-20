CREATE TABLE IF NOT EXISTS login_attempts (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(255) NOT NULL,
    ip_address VARCHAR(64),
    tenant_id BIGINT NOT NULL,
    attempted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    success BOOLEAN NOT NULL DEFAULT false,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    UNIQUE (username, tenant_id)
);
CREATE INDEX idx_login_attempts_username ON login_attempts(username, tenant_id);
CREATE INDEX idx_login_attempts_time ON login_attempts(attempted_at);
