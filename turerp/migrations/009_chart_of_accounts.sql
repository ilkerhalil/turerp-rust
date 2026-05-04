-- Chart of Accounts (TEK Skeleton)
CREATE TABLE chart_accounts (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    code VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    group_name VARCHAR(50) NOT NULL,
    parent_code VARCHAR(50),
    level SMALLINT NOT NULL DEFAULT 1,
    account_type VARCHAR(30) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    balance DECIMAL(19,4) NOT NULL DEFAULT 0,
    allow_posting BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    deleted_by BIGINT
);

-- Partial unique index: one active code per tenant
CREATE UNIQUE INDEX idx_chart_accounts_tenant_code
    ON chart_accounts (tenant_id, code) WHERE deleted_at IS NULL;

-- Filter by group
CREATE INDEX idx_chart_accounts_tenant_group
    ON chart_accounts (tenant_id, group_name) WHERE deleted_at IS NULL;

-- Parent code hierarchy
CREATE INDEX idx_chart_accounts_tenant_parent
    ON chart_accounts (tenant_id, parent_code) WHERE deleted_at IS NULL;

-- Standard time-series index
CREATE INDEX idx_chart_accounts_tenant_created
    ON chart_accounts (tenant_id, created_at DESC);