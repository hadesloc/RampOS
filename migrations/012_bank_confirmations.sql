-- ============================================================================
-- Bank Confirmations Table
-- Stores incoming bank webhook confirmations for pay-in matching
-- ============================================================================

-- Create bank_confirmations table
CREATE TABLE bank_confirmations (
    id VARCHAR(64) PRIMARY KEY DEFAULT 'BC_' || gen_random_uuid()::TEXT,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),

    -- Bank provider info
    provider VARCHAR(64) NOT NULL,  -- VietQR, Napas, VCB, etc.

    -- Transaction identification
    reference_code VARCHAR(128) NOT NULL,  -- Our reference code to match with intent
    bank_reference VARCHAR(128),           -- Bank's transaction reference
    bank_tx_id VARCHAR(128),               -- Bank's internal transaction ID

    -- Amount
    amount DECIMAL(30, 8) NOT NULL,
    currency VARCHAR(16) NOT NULL DEFAULT 'VND',

    -- Bank account info
    sender_account VARCHAR(64),
    sender_name VARCHAR(255),
    receiver_account VARCHAR(64),
    receiver_name VARCHAR(255),

    -- Matching status
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',  -- PENDING, MATCHED, UNMATCHED, DUPLICATE, REJECTED
    matched_intent_id VARCHAR(64) REFERENCES intents(id),
    matched_at TIMESTAMPTZ,

    -- Webhook details
    webhook_received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    webhook_signature VARCHAR(512),
    webhook_signature_verified BOOLEAN NOT NULL DEFAULT FALSE,

    -- Raw data for audit
    raw_payload JSONB NOT NULL,

    -- Processing
    processing_notes TEXT,

    -- Timestamps
    transaction_time TIMESTAMPTZ,  -- When bank processed the transaction
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT bank_confirmations_status_check CHECK (
        status IN ('PENDING', 'MATCHED', 'UNMATCHED', 'DUPLICATE', 'REJECTED')
    )
);

-- Indexes for efficient querying
CREATE INDEX idx_bank_confirmations_tenant ON bank_confirmations(tenant_id);
CREATE INDEX idx_bank_confirmations_reference ON bank_confirmations(tenant_id, reference_code);
CREATE INDEX idx_bank_confirmations_bank_tx ON bank_confirmations(bank_tx_id) WHERE bank_tx_id IS NOT NULL;
CREATE INDEX idx_bank_confirmations_status ON bank_confirmations(status) WHERE status = 'PENDING';
CREATE INDEX idx_bank_confirmations_provider ON bank_confirmations(provider, created_at DESC);
CREATE INDEX idx_bank_confirmations_intent ON bank_confirmations(matched_intent_id) WHERE matched_intent_id IS NOT NULL;

-- Unique constraint to prevent duplicate confirmations
CREATE UNIQUE INDEX idx_bank_confirmations_unique ON bank_confirmations(provider, bank_tx_id)
    WHERE bank_tx_id IS NOT NULL;

-- Auto-update updated_at timestamp
CREATE TRIGGER trigger_bank_confirmations_updated_at
    BEFORE UPDATE ON bank_confirmations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- Bank Webhook Secrets Table
-- Stores webhook secrets for different bank providers
-- ============================================================================

CREATE TABLE bank_webhook_secrets (
    id SERIAL PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    provider VARCHAR(64) NOT NULL,

    -- Secret storage (encrypted)
    secret_encrypted BYTEA NOT NULL,

    -- Configuration
    algorithm VARCHAR(32) NOT NULL DEFAULT 'HMAC-SHA256',  -- HMAC-SHA256, HMAC-SHA512, RSA-SHA256
    header_name VARCHAR(64) NOT NULL DEFAULT 'X-Signature',

    -- Status
    is_active BOOLEAN NOT NULL DEFAULT TRUE,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (tenant_id, provider)
);

CREATE INDEX idx_bank_webhook_secrets_lookup ON bank_webhook_secrets(tenant_id, provider) WHERE is_active = TRUE;

-- Auto-update updated_at timestamp
CREATE TRIGGER trigger_bank_webhook_secrets_updated_at
    BEFORE UPDATE ON bank_webhook_secrets
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================================================
-- RLS Policies for bank_confirmations
-- ============================================================================

ALTER TABLE bank_confirmations ENABLE ROW LEVEL SECURITY;
ALTER TABLE bank_webhook_secrets ENABLE ROW LEVEL SECURITY;

-- Policy for bank_confirmations
CREATE POLICY bank_confirmations_tenant_isolation ON bank_confirmations
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));

-- Policy for bank_webhook_secrets
CREATE POLICY bank_webhook_secrets_tenant_isolation ON bank_webhook_secrets
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));

-- Comments for documentation
COMMENT ON TABLE bank_confirmations IS 'Stores incoming bank webhook confirmations for pay-in matching';
COMMENT ON COLUMN bank_confirmations.reference_code IS 'Our reference code used to match with pending intents';
COMMENT ON COLUMN bank_confirmations.bank_tx_id IS 'Bank internal transaction ID for duplicate detection';
COMMENT ON COLUMN bank_confirmations.raw_payload IS 'Original webhook payload for audit trail';
COMMENT ON TABLE bank_webhook_secrets IS 'Stores webhook secrets for signature verification per provider';
