-- 045_subscriptions_soft_delete.down.sql
-- Reverse of 045: drop the partial indexes, then the columns (feature
-- columns and soft-delete columns) from both tables. Idempotent. Drop
-- deleted_by before deleted_at so the FK column goes first; order within
-- each table does not otherwise matter.
--
-- DESTRUCTIVE (data loss): dropping these columns is lossy and is intended
-- only for the offline/conservative down-replay path documented in
-- src/db/pool.rs (use scripts/backup_pg.sh for a real point-in-time
-- rollback). Two consequences:
--   1. Any plan/subscription soft-deleted after 045 had deleted_at set;
--      dropping deleted_at revives those rows as live (they reappear in
--      list queries), losing the soft-delete audit state.
--   2. Any non-NULL included_quantity/overage_rate (plans) or
--      trial_start_date/trial_end_date (subscriptions) written by
--      create_plan/create_subscription after 045 are silently destroyed.
-- Run only against a snapshot where that loss is acceptable.
DROP INDEX IF EXISTS idx_subscriptions_deleted_at;
DROP INDEX IF EXISTS idx_subscription_plans_deleted_at;

ALTER TABLE subscriptions
    DROP COLUMN IF EXISTS deleted_by,
    DROP COLUMN IF EXISTS deleted_at,
    DROP COLUMN IF EXISTS trial_end_date,
    DROP COLUMN IF EXISTS trial_start_date;

ALTER TABLE subscription_plans
    DROP COLUMN IF EXISTS deleted_by,
    DROP COLUMN IF EXISTS deleted_at,
    DROP COLUMN IF EXISTS overage_rate,
    DROP COLUMN IF EXISTS included_quantity;