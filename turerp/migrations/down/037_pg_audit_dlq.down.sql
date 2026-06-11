-- 037_pg_audit_dlq.down.sql
-- Reversible: drops the audit DLQ table and its index. Any
-- unreplayed rows are lost — the operator must run
-- `replay-audit-dlq` BEFORE the down, or accept the loss.
DROP INDEX IF EXISTS idx_audit_dlq_unreplayed;
DROP TABLE IF EXISTS pg_audit_dlq;
