-- Observability module: SLI/SLO tracking, alerting, and health checks

-- Health check history
CREATE TABLE IF NOT EXISTS health_checks (
    id BIGSERIAL PRIMARY KEY,
    component VARCHAR(100) NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('healthy', 'degraded', 'unhealthy')),
    latency_ms BIGINT NOT NULL DEFAULT 0,
    message TEXT,
    checked_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_health_checks_component ON health_checks(component, checked_at DESC);
CREATE INDEX idx_health_checks_checked_at ON health_checks(checked_at DESC);

-- SLI Definitions
CREATE TABLE IF NOT EXISTS sli_definitions (
    id VARCHAR(64) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    metric_type VARCHAR(50) NOT NULL CHECK (metric_type IN ('availability', 'latency', 'error_rate', 'throughput')),
    source VARCHAR(255) NOT NULL,
    window_minutes INT NOT NULL DEFAULT 5,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- SLI Measurements (time-series, consider retention policy)
CREATE TABLE IF NOT EXISTS sli_measurements (
    id BIGSERIAL PRIMARY KEY,
    sli_id VARCHAR(64) NOT NULL REFERENCES sli_definitions(id) ON DELETE CASCADE,
    value DOUBLE PRECISION NOT NULL,
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_sli_measurements_sli_time ON sli_measurements(sli_id, recorded_at DESC);

-- SLO Definitions
CREATE TABLE IF NOT EXISTS slo_definitions (
    id VARCHAR(64) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    sli_id VARCHAR(64) NOT NULL REFERENCES sli_definitions(id) ON DELETE CASCADE,
    target_value DOUBLE PRECISION NOT NULL,
    target_operator VARCHAR(10) NOT NULL DEFAULT 'Gte' CHECK (target_operator IN ('Gte', 'Lte')),
    error_budget DOUBLE PRECISION NOT NULL,
    window_days INT NOT NULL DEFAULT 7,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_slo_definitions_sli ON slo_definitions(sli_id);

-- SLO Compliance snapshots
CREATE TABLE IF NOT EXISTS slo_compliance (
    id BIGSERIAL PRIMARY KEY,
    slo_id VARCHAR(64) NOT NULL REFERENCES slo_definitions(id) ON DELETE CASCADE,
    slo_name VARCHAR(255) NOT NULL,
    current_value DOUBLE PRECISION NOT NULL,
    target_value DOUBLE PRECISION NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('compliant', 'at_risk', 'breached')),
    error_budget_remaining DOUBLE PRECISION NOT NULL DEFAULT 0,
    measured_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_slo_compliance_slo ON slo_compliance(slo_id, measured_at DESC);
CREATE INDEX idx_slo_compliance_measured_at ON slo_compliance(measured_at DESC);

-- Alert Rules
CREATE TABLE IF NOT EXISTS alert_rules (
    id VARCHAR(64) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    metric VARCHAR(255) NOT NULL,
    condition VARCHAR(10) NOT NULL CHECK (condition IN ('gt', 'gte', 'lt', 'lte', 'eq')),
    threshold DOUBLE PRECISION NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    duration_sec INT NOT NULL DEFAULT 60,
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_alert_rules_metric ON alert_rules(metric);
CREATE INDEX idx_alert_rules_enabled ON alert_rules(enabled);

-- Alerts (instances)
CREATE TABLE IF NOT EXISTS alerts (
    id VARCHAR(64) PRIMARY KEY,
    rule_id VARCHAR(64) NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    rule_name VARCHAR(255) NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    state VARCHAR(20) NOT NULL DEFAULT 'firing' CHECK (state IN ('firing', 'resolved', 'silenced')),
    message TEXT NOT NULL,
    value DOUBLE PRECISION,
    fired_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    resolved_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_alerts_state ON alerts(state, fired_at DESC);
CREATE INDEX idx_alerts_rule ON alerts(rule_id);
CREATE INDEX idx_alerts_fired_at ON alerts(fired_at DESC);

-- Sparkline data (time-series metric snapshots)
CREATE TABLE IF NOT EXISTS sparklines (
    id BIGSERIAL PRIMARY KEY,
    metric VARCHAR(255) NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    recorded_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_sparklines_metric ON sparklines(metric, recorded_at DESC);
