CREATE TABLE IF NOT EXISTS sgk_employee_registrations (
    id BIGSERIAL PRIMARY KEY,
    employee_id BIGINT NOT NULL,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    tc_kimlik_no VARCHAR(11) NOT NULL,
    sgk_sicil_no VARCHAR(20) NOT NULL,
    workplace_code VARCHAR(10) NOT NULL,
    profession_code VARCHAR(10) NOT NULL,
    registration_date TIMESTAMPTZ NOT NULL,
    termination_date TIMESTAMPTZ,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_sgk_registrations_tenant_id ON sgk_employee_registrations(tenant_id);
CREATE INDEX idx_sgk_registrations_employee_id ON sgk_employee_registrations(employee_id);

CREATE TRIGGER update_sgk_registrations_updated_at
    BEFORE UPDATE ON sgk_employee_registrations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS sgk_configs (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    year INTEGER NOT NULL,
    min_wage DECIMAL(15, 2) NOT NULL,
    sgk_earnings_ceiling DECIMAL(15, 2) NOT NULL,
    sgk_worker_rate DECIMAL(5, 4) NOT NULL,
    unemployment_worker_rate DECIMAL(5, 4) NOT NULL,
    stamp_tax_rate DECIMAL(5, 4) NOT NULL,
    agi_amount_single DECIMAL(15, 2) NOT NULL,
    agi_amount_married DECIMAL(15, 2) NOT NULL,
    agi_per_child DECIMAL(15, 2) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    UNIQUE(tenant_id, year)
);

CREATE INDEX idx_sgk_configs_tenant_id ON sgk_configs(tenant_id);

CREATE TRIGGER update_sgk_configs_updated_at
    BEFORE UPDATE ON sgk_configs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE IF NOT EXISTS employee_bonuses (
    id BIGSERIAL PRIMARY KEY,
    employee_id BIGINT NOT NULL,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    bonus_type VARCHAR(50) NOT NULL,
    amount DECIMAL(15, 2) NOT NULL,
    bonus_month INTEGER NOT NULL,
    bonus_year INTEGER NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_employee_bonuses_tenant_id ON employee_bonuses(tenant_id);
CREATE INDEX idx_employee_bonuses_employee_id ON employee_bonuses(employee_id);
CREATE INDEX idx_employee_bonuses_year_month ON employee_bonuses(bonus_year, bonus_month);
