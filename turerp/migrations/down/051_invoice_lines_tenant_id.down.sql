-- 051_invoice_lines_tenant_id.down.sql
-- Reverse of 051: drop the indexes, the FK constraint, then the tenant_id
-- column on invoice_lines. Idempotent.
--
-- DESTRUCTIVE (data loss): dropping tenant_id loses the column we backfilled
-- from invoices.tenant_id. Intended only for the offline/conservative
-- down-replay path documented in src/db/pool.rs (use scripts/backup_pg.sh
-- for a real point-in-time rollback). Re-adding the column later would
-- require re-running the backfill. The original cross-tenant leak returns
-- (isolation was never actually enforced pre-051). Run only against a
-- snapshot where losing invoice_lines.tenant_id is acceptable.

DROP INDEX IF EXISTS idx_invoice_lines_tenant_invoice;
DROP INDEX IF EXISTS idx_invoice_lines_tenant_id;
ALTER TABLE invoice_lines DROP CONSTRAINT IF EXISTS fk_invoice_lines_tenant;
ALTER TABLE invoice_lines DROP COLUMN IF EXISTS tenant_id;