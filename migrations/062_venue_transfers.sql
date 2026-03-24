-- Migration 062: Venue transfers
-- First-class wallet <-> venue transfer lifecycle records.

CREATE TABLE IF NOT EXISTS venue_transfers (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    beneficiary_profile_id TEXT NULL,
    wallet_attestation_id UUID NOT NULL REFERENCES wallet_attestations(id) ON DELETE RESTRICT,
    venue_connection_id TEXT NOT NULL,
    venue_account_id TEXT NOT NULL,
    transfer_direction TEXT NOT NULL DEFAULT 'wallet_to_venue',
    asset_symbol TEXT NOT NULL,
    network TEXT NOT NULL,
    amount NUMERIC(30, 8) NOT NULL,
    origin_intent_id TEXT NULL REFERENCES intents(id) ON DELETE SET NULL,
    rfq_id TEXT NULL REFERENCES rfq_requests(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    wallet_tx_hash TEXT NULL,
    venue_credit_ref TEXT NULL,
    failure_code TEXT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    submitted_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT venue_transfers_direction_check CHECK (
        transfer_direction IN ('wallet_to_venue', 'venue_to_wallet')
    ),
    CONSTRAINT venue_transfers_status_check CHECK (
        status IN ('pending', 'submitted', 'broadcast', 'confirmed', 'credited', 'failed', 'cancelled')
    ),
    CONSTRAINT venue_transfers_amount_positive CHECK (amount > 0),
    CONSTRAINT venue_transfers_metadata_object CHECK (
        jsonb_typeof(metadata) = 'object'
    ),
    CONSTRAINT venue_transfers_user_fk
        FOREIGN KEY (tenant_id, user_id) REFERENCES users(tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT venue_transfers_beneficiary_fk
        FOREIGN KEY (beneficiary_profile_id, tenant_id)
        REFERENCES beneficiary_profiles(id, tenant_id)
        ON DELETE SET NULL,
    CONSTRAINT venue_transfers_connection_fk
        FOREIGN KEY (venue_connection_id, tenant_id)
        REFERENCES venue_connections(id, tenant_id)
        ON DELETE RESTRICT,
    CONSTRAINT venue_transfers_account_fk
        FOREIGN KEY (venue_account_id, tenant_id)
        REFERENCES venue_accounts(id, tenant_id)
        ON DELETE RESTRICT,
    CONSTRAINT venue_transfers_id_tenant_unique UNIQUE (id, tenant_id)
);

CREATE INDEX IF NOT EXISTS idx_venue_transfers_tenant_status
    ON venue_transfers (tenant_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_venue_transfers_user
    ON venue_transfers (tenant_id, user_id, created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_venue_transfers_wallet_tx
    ON venue_transfers (tenant_id, network, wallet_tx_hash)
    WHERE wallet_tx_hash IS NOT NULL;

CREATE TRIGGER trigger_venue_transfers_updated_at
    BEFORE UPDATE ON venue_transfers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

ALTER TABLE venue_transfers ENABLE ROW LEVEL SECURITY;

CREATE POLICY venue_transfers_tenant_isolation ON venue_transfers
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));

COMMENT ON TABLE venue_transfers IS
    'Lifecycle records for tracked movement between an approved wallet and a connected venue account';
