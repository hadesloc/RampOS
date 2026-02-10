-- Migration 027: Off-ramp intents table
-- Replaces in-memory storage in OffRampService with persistent SQL storage

CREATE TABLE IF NOT EXISTS offramp_intents (
    id TEXT PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    user_id TEXT NOT NULL,
    crypto_asset TEXT NOT NULL,
    crypto_amount NUMERIC NOT NULL,
    exchange_rate NUMERIC NOT NULL,
    locked_rate_id TEXT,
    fees JSONB NOT NULL DEFAULT '{}',
    net_vnd_amount NUMERIC NOT NULL,
    gross_vnd_amount NUMERIC NOT NULL,
    bank_account JSONB NOT NULL DEFAULT '{}',
    deposit_address TEXT,
    tx_hash TEXT,
    bank_reference TEXT,
    state TEXT NOT NULL DEFAULT 'QUOTE_CREATED',
    state_history JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    quote_expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_offramp_intents_tenant_created ON offramp_intents(tenant_id, created_at DESC);
CREATE INDEX idx_offramp_intents_status ON offramp_intents(state);
CREATE INDEX idx_offramp_intents_user ON offramp_intents(tenant_id, user_id);

-- Enable RLS
ALTER TABLE offramp_intents ENABLE ROW LEVEL SECURITY;

CREATE POLICY offramp_intents_tenant_isolation ON offramp_intents
    USING (tenant_id::text = current_setting('app.current_tenant', true));
