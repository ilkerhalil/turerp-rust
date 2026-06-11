-- 037_pg_audit_dlq.sql
-- Dead-letter queue for the audit writer. If the writer fails to
-- persist a row to audit_logs, the row is serialized here for
-- replay once the writer is healthy.

CREATE TABLE IF NOT EXISTS pg_audit_dlq (
    id BIGSERIAL PRIMARY KEY,
    payload JSONB NOT NULL,
    error_message TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    replayed_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX IF NOT EXISTS idx_audit_dlq_unreplayed
    ON pg_audit_dlq (created_at) WHERE replayed_at IS NULL;
