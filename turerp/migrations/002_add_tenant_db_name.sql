-- Add db_name column to tenants table
-- This allows each tenant to have its own database name for multi-tenant isolation

ALTER TABLE tenants ADD COLUMN IF NOT EXISTS db_name VARCHAR(100) DEFAULT 'turerp';