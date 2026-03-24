-- Migration 051: Wallet attestations - governed wallet proof for venue funding
-- Tracks tenant/user/wallet trust posture before allowing venue transfers.

CREATE TABLE IF NOT EXISTS wallet_attestations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       VARCHAR(64) NOT NULL REFERENCES tenants(id),
    user_id         VARCHAR(64) NOT NULL,
    wallet_address  TEXT NOT NULL,
    chain_id        TEXT NOT NULL,
    attestation_status TEXT NOT NULL DEFAULT 'pending' CHECK (attestation_status IN ('pending','verified','rejected','flagged')),
    proof_kind      TEXT NOT NULL DEFAULT 'unknown',
    proof_artifact_uri TEXT,
    risk_state      TEXT NOT NULL DEFAULT 'unknown',
    last_verified_at TIMESTAMPTZ,
    metadata        JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT wallet_attestations_user_fk
        FOREIGN KEY (tenant_id, user_id) REFERENCES users(tenant_id, id)
);

CREATE INDEX IF NOT EXISTS idx_wallet_attestations_tenant_user ON wallet_attestations (tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_wallet_attestations_wallet_chain ON wallet_attestations (wallet_address, chain_id);
CREATE INDEX IF NOT EXISTS idx_wallet_attestations_status ON wallet_attestations (attestation_status);

CREATE OR REPLACE FUNCTION update_wallet_attestations_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_wallet_attestations_updated_at
    BEFORE UPDATE ON wallet_attestations
    FOR EACH ROW
    EXECUTE FUNCTION update_wallet_attestations_updated_at();
