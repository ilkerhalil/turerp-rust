-- 052_maintenance_records_tenant_id.down.sql
-- Reverse of 052: drop the composite index, the single-column index, the FK
-- constraint, then the tenant_id column on maintenance_records. Idempotent.
--
-- DESTRUCTIVE (data loss): dropping tenant_id loses the column we backfilled
-- from assets.tenant_id. Intended only for the offline/conservative down-replay
-- path documented in src/db/pool.rs (use scripts/backup_pg.sh for a real
-- point-in-time rollback). Re-adding the column later would require re-running
-- the backfill. The original cross-tenant leaks return (isolation was never
-- actually enforced pre-052). Run only against a snapshot where losing
-- maintenance_records.tenant_id is acceptable.

DROP INDEX IF EXISTS idx_maintenance_records_tenant_asset;
DROP INDEX IF EXISTS idx_maintenance_records_tenant_id;
ALTER TABLE maintenance_records DROP CONSTRAINT IF EXISTS fk_maintenance_records_tenant;
ALTER TABLE maintenance_records DROP COLUMN IF EXISTS tenant_id;