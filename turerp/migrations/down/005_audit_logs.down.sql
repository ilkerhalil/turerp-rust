-- 005_audit_logs.down.sql
-- Intentional no-op down — the audit_logs table is forward-only.
-- Dropping it would lose the entire audit trail.
SELECT 1;
