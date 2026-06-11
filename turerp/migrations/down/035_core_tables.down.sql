-- 035_core_tables.down.sql
-- Intentional no-op down — core domain tables (categories, units,
-- currencies, etc.) are forward-only. Dropping them would destroy
-- all customer data; the operator must run down-replay on a fresh DB.
SELECT 1;
