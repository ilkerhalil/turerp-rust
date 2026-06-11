-- 007_soft_delete.down.sql
-- Intentional no-op down — soft delete columns are forward-only.
-- Dropping them would orphan the soft-delete semantics.
SELECT 1;
