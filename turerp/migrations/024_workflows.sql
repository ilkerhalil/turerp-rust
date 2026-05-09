-- Workflow engine tables

-- Workflow templates: reusable approval process definitions
CREATE TABLE IF NOT EXISTS workflow_templates (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    entity_type VARCHAR(50) NOT NULL CHECK (entity_type IN ('invoice', 'purchase_order', 'expense', 'stock_transfer')),
    config_json JSONB NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_workflow_templates_tenant ON workflow_templates(tenant_id);
CREATE INDEX idx_workflow_templates_entity_type ON workflow_templates(entity_type);

-- Workflow instances: running approval processes
CREATE TABLE IF NOT EXISTS workflow_instances (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    template_id BIGINT NOT NULL REFERENCES workflow_templates(id),
    entity_id BIGINT NOT NULL,
    entity_type VARCHAR(50) NOT NULL CHECK (entity_type IN ('invoice', 'purchase_order', 'expense', 'stock_transfer')),
    status VARCHAR(50) NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'pending', 'approved', 'rejected', 'completed')),
    current_step INTEGER NOT NULL DEFAULT 1,
    assigned_user_id BIGINT,
    created_by BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_workflow_instances_tenant ON workflow_instances(tenant_id);
CREATE INDEX idx_workflow_instances_template ON workflow_instances(template_id);
CREATE INDEX idx_workflow_instances_status ON workflow_instances(status);
CREATE INDEX idx_workflow_instances_assigned ON workflow_instances(assigned_user_id);

-- Workflow steps: individual approval steps within an instance
CREATE TABLE IF NOT EXISTS workflow_steps (
    id BIGSERIAL PRIMARY KEY,
    instance_id BIGINT NOT NULL REFERENCES workflow_instances(id) ON DELETE CASCADE,
    step_number INTEGER NOT NULL,
    step_name VARCHAR(255) NOT NULL,
    approver_role VARCHAR(50),
    approver_user_id BIGINT,
    status VARCHAR(50) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
    comment TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_workflow_steps_instance ON workflow_steps(instance_id);
CREATE INDEX idx_workflow_steps_status ON workflow_steps(status);

-- Workflow audit log: immutable history of actions
CREATE TABLE IF NOT EXISTS workflow_audit_log (
    id BIGSERIAL PRIMARY KEY,
    instance_id BIGINT NOT NULL REFERENCES workflow_instances(id) ON DELETE CASCADE,
    step_id BIGINT REFERENCES workflow_steps(id),
    action VARCHAR(100) NOT NULL,
    user_id BIGINT NOT NULL,
    comment TEXT,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_workflow_audit_instance ON workflow_audit_log(instance_id);
CREATE INDEX idx_workflow_audit_timestamp ON workflow_audit_log(timestamp);
