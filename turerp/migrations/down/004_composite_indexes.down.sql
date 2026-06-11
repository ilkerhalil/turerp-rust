-- 004_composite_indexes.down.sql
-- Intentional no-op down — composite indexes are forward-only.
-- The index definitions are recovered by re-running the up migration;
-- dropping them is safe but recreating them in the right order is
-- not, so the down is a no-op.
SELECT 1;
