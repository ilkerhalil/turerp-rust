-- Composite indexes for paginated queries
-- Adds (tenant_id, created_at DESC) indexes for efficient pagination
-- and (tenant_id, status, created_at DESC) for filtered paginated queries

-- ============================================================================
-- CORE TABLES
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_users_tenant_created ON users(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_cari_tenant_created ON cari(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_categories_tenant_created ON categories(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_products_tenant_created ON products(tenant_id, created_at DESC);

-- ============================================================================
-- STOCK
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_warehouses_tenant_created ON warehouses(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_stock_movements_tenant_created ON stock_movements(tenant_id, created_at DESC);

-- ============================================================================
-- SALES & PURCHASE
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_sales_orders_tenant_created ON sales_orders(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sales_orders_tenant_status_created ON sales_orders(tenant_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_quotations_tenant_created ON quotations(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_quotations_tenant_status_created ON quotations(tenant_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_purchase_orders_tenant_created ON purchase_orders(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_purchase_orders_tenant_status_created ON purchase_orders(tenant_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_purchase_requests_tenant_created ON purchase_requests(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_purchase_requests_tenant_status_created ON purchase_requests(tenant_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_goods_receipts_tenant_created ON goods_receipts(tenant_id, created_at DESC);

-- ============================================================================
-- INVOICE
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_invoices_tenant_created ON invoices(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_invoices_tenant_status_created ON invoices(tenant_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_payments_tenant_created ON payments(tenant_id, created_at DESC);

-- ============================================================================
-- HR
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_employees_tenant_created ON employees(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_employees_tenant_status_created ON employees(tenant_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_attendance_tenant_created ON attendance(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_leave_requests_tenant_created ON leave_requests(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_leave_requests_tenant_status_created ON leave_requests(tenant_id, status, created_at DESC);

-- ============================================================================
-- ACCOUNTING
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_accounts_tenant_created ON accounts(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_journal_entries_tenant_created ON journal_entries(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_journal_entries_tenant_status_created ON journal_entries(tenant_id, status, created_at DESC);

-- ============================================================================
-- PROJECTS
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_projects_tenant_created ON projects(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_projects_tenant_status_created ON projects(tenant_id, status, created_at DESC);

-- ============================================================================
-- MANUFACTURING
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_work_orders_tenant_created ON work_orders(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_work_orders_tenant_status_created ON work_orders(tenant_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_bills_of_materials_tenant_created ON bills_of_materials(tenant_id, created_at DESC);

-- ============================================================================
-- CRM
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_leads_tenant_created ON leads(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_leads_tenant_status_created ON leads(tenant_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_crm_opportunities_tenant_created ON crm_opportunities(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_crm_opportunities_tenant_stage_created ON crm_opportunities(tenant_id, stage, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_campaigns_tenant_created ON campaigns(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_campaigns_tenant_status_created ON campaigns(tenant_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_support_tickets_tenant_created ON support_tickets(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_support_tickets_tenant_status_created ON support_tickets(tenant_id, status, created_at DESC);

-- ============================================================================
-- ASSETS
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_assets_tenant_created ON assets(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_assets_tenant_status_created ON assets(tenant_id, status, created_at DESC);

-- ============================================================================
-- FEATURE FLAGS & TENANTS
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_feature_flags_tenant_created ON feature_flags(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_tenants_created_at ON tenants(created_at DESC);