-- Outbox events table for reliable event delivery
CREATE TABLE IF NOT EXISTS outbox_events (
    id BIGSERIAL PRIMARY KEY,
    event JSONB NOT NULL,
    aggregate_type VARCHAR(100) NOT NULL,
    aggregate_id BIGINT NOT NULL,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    published_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_outbox_status ON outbox_events(status);
CREATE INDEX idx_outbox_tenant_status ON outbox_events(tenant_id, status);
CREATE INDEX idx_outbox_created_at ON outbox_events(created_at);
CREATE INDEX idx_outbox_pending ON outbox_events(status, created_at) WHERE status = 'pending';

-- Dead letter queue for failed events
CREATE TABLE IF NOT EXISTS dead_letter_queue (
    id BIGSERIAL PRIMARY KEY,
    original_event JSONB NOT NULL,
    aggregate_type VARCHAR(100) NOT NULL,
    aggregate_id BIGINT NOT NULL,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    error TEXT NOT NULL,
    original_attempts INTEGER NOT NULL DEFAULT 0,
    dead_lettered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retried_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_dlq_tenant ON dead_letter_queue(tenant_id);
CREATE INDEX idx_dlq_created ON dead_letter_queue(dead_lettered_at);

CREATE TRIGGER update_outbox_events_updated_at BEFORE UPDATE ON outbox_events
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_dead_letter_queue_updated_at BEFORE UPDATE ON dead_letter_queue
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
