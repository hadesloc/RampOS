-- Migration 034: LP (Liquidity Provider) registration
-- Stores registered LP keys for authentication to the RFQ auction market.
-- Each LP is identified by a key in format: lp_id:tenant_id:secret_hash
-- This replaces the "honor system" auth with actual key validation.

CREATE TABLE registered_lp_keys (
    id              TEXT PRIMARY KEY,               -- "lp_..." prefix
    tenant_id       TEXT NOT NULL REFERENCES tenants(id),

    -- LP identity
    lp_id           TEXT NOT NULL,
    lp_name         TEXT,

    -- Auth (SHA-256 hash of the secret, never store raw)
    key_hash        TEXT NOT NULL,

    -- Permissions
    can_bid_offramp BOOLEAN NOT NULL DEFAULT true,  -- can bid on OFFRAMP RFQs
    can_bid_onramp  BOOLEAN NOT NULL DEFAULT true,  -- can bid on ONRAMP RFQs
    max_bid_amount  NUMERIC,                        -- optional cap per bid

    -- Lifecycle
    is_active       BOOLEAN NOT NULL DEFAULT true,
    expires_at      TIMESTAMPTZ,                    -- NULL = never expires

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (tenant_id, lp_id),
    UNIQUE (tenant_id, key_hash)
);

CREATE INDEX idx_lp_keys_tenant ON registered_lp_keys(tenant_id, is_active);
CREATE INDEX idx_lp_keys_hash ON registered_lp_keys(tenant_id, key_hash);

ALTER TABLE registered_lp_keys ENABLE ROW LEVEL SECURITY;

CREATE POLICY lp_keys_tenant_isolation ON registered_lp_keys
    USING (tenant_id = current_setting('app.current_tenant', true));

CREATE TRIGGER trigger_lp_keys_updated_at
    BEFORE UPDATE ON registered_lp_keys
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Insert a sample test LP for dev/test environments only
-- (production should use proper LP onboarding flow)
-- REMOVE IN PRODUCTION:
-- INSERT INTO registered_lp_keys (id, tenant_id, lp_id, lp_name, key_hash)
-- VALUES ('lp_test_001', 'tenant_test', 'lp_test', 'Test LP', sha256('test_secret_change_me'::bytea)::text);
