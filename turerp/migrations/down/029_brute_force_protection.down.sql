-- 029_brute_force_protection.down.sql
-- Intentional no-op down — login_attempts is forward-only.
-- Dropping it would disable the brute-force defense (PR #144).
SELECT 1;
