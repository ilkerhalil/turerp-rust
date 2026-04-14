-- Business modules schema for Turerp ERP
-- Creates all tables for: Product, Stock, Sales, Purchase, Invoice, HR,
-- Accounting, Project, Manufacturing, CRM, Assets, Feature Flags, Tenant Config

-- ============================================================================
-- PRODUCT DOMAIN
-- ============================================================================

CREATE TABLE IF NOT EXISTS categories (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    name VARCHAR(200) NOT NULL,
    parent_id BIGINT REFERENCES categories(id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_categories_name_tenant ON categories(name, tenant_id);
CREATE INDEX idx_categories_tenant_id ON categories(tenant_id);
CREATE INDEX idx_categories_parent_id ON categories(parent_id);

CREATE TRIGGER update_categories_updated_at
    BEFORE UPDATE ON categories
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Units
CREATE TABLE IF NOT EXISTS units (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    code VARCHAR(50) NOT NULL,
    name VARCHAR(100) NOT NULL,
    is_integer BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_units_code_tenant ON units(code, tenant_id);
CREATE INDEX idx_units_tenant_id ON units(tenant_id);

-- Products
CREATE TABLE IF NOT EXISTS products (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    code VARCHAR(50) NOT NULL,
    name VARCHAR(200) NOT NULL,
    description TEXT,
    category_id BIGINT REFERENCES categories(id),
    unit_id BIGINT REFERENCES units(id),
    barcode VARCHAR(100),
    purchase_price NUMERIC(18,4) NOT NULL DEFAULT 0,
    sale_price NUMERIC(18,4) NOT NULL DEFAULT 0,
    tax_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_products_code_tenant ON products(code, tenant_id);
CREATE INDEX idx_products_tenant_id ON products(tenant_id);
CREATE INDEX idx_products_category_id ON products(category_id);
CREATE INDEX idx_products_barcode ON products(barcode) WHERE barcode IS NOT NULL;
CREATE INDEX idx_products_is_active ON products(is_active);

CREATE TRIGGER update_products_updated_at
    BEFORE UPDATE ON products
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Product Variants
CREATE TABLE IF NOT EXISTS product_variants (
    id BIGSERIAL PRIMARY KEY,
    product_id BIGINT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    name VARCHAR(200) NOT NULL,
    sku VARCHAR(100),
    barcode VARCHAR(100),
    price_modifier NUMERIC(18,4) NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_product_variants_sku ON product_variants(sku) WHERE sku IS NOT NULL;
CREATE INDEX idx_product_variants_product_id ON product_variants(product_id);

-- ============================================================================
-- STOCK DOMAIN
-- ============================================================================

CREATE TABLE IF NOT EXISTS warehouses (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    code VARCHAR(50) NOT NULL,
    name VARCHAR(200) NOT NULL,
    address TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_warehouses_code_tenant ON warehouses(code, tenant_id);
CREATE INDEX idx_warehouses_tenant_id ON warehouses(tenant_id);

CREATE TRIGGER update_warehouses_updated_at
    BEFORE UPDATE ON warehouses
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Stock Levels
CREATE TABLE IF NOT EXISTS stock_levels (
    id BIGSERIAL PRIMARY KEY,
    warehouse_id BIGINT NOT NULL REFERENCES warehouses(id),
    product_id BIGINT NOT NULL REFERENCES products(id),
    quantity NUMERIC(18,4) NOT NULL DEFAULT 0,
    reserved_quantity NUMERIC(18,4) NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_stock_levels_warehouse_product ON stock_levels(warehouse_id, product_id);
CREATE INDEX idx_stock_levels_product_id ON stock_levels(product_id);

CREATE TRIGGER update_stock_levels_updated_at
    BEFORE UPDATE ON stock_levels
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Stock Movements
CREATE TABLE IF NOT EXISTS stock_movements (
    id BIGSERIAL PRIMARY KEY,
    warehouse_id BIGINT NOT NULL REFERENCES warehouses(id),
    product_id BIGINT NOT NULL REFERENCES products(id),
    movement_type VARCHAR(30) NOT NULL,  -- Purchase, Sale, Return, Adjustment, Transfer, ProductionIn, ProductionOut, Waste
    quantity NUMERIC(18,4) NOT NULL,
    reference_type VARCHAR(50),
    reference_id BIGINT,
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    created_by BIGINT NOT NULL REFERENCES users(id)
);

CREATE INDEX idx_stock_movements_warehouse_id ON stock_movements(warehouse_id);
CREATE INDEX idx_stock_movements_product_id ON stock_movements(product_id);
CREATE INDEX idx_stock_movements_type ON stock_movements(movement_type);
CREATE INDEX idx_stock_movements_created_at ON stock_movements(created_at);

-- ============================================================================
-- SALES DOMAIN
-- ============================================================================

CREATE TABLE IF NOT EXISTS sales_orders (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    order_number VARCHAR(50) NOT NULL,
    cari_id BIGINT NOT NULL REFERENCES cari(id),
    status VARCHAR(30) NOT NULL DEFAULT 'Draft',  -- Draft, PendingApproval, Approved, InProgress, Shipped, Delivered, Cancelled, OnHold
    order_date TIMESTAMPTZ NOT NULL,
    delivery_date TIMESTAMPTZ,
    subtotal NUMERIC(18,4) NOT NULL DEFAULT 0,
    tax_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    discount_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    total_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    notes TEXT,
    shipping_address TEXT,
    billing_address TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_sales_orders_number_tenant ON sales_orders(order_number, tenant_id);
CREATE INDEX idx_sales_orders_tenant_id ON sales_orders(tenant_id);
CREATE INDEX idx_sales_orders_cari_id ON sales_orders(cari_id);
CREATE INDEX idx_sales_orders_status ON sales_orders(status);

CREATE TRIGGER update_sales_orders_updated_at
    BEFORE UPDATE ON sales_orders
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Sales Order Lines
CREATE TABLE IF NOT EXISTS sales_order_lines (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE,
    product_id BIGINT REFERENCES products(id),
    description VARCHAR(500) NOT NULL,
    quantity NUMERIC(18,4) NOT NULL,
    unit_price NUMERIC(18,4) NOT NULL,
    tax_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    discount_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    line_total NUMERIC(18,4) NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_sales_order_lines_order_id ON sales_order_lines(order_id);

-- Quotations
CREATE TABLE IF NOT EXISTS quotations (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    quotation_number VARCHAR(50) NOT NULL,
    cari_id BIGINT NOT NULL REFERENCES cari(id),
    status VARCHAR(30) NOT NULL DEFAULT 'Draft',  -- Draft, Sent, UnderReview, Accepted, Rejected, Expired, ConvertedToOrder
    valid_until TIMESTAMPTZ NOT NULL,
    subtotal NUMERIC(18,4) NOT NULL DEFAULT 0,
    tax_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    discount_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    total_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    notes TEXT,
    terms TEXT,
    sales_order_id BIGINT REFERENCES sales_orders(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_quotations_number_tenant ON quotations(quotation_number, tenant_id);
CREATE INDEX idx_quotations_tenant_id ON quotations(tenant_id);
CREATE INDEX idx_quotations_cari_id ON quotations(cari_id);
CREATE INDEX idx_quotations_status ON quotations(status);

CREATE TRIGGER update_quotations_updated_at
    BEFORE UPDATE ON quotations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Quotation Lines
CREATE TABLE IF NOT EXISTS quotation_lines (
    id BIGSERIAL PRIMARY KEY,
    quotation_id BIGINT NOT NULL REFERENCES quotations(id) ON DELETE CASCADE,
    product_id BIGINT REFERENCES products(id),
    description VARCHAR(500) NOT NULL,
    quantity NUMERIC(18,4) NOT NULL,
    unit_price NUMERIC(18,4) NOT NULL,
    tax_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    discount_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    line_total NUMERIC(18,4) NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_quotation_lines_quotation_id ON quotation_lines(quotation_id);

-- ============================================================================
-- PURCHASE DOMAIN
-- ============================================================================

CREATE TABLE IF NOT EXISTS purchase_orders (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    order_number VARCHAR(50) NOT NULL,
    cari_id BIGINT NOT NULL REFERENCES cari(id),
    status VARCHAR(30) NOT NULL DEFAULT 'Draft',  -- Draft, PendingApproval, Approved, SentToVendor, PartialReceived, Received, Cancelled, OnHold
    order_date TIMESTAMPTZ NOT NULL,
    expected_delivery_date TIMESTAMPTZ,
    subtotal NUMERIC(18,4) NOT NULL DEFAULT 0,
    tax_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    discount_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    total_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_purchase_orders_number_tenant ON purchase_orders(order_number, tenant_id);
CREATE INDEX idx_purchase_orders_tenant_id ON purchase_orders(tenant_id);
CREATE INDEX idx_purchase_orders_cari_id ON purchase_orders(cari_id);
CREATE INDEX idx_purchase_orders_status ON purchase_orders(status);

CREATE TRIGGER update_purchase_orders_updated_at
    BEFORE UPDATE ON purchase_orders
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Purchase Order Lines
CREATE TABLE IF NOT EXISTS purchase_order_lines (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    product_id BIGINT REFERENCES products(id),
    description VARCHAR(500) NOT NULL,
    quantity NUMERIC(18,4) NOT NULL,
    received_quantity NUMERIC(18,4) NOT NULL DEFAULT 0,
    unit_price NUMERIC(18,4) NOT NULL,
    tax_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    discount_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    line_total NUMERIC(18,4) NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_purchase_order_lines_order_id ON purchase_order_lines(order_id);

-- Purchase Requests
CREATE TABLE IF NOT EXISTS purchase_requests (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    request_number VARCHAR(50) NOT NULL,
    status VARCHAR(30) NOT NULL DEFAULT 'Draft',  -- Draft, PendingApproval, Approved, Rejected, ConvertedToOrder
    requested_by BIGINT NOT NULL REFERENCES users(id),
    department VARCHAR(100),
    priority VARCHAR(20) NOT NULL DEFAULT 'Normal',
    reason TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_purchase_requests_number_tenant ON purchase_requests(request_number, tenant_id);
CREATE INDEX idx_purchase_requests_tenant_id ON purchase_requests(tenant_id);
CREATE INDEX idx_purchase_requests_status ON purchase_requests(status);

CREATE TRIGGER update_purchase_requests_updated_at
    BEFORE UPDATE ON purchase_requests
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Purchase Request Lines
CREATE TABLE IF NOT EXISTS purchase_request_lines (
    id BIGSERIAL PRIMARY KEY,
    request_id BIGINT NOT NULL REFERENCES purchase_requests(id) ON DELETE CASCADE,
    product_id BIGINT REFERENCES products(id),
    description VARCHAR(500) NOT NULL,
    quantity NUMERIC(18,4) NOT NULL,
    notes TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_purchase_request_lines_request_id ON purchase_request_lines(request_id);

-- Goods Receipts
CREATE TABLE IF NOT EXISTS goods_receipts (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    receipt_number VARCHAR(50) NOT NULL,
    purchase_order_id BIGINT NOT NULL REFERENCES purchase_orders(id),
    status VARCHAR(30) NOT NULL DEFAULT 'Pending',  -- Pending, Partial, Completed, Cancelled
    receipt_date TIMESTAMPTZ NOT NULL,
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_goods_receipts_number_tenant ON goods_receipts(receipt_number, tenant_id);
CREATE INDEX idx_goods_receipts_tenant_id ON goods_receipts(tenant_id);
CREATE INDEX idx_goods_receipts_purchase_order_id ON goods_receipts(purchase_order_id);

-- Goods Receipt Lines
CREATE TABLE IF NOT EXISTS goods_receipt_lines (
    id BIGSERIAL PRIMARY KEY,
    receipt_id BIGINT NOT NULL REFERENCES goods_receipts(id) ON DELETE CASCADE,
    order_line_id BIGINT NOT NULL REFERENCES purchase_order_lines(id),
    product_id BIGINT REFERENCES products(id),
    quantity NUMERIC(18,4) NOT NULL,
    condition VARCHAR(30) NOT NULL DEFAULT 'Good',
    notes TEXT
);

CREATE INDEX idx_goods_receipt_lines_receipt_id ON goods_receipt_lines(receipt_id);

-- ============================================================================
-- INVOICE DOMAIN
-- ============================================================================

CREATE TABLE IF NOT EXISTS invoices (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    invoice_number VARCHAR(50) NOT NULL,
    invoice_type VARCHAR(30) NOT NULL DEFAULT 'SalesInvoice',  -- SalesInvoice, PurchaseInvoice, SalesReturn, PurchaseReturn
    status VARCHAR(30) NOT NULL DEFAULT 'Draft',  -- Draft, Pending, Approved, Sent, PartiallyPaid, Paid, Overdue, Cancelled, Refunded
    cari_id BIGINT NOT NULL REFERENCES cari(id),
    issue_date TIMESTAMPTZ NOT NULL,
    due_date TIMESTAMPTZ NOT NULL,
    subtotal NUMERIC(18,4) NOT NULL DEFAULT 0,
    tax_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    discount_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    total_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    paid_amount NUMERIC(18,4) NOT NULL DEFAULT 0,
    currency VARCHAR(10) NOT NULL DEFAULT 'TRY',
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_invoices_number_tenant ON invoices(invoice_number, tenant_id);
CREATE INDEX idx_invoices_tenant_id ON invoices(tenant_id);
CREATE INDEX idx_invoices_cari_id ON invoices(cari_id);
CREATE INDEX idx_invoices_status ON invoices(status);
CREATE INDEX idx_invoices_issue_date ON invoices(issue_date);

CREATE TRIGGER update_invoices_updated_at
    BEFORE UPDATE ON invoices
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Invoice Lines
CREATE TABLE IF NOT EXISTS invoice_lines (
    id BIGSERIAL PRIMARY KEY,
    invoice_id BIGINT NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    product_id BIGINT REFERENCES products(id),
    description VARCHAR(500) NOT NULL,
    quantity NUMERIC(18,4) NOT NULL,
    unit_price NUMERIC(18,4) NOT NULL,
    tax_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    discount_rate NUMERIC(18,4) NOT NULL DEFAULT 0,
    line_total NUMERIC(18,4) NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_invoice_lines_invoice_id ON invoice_lines(invoice_id);

-- Payments
CREATE TABLE IF NOT EXISTS payments (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    invoice_id BIGINT NOT NULL REFERENCES invoices(id),
    amount NUMERIC(18,4) NOT NULL,
    payment_date TIMESTAMPTZ NOT NULL,
    payment_method VARCHAR(50) NOT NULL,
    reference_number VARCHAR(100),
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_payments_tenant_id ON payments(tenant_id);
CREATE INDEX idx_payments_invoice_id ON payments(invoice_id);

-- ============================================================================
-- HR DOMAIN
-- ============================================================================

CREATE TABLE IF NOT EXISTS employees (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    user_id BIGINT REFERENCES users(id),
    employee_number VARCHAR(50) NOT NULL,
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) NOT NULL,
    phone VARCHAR(20),
    department VARCHAR(100),
    position VARCHAR(100),
    hire_date TIMESTAMPTZ NOT NULL,
    termination_date TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'Active',  -- Active, OnLeave, Terminated, Suspended
    salary NUMERIC(18,4) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_employees_number_tenant ON employees(employee_number, tenant_id);
CREATE UNIQUE INDEX idx_employees_email_tenant ON employees(email, tenant_id);
CREATE INDEX idx_employees_tenant_id ON employees(tenant_id);
CREATE INDEX idx_employees_department ON employees(department);
CREATE INDEX idx_employees_status ON employees(status);

CREATE TRIGGER update_employees_updated_at
    BEFORE UPDATE ON employees
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Attendance
CREATE TABLE IF NOT EXISTS attendance (
    id BIGSERIAL PRIMARY KEY,
    employee_id BIGINT NOT NULL REFERENCES employees(id),
    date TIMESTAMPTZ NOT NULL,
    check_in TIMESTAMPTZ,
    check_out TIMESTAMPTZ,
    hours_worked NUMERIC(18,4) NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'Present',  -- Present, Absent, Late, OnLeave, Holiday
    notes TEXT
);

CREATE UNIQUE INDEX idx_attendance_employee_date ON attendance(employee_id, date::date);
CREATE INDEX idx_attendance_date ON attendance(date);

-- Leave Types
CREATE TABLE IF NOT EXISTS leave_types (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    max_days_per_year NUMERIC(18,4) NOT NULL DEFAULT 0,
    requires_approval BOOLEAN NOT NULL DEFAULT true
);

CREATE UNIQUE INDEX idx_leave_types_name_tenant ON leave_types(name, tenant_id);
CREATE INDEX idx_leave_types_tenant_id ON leave_types(tenant_id);

-- Leave Requests
CREATE TABLE IF NOT EXISTS leave_requests (
    id BIGSERIAL PRIMARY KEY,
    employee_id BIGINT NOT NULL REFERENCES employees(id),
    leave_type_id BIGINT NOT NULL REFERENCES leave_types(id),
    status VARCHAR(20) NOT NULL DEFAULT 'Pending',  -- Pending, Approved, Rejected, Cancelled
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NOT NULL,
    total_days NUMERIC(18,4) NOT NULL DEFAULT 0,
    reason TEXT,
    approved_by BIGINT REFERENCES users(id),
    approved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_leave_requests_employee_id ON leave_requests(employee_id);
CREATE INDEX idx_leave_requests_status ON leave_requests(status);

-- Payrolls
CREATE TABLE IF NOT EXISTS payrolls (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    employee_id BIGINT NOT NULL REFERENCES employees(id),
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    basic_salary NUMERIC(18,4) NOT NULL DEFAULT 0,
    overtime_hours NUMERIC(18,4) NOT NULL DEFAULT 0,
    overtime_pay NUMERIC(18,4) NOT NULL DEFAULT 0,
    bonuses NUMERIC(18,4) NOT NULL DEFAULT 0,
    deductions NUMERIC(18,4) NOT NULL DEFAULT 0,
    net_salary NUMERIC(18,4) NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'Draft',  -- Draft, Calculated, Approved, Paid
    paid_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_payrolls_tenant_id ON payrolls(tenant_id);
CREATE INDEX idx_payrolls_employee_id ON payrolls(employee_id);
CREATE INDEX idx_payrolls_period ON payrolls(period_start, period_end);

-- ============================================================================
-- ACCOUNTING DOMAIN
-- ============================================================================

CREATE TABLE IF NOT EXISTS accounts (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    code VARCHAR(20) NOT NULL,
    name VARCHAR(200) NOT NULL,
    account_type VARCHAR(20) NOT NULL,  -- Asset, Liability, Equity, Revenue, Expense
    sub_type VARCHAR(30) NOT NULL,  -- CurrentAsset, FixedAsset, CurrentLiability, etc.
    parent_id BIGINT REFERENCES accounts(id),
    is_active BOOLEAN NOT NULL DEFAULT true,
    allow_transaction BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_accounts_code_tenant ON accounts(code, tenant_id);
CREATE INDEX idx_accounts_tenant_id ON accounts(tenant_id);
CREATE INDEX idx_accounts_type ON accounts(account_type);
CREATE INDEX idx_accounts_parent_id ON accounts(parent_id);

-- Seed default chart of accounts (matches in-memory seed)
INSERT INTO accounts (id, tenant_id, code, name, account_type, sub_type, is_active, allow_transaction) VALUES
    (1, 1, '1000', 'Cash', 'Asset', 'CurrentAsset', true, true),
    (2, 1, '1100', 'Accounts Receivable', 'Asset', 'CurrentAsset', true, true),
    (3, 1, '1200', 'Inventory', 'Asset', 'CurrentAsset', true, true),
    (4, 1, '1500', 'Fixed Assets', 'Asset', 'FixedAsset', true, true),
    (5, 1, '2000', 'Accounts Payable', 'Liability', 'CurrentLiability', true, true),
    (6, 1, '2100', 'Accrued Liabilities', 'Liability', 'CurrentLiability', true, true),
    (7, 1, '2500', 'Long-term Debt', 'Liability', 'LongTermLiability', true, true),
    (8, 1, '3000', 'Owner Equity', 'Equity', 'OwnersEquity', true, true),
    (9, 1, '3100', 'Retained Earnings', 'Equity', 'RetainedEarnings', true, true),
    (10, 1, '4000', 'Sales Revenue', 'Revenue', 'OperatingRevenue', true, true),
    (11, 1, '4100', 'Other Revenue', 'Revenue', 'NonOperatingRevenue', true, true),
    (12, 1, '5000', 'Cost of Goods Sold', 'Expense', 'OperatingExpense', true, true)
ON CONFLICT (id) DO NOTHING;

-- Journal Entries
CREATE TABLE IF NOT EXISTS journal_entries (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    entry_number VARCHAR(50) NOT NULL,
    date TIMESTAMPTZ NOT NULL,
    description VARCHAR(500) NOT NULL,
    reference VARCHAR(100),
    status VARCHAR(20) NOT NULL DEFAULT 'Draft',  -- Draft, Posted, Voided
    total_debit NUMERIC(18,4) NOT NULL DEFAULT 0,
    total_credit NUMERIC(18,4) NOT NULL DEFAULT 0,
    created_by BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    posted_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_journal_entries_number_tenant ON journal_entries(entry_number, tenant_id);
CREATE INDEX idx_journal_entries_tenant_id ON journal_entries(tenant_id);
CREATE INDEX idx_journal_entries_date ON journal_entries(date);
CREATE INDEX idx_journal_entries_status ON journal_entries(status);

-- Journal Lines
CREATE TABLE IF NOT EXISTS journal_lines (
    id BIGSERIAL PRIMARY KEY,
    entry_id BIGINT NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    account_id BIGINT NOT NULL REFERENCES accounts(id),
    debit NUMERIC(18,4) NOT NULL DEFAULT 0,
    credit NUMERIC(18,4) NOT NULL DEFAULT 0,
    description VARCHAR(500),
    reference VARCHAR(100)
);

CREATE INDEX idx_journal_lines_entry_id ON journal_lines(entry_id);
CREATE INDEX idx_journal_lines_account_id ON journal_lines(account_id);

-- ============================================================================
-- PROJECT DOMAIN
-- ============================================================================

CREATE TABLE IF NOT EXISTS projects (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    name VARCHAR(200) NOT NULL,
    description TEXT,
    cari_id BIGINT REFERENCES cari(id),
    status VARCHAR(20) NOT NULL DEFAULT 'Planning',  -- Planning, Active, OnHold, Completed, Cancelled
    start_date TIMESTAMPTZ,
    end_date TIMESTAMPTZ,
    budget NUMERIC(18,4) NOT NULL DEFAULT 0,
    actual_cost NUMERIC(18,4) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_projects_tenant_id ON projects(tenant_id);
CREATE INDEX idx_projects_status ON projects(status);
CREATE INDEX idx_projects_cari_id ON projects(cari_id);

CREATE TRIGGER update_projects_updated_at
    BEFORE UPDATE ON projects
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- WBS Items
CREATE TABLE IF NOT EXISTS wbs_items (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    parent_id BIGINT REFERENCES wbs_items(id),
    name VARCHAR(200) NOT NULL,
    code VARCHAR(50) NOT NULL,
    planned_hours NUMERIC(18,4) NOT NULL DEFAULT 0,
    actual_hours NUMERIC(18,4) NOT NULL DEFAULT 0,
    progress NUMERIC(5,2) NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX idx_wbs_items_code_project ON wbs_items(code, project_id);
CREATE INDEX idx_wbs_items_project_id ON wbs_items(project_id);

-- Project Costs
CREATE TABLE IF NOT EXISTS project_costs (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    wbs_item_id BIGINT REFERENCES wbs_items(id),
    cost_type VARCHAR(20) NOT NULL,  -- Labor, Material, Equipment, Subcontract, Other
    amount NUMERIC(18,4) NOT NULL,
    description VARCHAR(500) NOT NULL,
    incurred_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_project_costs_project_id ON project_costs(project_id);
CREATE INDEX idx_project_costs_wbs_item_id ON project_costs(wbs_item_id);

-- ============================================================================
-- MANUFACTURING DOMAIN
-- ============================================================================

CREATE TABLE IF NOT EXISTS work_orders (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    name VARCHAR(200) NOT NULL,
    product_id BIGINT NOT NULL REFERENCES products(id),
    quantity NUMERIC(18,4) NOT NULL,
    bom_id BIGINT,
    routing_id BIGINT,
    status VARCHAR(20) NOT NULL DEFAULT 'Draft',  -- Draft, Scheduled, InProgress, OnHold, Completed, Cancelled
    priority VARCHAR(10) NOT NULL DEFAULT 'Normal',  -- Low, Normal, High, Urgent
    planned_start TIMESTAMPTZ,
    planned_end TIMESTAMPTZ,
    actual_start TIMESTAMPTZ,
    actual_end TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_work_orders_tenant_id ON work_orders(tenant_id);
CREATE INDEX idx_work_orders_product_id ON work_orders(product_id);
CREATE INDEX idx_work_orders_status ON work_orders(status);

CREATE TRIGGER update_work_orders_updated_at
    BEFORE UPDATE ON work_orders
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Work Order Operations
CREATE TABLE IF NOT EXISTS work_order_operations (
    id BIGSERIAL PRIMARY KEY,
    work_order_id BIGINT NOT NULL REFERENCES work_orders(id) ON DELETE CASCADE,
    operation_sequence INTEGER NOT NULL,
    operation_name VARCHAR(200) NOT NULL,
    work_center_id BIGINT,
    planned_hours NUMERIC(18,4) NOT NULL DEFAULT 0,
    actual_hours NUMERIC(18,4) NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'Pending',
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_work_order_operations_work_order_id ON work_order_operations(work_order_id);

-- Work Order Materials
CREATE TABLE IF NOT EXISTS work_order_materials (
    id BIGSERIAL PRIMARY KEY,
    work_order_id BIGINT NOT NULL REFERENCES work_orders(id) ON DELETE CASCADE,
    product_id BIGINT NOT NULL REFERENCES products(id),
    quantity_required NUMERIC(18,4) NOT NULL,
    quantity_issued NUMERIC(18,4) NOT NULL DEFAULT 0,
    is_issued BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX idx_work_order_materials_work_order_id ON work_order_materials(work_order_id);

-- Bills of Materials
CREATE TABLE IF NOT EXISTS bills_of_materials (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    product_id BIGINT NOT NULL REFERENCES products(id),
    version VARCHAR(20) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    is_primary BOOLEAN NOT NULL DEFAULT false,
    valid_from TIMESTAMPTZ,
    valid_to TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_bom_product_version_tenant ON bills_of_materials(product_id, version, tenant_id);
CREATE INDEX idx_bom_tenant_id ON bills_of_materials(tenant_id);

CREATE TRIGGER update_bills_of_materials_updated_at
    BEFORE UPDATE ON bills_of_materials
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- BOM Lines
CREATE TABLE IF NOT EXISTS bom_lines (
    id BIGSERIAL PRIMARY KEY,
    bom_id BIGINT NOT NULL REFERENCES bills_of_materials(id) ON DELETE CASCADE,
    component_product_id BIGINT NOT NULL REFERENCES products(id),
    quantity NUMERIC(18,4) NOT NULL,
    unit_id BIGINT REFERENCES units(id),
    scrap_percentage NUMERIC(5,2) NOT NULL DEFAULT 0,
    is_optional BOOLEAN NOT NULL DEFAULT false,
    notes TEXT
);

CREATE INDEX idx_bom_lines_bom_id ON bom_lines(bom_id);

-- Routings
CREATE TABLE IF NOT EXISTS routings (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    product_id BIGINT NOT NULL REFERENCES products(id),
    version VARCHAR(20) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    is_primary BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_routings_product_version_tenant ON routings(product_id, version, tenant_id);
CREATE INDEX idx_routings_tenant_id ON routings(tenant_id);

CREATE TRIGGER update_routings_updated_at
    BEFORE UPDATE ON routings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Routing Operations
CREATE TABLE IF NOT EXISTS routing_operations (
    id BIGSERIAL PRIMARY KEY,
    routing_id BIGINT NOT NULL REFERENCES routings(id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL,
    operation_name VARCHAR(200) NOT NULL,
    work_center_id BIGINT,
    setup_hours NUMERIC(18,4) NOT NULL DEFAULT 0,
    run_hours NUMERIC(18,4) NOT NULL DEFAULT 0,
    description TEXT
);

CREATE INDEX idx_routing_operations_routing_id ON routing_operations(routing_id);

-- Inspections
CREATE TABLE IF NOT EXISTS inspections (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    work_order_id BIGINT REFERENCES work_orders(id),
    product_id BIGINT NOT NULL REFERENCES products(id),
    inspection_type VARCHAR(50) NOT NULL,
    quantity_inspected NUMERIC(18,4) NOT NULL DEFAULT 0,
    quantity_passed NUMERIC(18,4) NOT NULL DEFAULT 0,
    quantity_failed NUMERIC(18,4) NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'Pending',  -- Pending, InProgress, Passed, Failed, Rework
    inspector_id BIGINT REFERENCES users(id),
    inspected_at TIMESTAMPTZ,
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_inspections_tenant_id ON inspections(tenant_id);
CREATE INDEX idx_inspections_work_order_id ON inspections(work_order_id);

-- Non-Conformance Reports
CREATE TABLE IF NOT EXISTS non_conformance_reports (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    inspection_id BIGINT REFERENCES inspections(id),
    product_id BIGINT NOT NULL REFERENCES products(id),
    ncr_type VARCHAR(10) NOT NULL,  -- Minor, Major, Critical
    description TEXT NOT NULL,
    root_cause TEXT,
    corrective_action TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'Open',  -- Open, UnderReview, CorrectiveAction, Closed, Rejected
    raised_by BIGINT NOT NULL REFERENCES users(id),
    raised_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at TIMESTAMPTZ
);

CREATE INDEX idx_ncr_tenant_id ON non_conformance_reports(tenant_id);
CREATE INDEX idx_ncr_inspection_id ON non_conformance_reports(inspection_id);
CREATE INDEX idx_ncr_status ON non_conformance_reports(status);

-- ============================================================================
-- CRM DOMAIN
-- ============================================================================

CREATE TABLE IF NOT EXISTS leads (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    name VARCHAR(200) NOT NULL,
    company VARCHAR(200),
    email VARCHAR(255),
    phone VARCHAR(20),
    source VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'New',  -- New, Contacted, Qualified, Unqualified, Converted
    assigned_to BIGINT REFERENCES users(id),
    converted_to_customer_id BIGINT REFERENCES cari(id),
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_leads_tenant_id ON leads(tenant_id);
CREATE INDEX idx_leads_status ON leads(status);
CREATE INDEX idx_leads_assigned_to ON leads(assigned_to);

CREATE TRIGGER update_leads_updated_at
    BEFORE UPDATE ON leads
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Opportunities
CREATE TABLE IF NOT EXISTS opportunities (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    lead_id BIGINT REFERENCES leads(id),
    name VARCHAR(200) NOT NULL,
    customer_id BIGINT REFERENCES cari(id),
    value NUMERIC(18,4) NOT NULL DEFAULT 0,
    probability NUMERIC(5,2) NOT NULL DEFAULT 0,
    expected_close_date TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'Open',  -- Open, Won, Lost, Cancelled
    assigned_to BIGINT REFERENCES users(id),
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_opportunities_tenant_id ON opportunities(tenant_id);
CREATE INDEX idx_opportunities_status ON opportunities(status);
CREATE INDEX idx_opportunities_lead_id ON opportunities(lead_id);

CREATE TRIGGER update_opportunities_updated_at
    BEFORE UPDATE ON opportunities
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Campaigns
CREATE TABLE IF NOT EXISTS campaigns (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    name VARCHAR(200) NOT NULL,
    description TEXT,
    campaign_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'Draft',  -- Draft, Scheduled, Active, Completed, Cancelled
    budget NUMERIC(18,4) NOT NULL DEFAULT 0,
    actual_cost NUMERIC(18,4) NOT NULL DEFAULT 0,
    start_date TIMESTAMPTZ,
    end_date TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_campaigns_tenant_id ON campaigns(tenant_id);
CREATE INDEX idx_campaigns_status ON campaigns(status);

CREATE TRIGGER update_campaigns_updated_at
    BEFORE UPDATE ON campaigns
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Support Tickets
CREATE TABLE IF NOT EXISTS tickets (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    ticket_number VARCHAR(50) NOT NULL,
    subject VARCHAR(500) NOT NULL,
    description TEXT NOT NULL,
    customer_id BIGINT REFERENCES cari(id),
    assigned_to BIGINT REFERENCES users(id),
    status VARCHAR(20) NOT NULL DEFAULT 'Open',  -- Open, InProgress, Pending, Resolved, Closed
    priority VARCHAR(10) NOT NULL DEFAULT 'Medium',  -- Low, Medium, High, Critical
    category VARCHAR(50),
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_tickets_number_tenant ON tickets(ticket_number, tenant_id);
CREATE INDEX idx_tickets_tenant_id ON tickets(tenant_id);
CREATE INDEX idx_tickets_status ON tickets(status);
CREATE INDEX idx_tickets_assigned_to ON tickets(assigned_to);

CREATE TRIGGER update_tickets_updated_at
    BEFORE UPDATE ON tickets
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- ASSETS DOMAIN
-- ============================================================================

CREATE TABLE IF NOT EXISTS asset_categories (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    name VARCHAR(200) NOT NULL,
    description TEXT,
    default_useful_life_years INTEGER NOT NULL DEFAULT 5,
    default_depreciation_method VARCHAR(30) NOT NULL DEFAULT 'StraightLine',  -- StraightLine, DecliningBalance, UnitsOfProduction, None
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_asset_categories_name_tenant ON asset_categories(name, tenant_id);
CREATE INDEX idx_asset_categories_tenant_id ON asset_categories(tenant_id);

-- Assets
CREATE TABLE IF NOT EXISTS assets (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    asset_code VARCHAR(50) NOT NULL,
    name VARCHAR(200) NOT NULL,
    category_id BIGINT REFERENCES asset_categories(id),
    description TEXT,
    serial_number VARCHAR(100),
    location VARCHAR(200),
    status VARCHAR(20) NOT NULL DEFAULT 'Active',  -- Active, InUse, UnderMaintenance, Disposed, WrittenOff
    acquisition_date TIMESTAMPTZ NOT NULL,
    acquisition_cost NUMERIC(18,4) NOT NULL DEFAULT 0,
    salvage_value NUMERIC(18,4) NOT NULL DEFAULT 0,
    useful_life_years INTEGER NOT NULL DEFAULT 5,
    depreciation_method VARCHAR(30) NOT NULL DEFAULT 'StraightLine',  -- StraightLine, DecliningBalance, UnitsOfProduction, None
    accumulated_depreciation NUMERIC(18,4) NOT NULL DEFAULT 0,
    book_value NUMERIC(18,4) NOT NULL DEFAULT 0,
    warranty_expiry TIMESTAMPTZ,
    insurance_number VARCHAR(50),
    insurance_expiry TIMESTAMPTZ,
    responsible_person_id BIGINT REFERENCES users(id),
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_assets_code_tenant ON assets(asset_code, tenant_id);
CREATE INDEX idx_assets_tenant_id ON assets(tenant_id);
CREATE INDEX idx_assets_category_id ON assets(category_id);
CREATE INDEX idx_assets_status ON assets(status);

CREATE TRIGGER update_assets_updated_at
    BEFORE UPDATE ON assets
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Maintenance Records
CREATE TABLE IF NOT EXISTS maintenance_records (
    id BIGSERIAL PRIMARY KEY,
    asset_id BIGINT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    maintenance_date TIMESTAMPTZ NOT NULL,
    maintenance_type VARCHAR(50) NOT NULL,
    description TEXT NOT NULL,
    cost NUMERIC(18,4) NOT NULL DEFAULT 0,
    performed_by VARCHAR(200),
    next_maintenance_date TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_maintenance_records_asset_id ON maintenance_records(asset_id);

-- ============================================================================
-- FEATURE FLAGS
-- ============================================================================

CREATE TABLE IF NOT EXISTS feature_flags (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'Disabled',  -- Enabled, Disabled
    tenant_id BIGINT REFERENCES tenants(id),  -- NULL means global flag
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP
);

CREATE UNIQUE INDEX idx_feature_flags_name_tenant ON feature_flags(name, tenant_id);
CREATE INDEX idx_feature_flags_tenant_id ON feature_flags(tenant_id);

CREATE TRIGGER update_feature_flags_updated_at
    BEFORE UPDATE ON feature_flags
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- TENANT CONFIG
-- ============================================================================

CREATE TABLE IF NOT EXISTS tenant_configs (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id),
    key VARCHAR(100) NOT NULL,
    value JSONB NOT NULL,
    is_encrypted BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE UNIQUE INDEX idx_tenant_configs_key_tenant ON tenant_configs(key, tenant_id);
CREATE INDEX idx_tenant_configs_tenant_id ON tenant_configs(tenant_id);

CREATE TRIGGER update_tenant_configs_updated_at
    BEFORE UPDATE ON tenant_configs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();