-- Cost Center / Profit Center module for Turerp ERP
-- Provides cost tracking and profitability analysis across business units

-- ============================================================================
-- COST CENTERS
-- ============================================================================

CREATE TABLE IF NOT EXISTS cost_centers (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    code VARCHAR(50) NOT NULL,
    name VARCHAR(200) NOT NULL,
    description TEXT,
    type VARCHAR(20) NOT NULL DEFAULT 'cost',  -- cost, profit
    parent_id BIGINT REFERENCES cost_centers(id),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    deleted_by BIGINT REFERENCES users(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX idx_cost_centers_code_tenant ON cost_centers(code, tenant_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_cost_centers_tenant_id ON cost_centers(tenant_id);
CREATE INDEX idx_cost_centers_type ON cost_centers(type);
CREATE INDEX idx_cost_centers_parent_id ON cost_centers(parent_id);
CREATE INDEX idx_cost_centers_deleted_at ON cost_centers(deleted_at) WHERE deleted_at IS NULL;

CREATE TRIGGER update_cost_centers_updated_at
    BEFORE UPDATE ON cost_centers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- COST CENTER ALLOCATIONS
-- ============================================================================

CREATE TABLE IF NOT EXISTS cost_center_allocations (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    source_type VARCHAR(50) NOT NULL,  -- invoice, journal_line, project_cost, work_order, payroll
    source_id BIGINT NOT NULL,
    cost_center_id BIGINT NOT NULL REFERENCES cost_centers(id),
    amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    percentage NUMERIC(5,2) NOT NULL DEFAULT 100,
    allocation_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    description VARCHAR(500),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_cost_center_allocations_tenant_id ON cost_center_allocations(tenant_id);
CREATE INDEX idx_cost_center_allocations_cost_center_id ON cost_center_allocations(cost_center_id);
CREATE INDEX idx_cost_center_allocations_source ON cost_center_allocations(source_type, source_id);
CREATE INDEX idx_cost_center_allocations_date ON cost_center_allocations(allocation_date);

-- ============================================================================
-- ADD cost_center_id TO EXISTING TABLES
-- ============================================================================

ALTER TABLE invoices ADD COLUMN IF NOT EXISTS cost_center_id BIGINT REFERENCES cost_centers(id);
CREATE INDEX IF NOT EXISTS idx_invoices_cost_center_id ON invoices(cost_center_id);

ALTER TABLE journal_lines ADD COLUMN IF NOT EXISTS cost_center_id BIGINT REFERENCES cost_centers(id);
CREATE INDEX IF NOT EXISTS idx_journal_lines_cost_center_id ON journal_lines(cost_center_id);

ALTER TABLE project_costs ADD COLUMN IF NOT EXISTS cost_center_id BIGINT REFERENCES cost_centers(id);
CREATE INDEX IF NOT EXISTS idx_project_costs_cost_center_id ON project_costs(cost_center_id);

ALTER TABLE work_orders ADD COLUMN IF NOT EXISTS cost_center_id BIGINT REFERENCES cost_centers(id);
CREATE INDEX IF NOT EXISTS idx_work_orders_cost_center_id ON work_orders(cost_center_id);

ALTER TABLE payrolls ADD COLUMN IF NOT EXISTS cost_center_id BIGINT REFERENCES cost_centers(id);
CREATE INDEX IF NOT EXISTS idx_payrolls_cost_center_id ON payrolls(cost_center_id);
