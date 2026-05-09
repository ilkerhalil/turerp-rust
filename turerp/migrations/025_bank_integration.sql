-- Bank Integration for Turkish banks
-- Tables: bank_accounts, bank_statements, bank_transactions, reconciliation_rules

-- Bank accounts table
CREATE TABLE IF NOT EXISTS bank_accounts (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    company_id BIGINT,
    bank_code VARCHAR(50) NOT NULL,
    account_number VARCHAR(100) NOT NULL,
    iban VARCHAR(34),
    account_name VARCHAR(200) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'TRY',
    branch_code VARCHAR(50),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    deleted_by BIGINT
);

CREATE INDEX idx_bank_accounts_tenant ON bank_accounts(tenant_id);
CREATE INDEX idx_bank_accounts_tenant_deleted ON bank_accounts(tenant_id, deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX idx_bank_accounts_bank_code ON bank_accounts(bank_code);

-- Bank statements table
CREATE TABLE IF NOT EXISTS bank_statements (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    account_id BIGINT NOT NULL REFERENCES bank_accounts(id),
    statement_date DATE NOT NULL,
    format VARCHAR(20) NOT NULL,
    raw_data TEXT NOT NULL,
    processed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bank_statements_tenant ON bank_statements(tenant_id);
CREATE INDEX idx_bank_statements_account ON bank_statements(account_id);
CREATE INDEX idx_bank_statements_date ON bank_statements(statement_date);

-- Bank transactions table
CREATE TABLE IF NOT EXISTS bank_transactions (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    account_id BIGINT NOT NULL REFERENCES bank_accounts(id),
    transaction_date DATE NOT NULL,
    description TEXT NOT NULL,
    amount NUMERIC(19, 4) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'TRY',
    balance_after NUMERIC(19, 4),
    reference_no VARCHAR(200),
    matched_invoice_id BIGINT,
    matched_payment_id BIGINT,
    match_status VARCHAR(20) NOT NULL DEFAULT 'unmatched',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bank_transactions_tenant ON bank_transactions(tenant_id);
CREATE INDEX idx_bank_transactions_account ON bank_transactions(account_id);
CREATE INDEX idx_bank_transactions_date ON bank_transactions(transaction_date);
CREATE INDEX idx_bank_transactions_match_status ON bank_transactions(match_status);
CREATE INDEX idx_bank_transactions_reference ON bank_transactions(reference_no);
CREATE INDEX idx_bank_transactions_unmatched ON bank_transactions(tenant_id, match_status) WHERE match_status = 'unmatched';

-- Reconciliation rules table
CREATE TABLE IF NOT EXISTS reconciliation_rules (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    rule_name VARCHAR(200) NOT NULL,
    match_field VARCHAR(50) NOT NULL,
    match_pattern VARCHAR(500) NOT NULL,
    auto_match BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ
);

CREATE INDEX idx_reconciliation_rules_tenant ON reconciliation_rules(tenant_id);
CREATE INDEX idx_reconciliation_rules_active ON reconciliation_rules(tenant_id, is_active);
