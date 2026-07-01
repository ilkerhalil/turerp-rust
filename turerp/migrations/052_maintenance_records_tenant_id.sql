-- 052: Add tenant_id to maintenance_records
--
-- Closes the migration-needed cross-tenant leak on maintenance_records, which
-- 003 created with only asset_id (NOT NULL REFERENCES assets(id) ON DELETE
-- CASCADE) and no tenant_id. Both AssetsRepository maintenance methods skipped
-- tenant scoping and are reached by HTTP routes:
--   * create_maintenance_record — INSERT bound only asset_id, no tenant_id, no
--     asset-ownership check → a tenant-A admin could POST a maintenance record
--     against a tenant-B asset by guessing asset_id (the body asset_id is an
--     untrusted FK).
--   * get_maintenance_records — `WHERE asset_id = $1` only → a tenant-A user
--     could list a tenant-B asset's maintenance history by guessing asset_id.
--
-- Backfill from assets.tenant_id: maintenance_records.asset_id is NOT NULL
-- REFERENCES assets(id) and assets.tenant_id is NOT NULL REFERENCES
-- tenants(id), so every existing row receives a valid tenant_id and SET NOT
-- NULL / the new FK are safe. Mirrors 048 (hr attendance/leave), 049
-- (stock_levels), 050 (manufacturing child tables) and 051 (invoice_lines).
--
-- The code fix also adds a parent-ownership precheck on the write path
-- (service: get_asset(asset_id, tenant_id) → NotFound before INSERT) and
-- threads tenant_id through the read (AND tenant_id = $N). This column adds
-- defense-in-depth so maintenance reads/writes are scoped even without the
-- parent lookup.

ALTER TABLE maintenance_records ADD COLUMN IF NOT EXISTS tenant_id BIGINT;
UPDATE maintenance_records m
  SET tenant_id = a.tenant_id
  FROM assets a
  WHERE m.asset_id = a.id AND m.tenant_id IS NULL;
ALTER TABLE maintenance_records ALTER COLUMN tenant_id SET NOT NULL;
ALTER TABLE maintenance_records ADD CONSTRAINT fk_maintenance_records_tenant
  FOREIGN KEY (tenant_id) REFERENCES tenants(id);
CREATE INDEX IF NOT EXISTS idx_maintenance_records_tenant_id
  ON maintenance_records(tenant_id);
CREATE INDEX IF NOT EXISTS idx_maintenance_records_tenant_asset
  ON maintenance_records(tenant_id, asset_id);