CREATE TABLE IF NOT EXISTS blockchain_hash_entries (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    period_id BIGINT NOT NULL,
    entry_id BIGINT NOT NULL,
    previous_hash TEXT,
    entry_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_hash_entries_tenant_period ON blockchain_hash_entries(tenant_id, period_id);
CREATE INDEX idx_hash_entries_entry_id ON blockchain_hash_entries(entry_id);

CREATE TABLE IF NOT EXISTS blockchain_merkle_trees (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    period_id BIGINT NOT NULL UNIQUE,
    root_hash TEXT NOT NULL,
    leaf_count INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_merkle_trees_tenant_period ON blockchain_merkle_trees(tenant_id, period_id);

CREATE TABLE IF NOT EXISTS blockchain_ledger_hash_states (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    period_id BIGINT NOT NULL UNIQUE,
    merkle_root TEXT NOT NULL,
    entry_count INTEGER NOT NULL,
    first_entry_hash TEXT,
    last_entry_hash TEXT,
    generated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_ledger_hash_states_tenant_period ON blockchain_ledger_hash_states(tenant_id, period_id);
