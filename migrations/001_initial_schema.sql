-- RampOS Database Schema
-- PostgreSQL 15+

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================================
-- TENANTS (Exchanges using RampOS)
-- ============================================================================

CREATE TABLE tenants (
    id VARCHAR(64) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'ACTIVE', -- ACTIVE, SUSPENDED, PENDING

    -- API credentials
    api_key_hash VARCHAR(255) NOT NULL,
    webhook_secret_hash VARCHAR(255) NOT NULL,
    webhook_url VARCHAR(512),

    -- Configuration
    config JSONB NOT NULL DEFAULT '{}',

    -- Limits
    daily_payin_limit_vnd DECIMAL(20, 2) DEFAULT 10000000000, -- 10B VND
    daily_payout_limit_vnd DECIMAL(20, 2) DEFAULT 5000000000,  -- 5B VND

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT tenants_status_check CHECK (status IN ('ACTIVE', 'SUSPENDED', 'PENDING'))
);

CREATE INDEX idx_tenants_status ON tenants(status);

-- ============================================================================
-- USERS (End users on tenant platforms)
-- ============================================================================

CREATE TABLE users (
    id VARCHAR(64) NOT NULL,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),

    -- KYC
    kyc_tier SMALLINT NOT NULL DEFAULT 0, -- 0: None, 1: Basic, 2: Enhanced, 3: Business
    kyc_status VARCHAR(32) NOT NULL DEFAULT 'PENDING',
    kyc_verified_at TIMESTAMPTZ,

    -- Risk
    risk_score DECIMAL(5, 2) DEFAULT 0,
    risk_flags JSONB DEFAULT '[]',

    -- Limits (per-user overrides)
    daily_payin_limit_vnd DECIMAL(20, 2),
    daily_payout_limit_vnd DECIMAL(20, 2),

    -- Status
    status VARCHAR(32) NOT NULL DEFAULT 'ACTIVE',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (tenant_id, id),
    CONSTRAINT users_kyc_tier_check CHECK (kyc_tier BETWEEN 0 AND 3),
    CONSTRAINT users_status_check CHECK (status IN ('ACTIVE', 'SUSPENDED', 'BLOCKED'))
);

CREATE INDEX idx_users_tenant ON users(tenant_id);
CREATE INDEX idx_users_kyc_status ON users(tenant_id, kyc_status);
CREATE INDEX idx_users_risk_score ON users(tenant_id, risk_score DESC);

-- ============================================================================
-- INTENTS (Core transaction entities)
-- ============================================================================

CREATE TABLE intents (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    user_id VARCHAR(64) NOT NULL,

    -- Type and state
    intent_type VARCHAR(32) NOT NULL,
    state VARCHAR(64) NOT NULL,
    state_history JSONB NOT NULL DEFAULT '[]',

    -- Amount
    amount DECIMAL(30, 8) NOT NULL,
    currency VARCHAR(16) NOT NULL,
    actual_amount DECIMAL(30, 8),

    -- Rails
    rails_provider VARCHAR(64),
    reference_code VARCHAR(64),
    bank_tx_id VARCHAR(128),

    -- On-chain (for crypto intents)
    chain_id VARCHAR(32),
    tx_hash VARCHAR(128),
    from_address VARCHAR(128),
    to_address VARCHAR(128),

    -- Metadata
    metadata JSONB NOT NULL DEFAULT '{}',

    -- Idempotency
    idempotency_key VARCHAR(128),

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,

    CONSTRAINT intents_type_check CHECK (intent_type IN (
        'PAYIN_VND', 'PAYOUT_VND', 'TRADE_EXECUTED',
        'DEPOSIT_ONCHAIN', 'WITHDRAW_ONCHAIN'
    ))
);

CREATE UNIQUE INDEX idx_intents_idempotency ON intents(tenant_id, idempotency_key)
    WHERE idempotency_key IS NOT NULL;
CREATE INDEX idx_intents_tenant_user ON intents(tenant_id, user_id);
CREATE INDEX idx_intents_state ON intents(tenant_id, state);
CREATE INDEX idx_intents_type ON intents(tenant_id, intent_type);
CREATE INDEX idx_intents_reference ON intents(tenant_id, reference_code)
    WHERE reference_code IS NOT NULL;
CREATE INDEX idx_intents_created ON intents(tenant_id, created_at DESC);
CREATE INDEX idx_intents_expires ON intents(expires_at)
    WHERE expires_at IS NOT NULL AND state NOT IN ('COMPLETED', 'EXPIRED', 'CANCELLED');

-- ============================================================================
-- LEDGER ENTRIES (Double-entry accounting)
-- ============================================================================

