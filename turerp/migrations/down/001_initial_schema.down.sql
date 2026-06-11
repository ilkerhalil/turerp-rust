-- 001_initial_schema.down.sql
-- Intentional no-op down — the initial schema is forward-only.
-- Rolling back this migration would require dropping every domain
-- table, which is destructive and not supported by the GA rollback
-- path. The operator runs `MIGRATIONS_DOWN=1` against a fresh DB
-- (not the live one) when down-replay is needed; the live DB
-- stays on the current schema.
SELECT 1;
