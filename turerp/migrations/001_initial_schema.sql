-- Initial schema for Turerp ERP
-- Run with: psql -d turerp -f migrations/001_initial_schema.sql

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Tenants table
CREATE TABLE IF NOT EXISTS tenants (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    subdomain VARCHAR(255) UNIQUE NOT NULL,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_tenants_subdomain ON tenants(subdomain);
CREATE INDEX idx_tenants_is_active ON tenants(is_active);

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id BIGSERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL,
    email VARCHAR(255) NOT NULL,
    full_name VARCHAR(100),
    password VARCHAR(255) NOT NULL,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    role VARCHAR(20) NOT NULL DEFAULT 'user',
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- Unique constraints for users (per tenant)
CREATE UNIQUE INDEX idx_users_username_tenant ON users(username, tenant_id);
CREATE UNIQUE INDEX idx_users_email_tenant ON users(email, tenant_id);

-- Additional indexes for users
CREATE INDEX idx_users_tenant_id ON users(tenant_id);
CREATE INDEX idx_users_is_active ON users(is_active);

-- Update trigger for updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply updated_at trigger to tables
CREATE TRIGGER update_tenants_updated_at
    BEFORE UPDATE ON tenants
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Insert default tenant
INSERT INTO tenants (id, name, subdomain, is_active)
VALUES (1, 'Default Tenant', 'default', true)
ON CONFLICT (id) DO NOTHING;

-- Insert default admin user for DEVELOPMENT ONLY
-- ⚠️ SECURITY WARNING: Remove this default user in production deployments!
-- For production, use environment variables to create initial admin:
--   INITIAL_ADMIN_USERNAME, INITIAL_ADMIN_EMAIL, INITIAL_ADMIN_PASSWORD
-- Or run: DELETE FROM users WHERE username = 'admin' before deployment
INSERT INTO users (username, email, full_name, password, tenant_id, role, is_active)
VALUES (
    'admin',
    'admin@turerp.local',
    'System Administrator',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYIBdtk4dKG',  -- password: Admin123!
    1,
    'admin',
    true
)
ON CONFLICT DO NOTHING;

-- Cari (Customer/Vendor) table
CREATE TABLE IF NOT EXISTS cari (
    id BIGSERIAL PRIMARY KEY,
    code VARCHAR(50) NOT NULL,
    name VARCHAR(200) NOT NULL,
    cari_type VARCHAR(20) NOT NULL DEFAULT 'customer',
    tax_number VARCHAR(20),
    tax_office VARCHAR(100),
    identity_number VARCHAR(11),
    email VARCHAR(255),
    phone VARCHAR(20),
    address VARCHAR(500),
    city VARCHAR(100),
    country VARCHAR(100),
    postal_code VARCHAR(20),
    credit_limit DOUBLE PRECISION DEFAULT 0.0,
    current_balance DOUBLE PRECISION DEFAULT 0.0,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    created_by BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE
);

-- Unique constraints for cari (per tenant)
CREATE UNIQUE INDEX idx_cari_code_tenant ON cari(code, tenant_id);

-- Additional indexes for cari
CREATE INDEX idx_cari_tenant_id ON cari(tenant_id);
CREATE INDEX idx_cari_tenant_type ON cari(tenant_id, cari_type);  -- Composite index for find_by_type
CREATE INDEX idx_cari_type ON cari(cari_type);
CREATE INDEX idx_cari_status ON cari(status);
CREATE INDEX idx_cari_name ON cari(name);

-- Apply updated_at trigger to cari
CREATE TRIGGER update_cari_updated_at
    BEFORE UPDATE ON cari
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();