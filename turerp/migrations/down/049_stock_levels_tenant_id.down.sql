-- 049_stock_levels_tenant_id.down.sql
-- Reverse of 049: drop the indexes, the FK constraint, then the tenant_id
-- column. Idempotent.
--
-- DESTRUCTIVE (data loss): dropping tenant_id loses the column we backfilled
-- from products.tenant_id. Intended only for the offline/conservative
-- down-replay path documented in src/db/pool.rs (use scripts/backup_pg.sh
-- for a real point-in-time rollback). Re-adding the column later would
-- require re-running the backfill. The original cross-tenant leak returns
-- (isolation was never actually enforced pre-049). Run only against a
-- snapshot where losing the tenant_id column on stock_levels is acceptable.
DROP INDEX IF EXISTS idx_stock_levels_tenant_product;
DROP INDEX IF EXISTS idx_stock_levels_tenant_id;
ALTER TABLE stock_levels DROP CONSTRAINT IF EXISTS fk_stock_levels_tenant;
ALTER TABLE stock_levels DROP COLUMN IF EXISTS tenant_id;