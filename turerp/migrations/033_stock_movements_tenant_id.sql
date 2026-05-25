-- Add tenant_id to stock_movements for multi-tenant isolation

ALTER TABLE stock_movements ADD COLUMN IF NOT EXISTS tenant_id BIGINT;

-- Backfill existing rows with default tenant_id
UPDATE stock_movements SET tenant_id = 1 WHERE tenant_id IS NULL;

-- Make tenant_id NOT NULL after backfill
ALTER TABLE stock_movements ALTER COLUMN tenant_id SET NOT NULL;

-- Add foreign key constraint
ALTER TABLE stock_movements ADD CONSTRAINT fk_stock_movements_tenant
    FOREIGN KEY (tenant_id) REFERENCES tenants(id);

-- Create composite index for tenant-scoped queries
CREATE INDEX IF NOT EXISTS idx_stock_movements_tenant_created
    ON stock_movements(tenant_id, created_at);
