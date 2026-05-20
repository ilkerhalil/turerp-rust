-- Migration 028: Add missing tables for barcode, IP whitelist, e-archive, customer portal, vendor portal

-- Barcode configurations
CREATE TABLE IF NOT EXISTS barcode_configs (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id BIGINT NOT NULL,
    barcode_type VARCHAR(20) NOT NULL,
    code VARCHAR(255) NOT NULL,
    image_data TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE (tenant_id, entity_type, entity_id)
);

CREATE INDEX idx_barcode_configs_tenant ON barcode_configs(tenant_id);
CREATE INDEX idx_barcode_configs_entity ON barcode_configs(tenant_id, entity_type, entity_id);

-- IP whitelist entries
CREATE TABLE IF NOT EXISTS ip_whitelist_entries (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    ip_address VARCHAR(64) NOT NULL,
    description VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_ip_whitelist_tenant ON ip_whitelist_entries(tenant_id);
CREATE INDEX idx_ip_whitelist_tenant_active ON ip_whitelist_entries(tenant_id, is_active);

-- E-Archive documents
CREATE TABLE IF NOT EXISTS earchive_documents (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    document_type VARCHAR(30) NOT NULL,
    related_invoice_id BIGINT,
    uuid VARCHAR(64) NOT NULL UNIQUE,
    xml_content TEXT NOT NULL,
    signature TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'Draft',
    gib_response TEXT,
    error_message TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    sent_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_earchive_tenant ON earchive_documents(tenant_id);
CREATE INDEX idx_earchive_status ON earchive_documents(tenant_id, status);
CREATE INDEX idx_earchive_uuid ON earchive_documents(uuid);

-- Customer portal users
CREATE TABLE IF NOT EXISTS portal_users (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    cari_id BIGINT NOT NULL,
    email VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    full_name VARCHAR(255) NOT NULL,
    phone VARCHAR(50),
    language VARCHAR(10) NOT NULL DEFAULT 'en',
    timezone VARCHAR(50) NOT NULL DEFAULT 'Europe/Istanbul',
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_login_at TIMESTAMP WITH TIME ZONE,
    UNIQUE (tenant_id, email)
);

CREATE INDEX idx_portal_users_tenant ON portal_users(tenant_id);
CREATE INDEX idx_portal_users_email ON portal_users(tenant_id, email);
CREATE INDEX idx_portal_users_cari ON portal_users(tenant_id, cari_id);

-- Support tickets
CREATE TABLE IF NOT EXISTS support_tickets (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    portal_user_id BIGINT NOT NULL,
    cari_id BIGINT NOT NULL,
    ticket_number VARCHAR(50) NOT NULL,
    subject VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'open',
    priority VARCHAR(20) NOT NULL DEFAULT 'medium',
    category VARCHAR(20) NOT NULL DEFAULT 'general',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    resolved_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_support_tickets_tenant ON support_tickets(tenant_id);
CREATE INDEX idx_support_tickets_user ON support_tickets(tenant_id, portal_user_id);
CREATE INDEX idx_support_tickets_cari ON support_tickets(tenant_id, cari_id);

-- Vendor portal users
CREATE TABLE IF NOT EXISTS vendor_users (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    cari_id BIGINT NOT NULL,
    email VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    full_name VARCHAR(255) NOT NULL,
    phone VARCHAR(50),
    language VARCHAR(10) NOT NULL DEFAULT 'en',
    timezone VARCHAR(50) NOT NULL DEFAULT 'Europe/Istanbul',
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_login_at TIMESTAMP WITH TIME ZONE,
    UNIQUE (tenant_id, email)
);

CREATE INDEX idx_vendor_users_tenant ON vendor_users(tenant_id);
CREATE INDEX idx_vendor_users_email ON vendor_users(tenant_id, email);
CREATE INDEX idx_vendor_users_cari ON vendor_users(tenant_id, cari_id);

-- Delivery notes
CREATE TABLE IF NOT EXISTS delivery_notes (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    vendor_user_id BIGINT NOT NULL,
    cari_id BIGINT NOT NULL,
    note_number VARCHAR(50) NOT NULL,
    purchase_order_id BIGINT NOT NULL,
    description TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    shipped_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_delivery_notes_tenant ON delivery_notes(tenant_id);
CREATE INDEX idx_delivery_notes_vendor ON delivery_notes(tenant_id, vendor_user_id);
CREATE INDEX idx_delivery_notes_cari ON delivery_notes(tenant_id, cari_id);
