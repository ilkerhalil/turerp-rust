-- 044_reconciliation_rules_soft_delete.down.sql
-- Reverse of 044: drop the partial index, then the columns. Idempotent.
DROP INDEX IF EXISTS idx_reconciliation_rules_deleted_at;
ALTER TABLE reconciliation_rules DROP COLUMN IF EXISTS deleted_by;
ALTER TABLE reconciliation_rules DROP COLUMN IF EXISTS deleted_at;