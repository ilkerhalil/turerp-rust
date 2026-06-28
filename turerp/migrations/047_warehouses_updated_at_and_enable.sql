-- 047_warehouses_updated_at_and_enable.sql
--
-- Two-part fix for issue #152-5 (GET/POST/PUT /api/v1/stock/warehouses 500).
--
-- (1) warehouses (migration 003_business_modules.sql:87) was created WITHOUT
--     an `updated_at` column, yet 003:100 creates a `update_warehouses_updated_at
--     BEFORE UPDATE` trigger that calls update_updated_at_column()
--     (001: `NEW.updated_at = NOW()`). With no updated_at column the trigger
--     raises "record NEW has no field updated_at" on every UPDATE -> HTTP 500
--     on PUT /stock/warehouses/{id}, soft-delete and restore. Adding the column
--     (DEFAULT NOW() backfills existing rows on PG >= 11) lets the trigger work.
--     The trigger itself is unchanged.
--
-- (2) The handler SQL was ALSO broken: WarehouseRow
--     (src/domain/stock/postgres_repository.rs:25) declares
--     `total_count: Option<i64>` but create / find_by_id / find_by_tenant /
--     update / find_deleted SELECT|RETURNING did not include it, so sqlx FromRow
--     decode failed ("column total_count does not exist") -> 500 on GET/POST.
--     That is fixed in the source (this PR) by selecting `NULL::bigint AS
--     total_count` on the non-paginated paths (the paginated path already used
--     `COUNT(*) OVER() as total_count`). NULL::bigint avoids the int4
--     total_count decode class fixed in PR #173.
--
-- With both root causes fixed, the broken-endpoint gate `core.stock.warehouses`
-- (seeded 'disabled' in 036_flag_seed_defaults.sql:22, "flip to on after #152-5
-- fix") is flipped ON so the now-working handler is reachable. The other six
-- core.* gates remain disabled until their respective #152-N fixes land.
ALTER TABLE warehouses
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();

UPDATE feature_flags
    SET status = 'enabled', updated_at = NOW()
    WHERE name = 'core.stock.warehouses' AND tenant_id IS NULL;