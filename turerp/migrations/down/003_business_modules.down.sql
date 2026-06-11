-- 003_business_modules.down.sql
-- Intentional no-op down — the business module tables are forward-only.
-- Dropping them would destroy all customer data; the operator must
-- run the down-replay on a fresh DB.
SELECT 1;
