-- Core notification log (history + audit)
CREATE TABLE IF NOT EXISTS notifications (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    user_id BIGINT,
    channel VARCHAR(20) NOT NULL,
    priority VARCHAR(20) NOT NULL DEFAULT 'normal',
    status VARCHAR(20) NOT NULL DEFAULT 'queued',
    notification_type VARCHAR(50) NOT NULL,
    subject TEXT,
    body TEXT,
    recipient VARCHAR(255) NOT NULL,
    template_key VARCHAR(100),
    template_vars JSONB,
    provider_message_id VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    sent_at TIMESTAMPTZ,
    read_at TIMESTAMPTZ,
    last_error TEXT,
    attempts INT NOT NULL DEFAULT 0,
    job_id BIGINT
);
CREATE INDEX idx_notifications_tenant_user ON notifications(tenant_id, user_id, created_at DESC);
CREATE INDEX idx_notifications_status ON notifications(tenant_id, status);
CREATE INDEX idx_notifications_channel ON notifications(tenant_id, channel);

-- In-app notification bell (fast unread queries)
CREATE TABLE IF NOT EXISTS in_app_notifications (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    title VARCHAR(255) NOT NULL,
    message TEXT NOT NULL,
    notification_type VARCHAR(50) NOT NULL,
    read BOOLEAN NOT NULL DEFAULT false,
    link VARCHAR(500),
    related_notification_id BIGINT REFERENCES notifications(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    read_at TIMESTAMPTZ
);
CREATE INDEX idx_in_app_user_read ON in_app_notifications(tenant_id, user_id, read, created_at DESC);

-- Per-user notification preferences
CREATE TABLE IF NOT EXISTS notification_preferences (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    channel VARCHAR(20) NOT NULL,
    notification_type VARCHAR(50) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(tenant_id, user_id, channel, notification_type)
);
CREATE INDEX idx_notification_prefs_user ON notification_preferences(tenant_id, user_id);

-- Email templates (tenant overrides + global defaults)
CREATE TABLE IF NOT EXISTS email_templates (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT,
    template_key VARCHAR(100) NOT NULL,
    subject_template TEXT NOT NULL,
    body_template TEXT NOT NULL,
    html_template TEXT,
    locale VARCHAR(10) DEFAULT 'tr',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(tenant_id, template_key, locale)
);
