-- 054_purchase_order_lines_tenant_id.down.sql
-- Reverse of 054: drop the composite index, the single-column index, the FK
-- constraint, then the tenant_id column on purchase_order_lines. Idempotent.
--
-- DESTRUCTIVE (data loss): dropping tenant_id loses the column we backfilled
-- from purchase_orders.tenant_id. Intended only for the offline/conservative
-- down-replay path documented in src/db/pool.rs (use scripts/backup_pg.sh for
-- a real point-in-time rollback). Re-adding the column later would require
-- re-running the backfill. The cross-tenant leak returns: the line repo would
-- again operate on a raw order_id with no SQL-level tenant filter, and the
-- destroy/soft_delete-before-parent ordering bugs would be live again. Run
-- only against a snapshot where losing purchase_order_lines.tenant_id is
-- acceptable.

DROP INDEX IF EXISTS idx_purchase_order_lines_tenant_order;
DROP INDEX IF EXISTS idx_purchase_order_lines_tenant_id;
ALTER TABLE purchase_order_lines DROP CONSTRAINT IF EXISTS fk_purchase_order_lines_tenant;
ALTER TABLE purchase_order_lines DROP COLUMN IF EXISTS tenant_id;