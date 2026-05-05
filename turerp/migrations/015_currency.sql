-- Migration: 015_currency.sql
-- Multi-currency support: currencies and exchange_rates tables

-- Create currencies table
CREATE TABLE IF NOT EXISTS currencies (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    code VARCHAR(3) NOT NULL,
    name VARCHAR(100) NOT NULL,
    symbol VARCHAR(10) NOT NULL,
    decimal_places INTEGER NOT NULL DEFAULT 2,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_base BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    CONSTRAINT currencies_tenant_code_unique UNIQUE (tenant_id, code)
);

-- Create exchange_rates table
CREATE TABLE IF NOT EXISTS exchange_rates (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    from_currency VARCHAR(3) NOT NULL,
    to_currency VARCHAR(3) NOT NULL,
    rate NUMERIC(20, 10) NOT NULL,
    effective_date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT exchange_rates_tenant_from_to_date_unique UNIQUE (tenant_id, from_currency, to_currency, effective_date)
);

-- Add currency fields to existing financial tables
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS exchange_rate NUMERIC(20, 10) NOT NULL DEFAULT 1.0;
ALTER TABLE payments ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'TRY';
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'TRY';
ALTER TABLE sales_orders ADD COLUMN IF NOT EXISTS exchange_rate NUMERIC(20, 10) NOT NULL DEFAULT 1.0;
ALTER TABLE purchase_orders ADD COLUMN IF NOT EXISTS currency VARCHAR(3) NOT NULL DEFAULT 'TRY';
ALTER TABLE purchase_orders ADD COLUMN IF NOT EXISTS exchange_rate NUMERIC(20, 10) NOT NULL DEFAULT 1.0;
ALTER TABLE cari_accounts ADD COLUMN IF NOT EXISTS default_currency VARCHAR(3) NOT NULL DEFAULT 'TRY';

-- Add currency fields to tenants
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS base_currency VARCHAR(3) NOT NULL DEFAULT 'TRY';
ALTER TABLE tenants ADD COLUMN IF NOT EXISTS supported_currencies TEXT[] NOT NULL DEFAULT ARRAY['TRY'];

-- Composite indexes for tenant-isolated lookups
CREATE INDEX IF NOT EXISTS idx_currencies_tenant_active ON currencies (tenant_id, is_active);
CREATE INDEX IF NOT EXISTS idx_currencies_tenant_base ON currencies (tenant_id, is_base) WHERE is_base = TRUE;
CREATE INDEX IF NOT EXISTS idx_exchange_rates_tenant_from_to ON exchange_rates (tenant_id, from_currency, to_currency);
CREATE INDEX IF NOT EXISTS idx_exchange_rates_tenant_date ON exchange_rates (tenant_id, effective_date);
CREATE INDEX IF NOT EXISTS idx_exchange_rates_effective ON exchange_rates (tenant_id, from_currency, to_currency, effective_date DESC);
