-- 056_purchase_request_lines_tenant_id.down.sql
-- Reverse of 056: drop the composite index, the single-column index, the FK
-- constraint, then the tenant_id column on purchase_request_lines. Idempotent.
--
-- DESTRUCTIVE (data loss): dropping tenant_id loses the column we backfilled
-- from purchase_requests.tenant_id. Intended only for the offline/conservative
-- down-replay path documented in src/db/pool.rs (use scripts/backup_pg.sh for
-- a real point-in-time rollback). Re-adding the column later would require
-- re-running the backfill. The cross-tenant leak returns: the line repo would
-- again operate on a raw request_id with no SQL-level tenant filter, and the
-- destroy/soft_delete-before-parent ordering bugs would be live again. Run
-- only against a snapshot where losing purchase_request_lines.tenant_id is
-- acceptable.

DROP INDEX IF EXISTS idx_purchase_request_lines_tenant_request;
DROP INDEX IF EXISTS idx_purchase_request_lines_tenant_id;
ALTER TABLE purchase_request_lines DROP CONSTRAINT IF EXISTS fk_purchase_request_lines_tenant;
ALTER TABLE purchase_request_lines DROP COLUMN IF EXISTS tenant_id;