-- 045_subscriptions_soft_delete.sql
--
-- subscription_plans and subscriptions (migration 026_subscriptions.sql) were
-- created AFTER migration 020 (soft_delete_complete) but WITHOUT the
-- soft-delete columns, even though the Row structs and every read query
-- assume them. Additionally 026 omitted two feature columns per table that
-- the Row structs and INSERT/UPDATE/RETURNING statements already reference,
-- so the queries fail at plan time (`column "..." does not exist`) -> HTTP 500
-- on GET /api/v1/subscription-plans and /subscriptions (and find_*_by_id /
-- create / update / delete / billing / dunning paths).
--
-- Concretely, what the code already uses vs. what 026 created:
--
--   SubscriptionPlanRow (src/domain/subscription/postgres_repository.rs:21)
--     expects 15 columns; 026 created 11. Missing:
--       included_quantity  Option<i64>      -> BIGINT
--       overage_rate        Option<Decimal>  -> NUMERIC
--       deleted_at          Option<DateTime> -> TIMESTAMPTZ
--       deleted_by          Option<i64>      -> BIGINT REFERENCES users(id)
--     (create_plan INSERT, find_plan_by_id/find_plans_by_tenant SELECT,
--      update_plan SET/RETURNING, and delete_plan SET deleted_at=NOW() all
--      reference these.)
--
--   SubscriptionRow (postgres_repository.rs:71) expects 16 columns; 026
--   created 12. Missing:
--       trial_start_date    Option<NaiveDate> -> DATE
--       trial_end_date      Option<NaiveDate> -> DATE
--       deleted_at          Option<DateTime>  -> TIMESTAMPTZ
--       deleted_by          Option<i64>       -> BIGINT REFERENCES users(id)
--     (create_subscription INSERT, find_subscription_by_id /
--      find_subscriptions_by_tenant / find_active_by_customer /
--      find_due_for_billing / find_subscriptions_for_dunning /
--      find_trial_subscriptions SELECT, update_subscription /
--      renew_subscription SET/RETURNING, and delete_subscription
--      SET deleted_at=NOW() all reference these.)
--
-- Same soft-delete-missing-column class as reconciliation_rules (migration
-- 044) and tax_periods (migration 040): 020 soft_delete_complete skipped the
-- tables created later in 026, and 026 itself under-declared the feature
-- columns. The feature columns are nullable (Option<...>) so existing rows
-- decode as NULL with no data loss; the soft-delete columns follow the 020
-- convention (deleted_at TIMESTAMPTZ, deleted_by BIGINT FK users ON DELETE
-- SET NULL) plus the partial-index convention from 020/040/044.
--
-- NOTE (follow-up, pre-existing, NOT addressed here): the soft-delete
-- write paths (delete_plan, delete_subscription) SET deleted_at = NOW() but
-- take no deleted_by argument, so deleted_by is never populated on
-- soft-delete (stays NULL). That audit-attribution gap is surfaced, not
-- introduced, by adding the column; the column is required because the Row
-- structs SELECT it. Wiring deleted_by through those paths is tracked
-- separately (same residual class as reconciliation_rules delete_rule in 044).
ALTER TABLE subscription_plans
    ADD COLUMN IF NOT EXISTS included_quantity BIGINT,
    ADD COLUMN IF NOT EXISTS overage_rate NUMERIC(18,2),
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deleted_by BIGINT REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE subscriptions
    ADD COLUMN IF NOT EXISTS trial_start_date DATE,
    ADD COLUMN IF NOT EXISTS trial_end_date DATE,
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deleted_by BIGINT REFERENCES users(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_subscription_plans_deleted_at
    ON subscription_plans(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_subscriptions_deleted_at
    ON subscriptions(deleted_at) WHERE deleted_at IS NULL;