CREATE TABLE ledger_entries (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    user_id VARCHAR(64),
    intent_id VARCHAR(64) NOT NULL REFERENCES intents(id),
    transaction_id VARCHAR(64) NOT NULL,

    -- Account info
    account_type VARCHAR(64) NOT NULL,
    direction VARCHAR(8) NOT NULL, -- DEBIT or CREDIT

    -- Amount
    amount DECIMAL(30, 8) NOT NULL,
    currency VARCHAR(16) NOT NULL,
    balance_after DECIMAL(30, 8) NOT NULL,

    -- Sequence for ordering
    sequence BIGSERIAL,

    -- Metadata
    description TEXT,
    metadata JSONB DEFAULT '{}',

    -- Timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT ledger_direction_check CHECK (direction IN ('DEBIT', 'CREDIT')),
    CONSTRAINT ledger_amount_positive CHECK (amount >= 0)
);

CREATE INDEX idx_ledger_tenant ON ledger_entries(tenant_id);
CREATE INDEX idx_ledger_intent ON ledger_entries(intent_id);
CREATE INDEX idx_ledger_transaction ON ledger_entries(transaction_id);
CREATE INDEX idx_ledger_account ON ledger_entries(tenant_id, account_type, currency);
CREATE INDEX idx_ledger_user ON ledger_entries(tenant_id, user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_ledger_sequence ON ledger_entries(sequence);

-- ============================================================================
-- ACCOUNT BALANCES (Materialized view for fast balance queries)
-- ============================================================================

CREATE TABLE account_balances (
    id SERIAL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    user_id VARCHAR(64), -- NULL for system accounts
    account_type VARCHAR(64) NOT NULL,
    currency VARCHAR(16) NOT NULL,

    balance DECIMAL(30, 8) NOT NULL DEFAULT 0,
    last_entry_id VARCHAR(64),
    last_sequence BIGINT,

    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Unique constraint for composite key
    CONSTRAINT account_balances_unique UNIQUE (tenant_id, user_id, account_type, currency)
);

CREATE INDEX idx_balances_user ON account_balances(tenant_id, user_id)
    WHERE user_id IS NOT NULL;

-- ============================================================================
-- WEBHOOKS (Outbox pattern for reliable delivery)
-- ============================================================================

CREATE TABLE webhook_events (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),

    -- Event details
    event_type VARCHAR(64) NOT NULL,
    intent_id VARCHAR(64) REFERENCES intents(id),
    payload JSONB NOT NULL,

    -- Delivery status
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',
    attempts INT NOT NULL DEFAULT 0,
    max_attempts INT NOT NULL DEFAULT 10,
    last_attempt_at TIMESTAMPTZ,
    next_attempt_at TIMESTAMPTZ,
    last_error TEXT,

    -- Delivery confirmation
    delivered_at TIMESTAMPTZ,
    response_status INT,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT webhook_status_check CHECK (status IN ('PENDING', 'DELIVERED', 'FAILED', 'CANCELLED'))
);

CREATE INDEX idx_webhooks_pending ON webhook_events(next_attempt_at)
    WHERE status = 'PENDING';
CREATE INDEX idx_webhooks_tenant ON webhook_events(tenant_id, created_at DESC);
CREATE INDEX idx_webhooks_intent ON webhook_events(intent_id);

-- ============================================================================
-- RAILS ADAPTERS (Bank/PSP configurations per tenant)
-- ============================================================================

CREATE TABLE rails_adapters (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),

    -- Provider info
    provider_code VARCHAR(64) NOT NULL,
    provider_name VARCHAR(255) NOT NULL,
    adapter_type VARCHAR(32) NOT NULL, -- BANK, PSP, CRYPTO

    -- Configuration (encrypted)
    config_encrypted BYTEA NOT NULL,

    -- Capabilities
    supports_payin BOOLEAN NOT NULL DEFAULT true,
    supports_payout BOOLEAN NOT NULL DEFAULT true,
    supports_virtual_account BOOLEAN NOT NULL DEFAULT false,

    -- Status
    status VARCHAR(32) NOT NULL DEFAULT 'ACTIVE',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (tenant_id, provider_code),
    CONSTRAINT rails_status_check CHECK (status IN ('ACTIVE', 'DISABLED', 'TESTING'))
);

-- ============================================================================
-- VIRTUAL ACCOUNTS (For pay-in tracking)
-- ============================================================================

CREATE TABLE virtual_accounts (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    user_id VARCHAR(64) NOT NULL,
    rails_adapter_id VARCHAR(64) NOT NULL REFERENCES rails_adapters(id),

    -- Account details
    bank_code VARCHAR(32) NOT NULL,
    account_number VARCHAR(64) NOT NULL,
    account_name VARCHAR(255) NOT NULL,

    -- Status
    status VARCHAR(32) NOT NULL DEFAULT 'ACTIVE',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,

    UNIQUE (bank_code, account_number)
);

CREATE INDEX idx_va_tenant_user ON virtual_accounts(tenant_id, user_id);
CREATE INDEX idx_va_account ON virtual_accounts(bank_code, account_number);

