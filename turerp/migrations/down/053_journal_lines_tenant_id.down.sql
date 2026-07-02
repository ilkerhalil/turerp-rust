-- 053_journal_lines_tenant_id.down.sql
-- Reverse of 053: drop the composite index, the single-column index, the FK
-- constraint, then the tenant_id column on journal_lines. Idempotent.
--
-- DESTRUCTIVE (data loss): dropping tenant_id loses the column we backfilled
-- from journal_entries.tenant_id. Intended only for the offline/conservative
-- down-replay path documented in src/db/pool.rs (use scripts/backup_pg.sh for
-- a real point-in-time rollback). Re-adding the column later would require
-- re-running the backfill. The latent cross-tenant backstop returns (SQL
-- scoping was never enforced pre-053; live paths were already parent-gated).
-- Run only against a snapshot where losing journal_lines.tenant_id is
-- acceptable.

DROP INDEX IF EXISTS idx_journal_lines_tenant_entry;
DROP INDEX IF EXISTS idx_journal_lines_tenant_id;
ALTER TABLE journal_lines DROP CONSTRAINT IF EXISTS fk_journal_lines_tenant;
ALTER TABLE journal_lines DROP COLUMN IF EXISTS tenant_id;