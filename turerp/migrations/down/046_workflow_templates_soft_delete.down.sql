-- 046_workflow_templates_soft_delete.down.sql
-- Reverse of 046: drop the partial index, then the columns. Idempotent.
--
-- DESTRUCTIVE (data loss): dropping deleted_at/deleted_by is lossy and is
-- intended only for the offline/conservative down-replay path documented
-- in src/db/pool.rs (use scripts/backup_pg.sh for a real point-in-time
-- rollback). Any template soft-deleted after 046 had deleted_at set;
-- dropping deleted_at revives those rows as live (they reappear in list
-- queries), losing the soft-delete audit state. deleted_by holds no user
-- data today (never populated — see 046 NOTE) so dropping it loses nothing.
-- Run only against a snapshot where reviving soft-deleted templates is
-- acceptable.
DROP INDEX IF EXISTS idx_workflow_templates_deleted_at;
ALTER TABLE workflow_templates DROP COLUMN IF EXISTS deleted_by;
ALTER TABLE workflow_templates DROP COLUMN IF EXISTS deleted_at;