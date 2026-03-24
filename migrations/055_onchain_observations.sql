-- Migration 055: On-chain observations
-- Adds a tenant-scoped authoritative observation store for on-chain transfer status.

CREATE TABLE IF NOT EXISTS onchain_observations (
    id VARCHAR(64) PRIMARY KEY DEFAULT 'OCO_' || gen_random_uuid()::TEXT,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    intent_id VARCHAR(64) REFERENCES intents(id) ON DELETE SET NULL,
    offramp_intent_id TEXT REFERENCES offramp_intents(id) ON DELETE SET NULL,

    -- Observation identity
    tx_hash VARCHAR(128) NOT NULL,
    chain_id BIGINT NOT NULL,

    -- Economic facts
    asset_code VARCHAR(32) NOT NULL,
    amount DECIMAL(30, 8) NOT NULL,
    from_address VARCHAR(128) NOT NULL,
    to_address VARCHAR(128) NOT NULL,

    -- Confirmation and lifecycle truth
    status VARCHAR(32) NOT NULL DEFAULT 'OBSERVED',
    confirmations INTEGER NOT NULL DEFAULT 0,
    required_confirmations INTEGER NOT NULL DEFAULT 0,
    block_number BIGINT,

    -- Source lineage
    observation_source VARCHAR(64) NOT NULL,
    raw_payload JSONB,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- Timestamps
    observed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT onchain_observations_status_check CHECK (
        status IN ('OBSERVED', 'PENDING', 'CONFIRMED', 'FAILED', 'REPLACED')
    ),
    CONSTRAINT onchain_observations_link_check CHECK (
        intent_id IS NOT NULL OR offramp_intent_id IS NOT NULL
    ),
    CONSTRAINT onchain_observations_confirmations_check CHECK (
        confirmations >= 0 AND required_confirmations >= 0
    )
);

CREATE UNIQUE INDEX idx_onchain_observations_tenant_chain_hash
    ON onchain_observations(tenant_id, chain_id, tx_hash);
CREATE INDEX idx_onchain_observations_offramp
    ON onchain_observations(tenant_id, offramp_intent_id)
    WHERE offramp_intent_id IS NOT NULL;
CREATE INDEX idx_onchain_observations_intent
    ON onchain_observations(tenant_id, intent_id)
    WHERE intent_id IS NOT NULL;
CREATE INDEX idx_onchain_observations_status
    ON onchain_observations(tenant_id, status, observed_at DESC);

CREATE TRIGGER trigger_onchain_observations_updated_at
    BEFORE UPDATE ON onchain_observations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

ALTER TABLE onchain_observations ENABLE ROW LEVEL SECURITY;

CREATE POLICY onchain_observations_tenant_isolation ON onchain_observations
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));

COMMENT ON TABLE onchain_observations IS 'Tenant-scoped authoritative observation rows for on-chain transfer status and confirmations';
COMMENT ON COLUMN onchain_observations.observation_source IS 'Origin of the observation, for example deposit_monitor, withdraw_confirm, or explorer_import';