-- ============================================================================
-- COMPLIANCE: KYC Records
-- ============================================================================

CREATE TABLE kyc_records (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    user_id VARCHAR(64) NOT NULL,

    -- KYC data
    tier SMALLINT NOT NULL,
    provider VARCHAR(64),
    provider_reference VARCHAR(128),

    -- Verification result
    status VARCHAR(32) NOT NULL, -- PENDING, APPROVED, REJECTED, EXPIRED
    verification_data JSONB NOT NULL DEFAULT '{}',
    rejection_reason TEXT,

    -- Documents (references to secure storage)
    documents JSONB DEFAULT '[]',

    -- Timestamps
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    verified_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,

    FOREIGN KEY (tenant_id, user_id) REFERENCES users(tenant_id, id)
);

CREATE INDEX idx_kyc_user ON kyc_records(tenant_id, user_id);
CREATE INDEX idx_kyc_status ON kyc_records(tenant_id, status);

-- ============================================================================
-- COMPLIANCE: AML Cases
-- ============================================================================

CREATE TABLE aml_cases (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    user_id VARCHAR(64),
    intent_id VARCHAR(64) REFERENCES intents(id),

    -- Case details
    case_type VARCHAR(64) NOT NULL, -- VELOCITY, STRUCTURING, NAME_MISMATCH, SANCTIONS, PEP, etc.
    severity VARCHAR(16) NOT NULL, -- LOW, MEDIUM, HIGH, CRITICAL
    status VARCHAR(32) NOT NULL DEFAULT 'OPEN', -- OPEN, REVIEW, HOLD, RELEASED, REPORTED

    -- Detection
    rule_id VARCHAR(64),
    rule_name VARCHAR(255),
    detection_data JSONB NOT NULL,

    -- Resolution
    assigned_to VARCHAR(64),
    resolution TEXT,
    resolved_at TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_aml_tenant ON aml_cases(tenant_id);
CREATE INDEX idx_aml_user ON aml_cases(tenant_id, user_id) WHERE user_id IS NOT NULL;
CREATE INDEX idx_aml_status ON aml_cases(tenant_id, status);
CREATE INDEX idx_aml_severity ON aml_cases(tenant_id, severity, status);

-- ============================================================================
-- AUDIT LOG (Append-only)
-- ============================================================================

CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,

    -- Actor
    actor_type VARCHAR(32) NOT NULL, -- SYSTEM, USER, ADMIN, API
    actor_id VARCHAR(64),

    -- Action
    action VARCHAR(64) NOT NULL,
    resource_type VARCHAR(64) NOT NULL,
    resource_id VARCHAR(64),

    -- Details
    details JSONB NOT NULL DEFAULT '{}',

    -- Request context
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(64),

    -- Timestamp
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Hash chain for integrity
    prev_hash VARCHAR(64),
    entry_hash VARCHAR(64) NOT NULL
);

CREATE INDEX idx_audit_tenant ON audit_log(tenant_id, created_at DESC);
CREATE INDEX idx_audit_resource ON audit_log(tenant_id, resource_type, resource_id);
CREATE INDEX idx_audit_actor ON audit_log(tenant_id, actor_type, actor_id);

-- ============================================================================
-- RECONCILIATION BATCHES
-- ============================================================================

CREATE TABLE recon_batches (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    rails_adapter_id VARCHAR(64) REFERENCES rails_adapters(id),

    -- Batch period
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,

    -- Status
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',

    -- Summary
    total_intents INT NOT NULL DEFAULT 0,
    matched_intents INT NOT NULL DEFAULT 0,
    unmatched_intents INT NOT NULL DEFAULT 0,
    discrepancy_amount DECIMAL(30, 8) DEFAULT 0,

    -- Files
    our_file_hash VARCHAR(128),
    provider_file_hash VARCHAR(128),
    report_url TEXT,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_recon_tenant ON recon_batches(tenant_id, created_at DESC);
CREATE INDEX idx_recon_status ON recon_batches(status) WHERE status = 'PENDING';

-- ============================================================================
-- FUNCTIONS & TRIGGERS
-- ============================================================================

-- Auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_tenants_updated_at
    BEFORE UPDATE ON tenants
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_intents_updated_at
    BEFORE UPDATE ON intents
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_aml_cases_updated_at
    BEFORE UPDATE ON aml_cases
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Function to append state to history
CREATE OR REPLACE FUNCTION append_state_history()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.state IS DISTINCT FROM NEW.state THEN
        NEW.state_history = OLD.state_history || jsonb_build_object(
            'from', OLD.state,
            'to', NEW.state,
            'at', NOW()
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_intent_state_history
    BEFORE UPDATE ON intents
    FOR EACH ROW EXECUTE FUNCTION append_state_history();
