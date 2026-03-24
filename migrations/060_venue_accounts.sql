-- Migration 060: Venue accounts
-- Stores actual venue account identifiers underneath a logical venue connection.

CREATE TABLE IF NOT EXISTS venue_accounts (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    venue_connection_id TEXT NOT NULL,
    venue_key TEXT NOT NULL,
    account_label TEXT NULL,
    account_ref TEXT NULL,
    wallet_address TEXT NULL,
    subaccount_ref TEXT NULL,
    api_scope_summary JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'pending',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_verified_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT venue_accounts_identity_check CHECK (
        account_ref IS NOT NULL
        OR wallet_address IS NOT NULL
        OR subaccount_ref IS NOT NULL
    ),
    CONSTRAINT venue_accounts_status_check CHECK (
        status IN ('pending', 'active', 'restricted', 'revoked', 'archived')
    ),
    CONSTRAINT venue_accounts_api_scope_summary_shape CHECK (
        jsonb_typeof(api_scope_summary) IN ('object', 'array')
    ),
    CONSTRAINT venue_accounts_metadata_object CHECK (
        jsonb_typeof(metadata) = 'object'
    ),
    CONSTRAINT venue_accounts_connection_fk
        FOREIGN KEY (venue_connection_id, tenant_id)
        REFERENCES venue_connections(id, tenant_id)
        ON DELETE CASCADE,
    CONSTRAINT venue_accounts_id_tenant_unique UNIQUE (id, tenant_id)
);

CREATE INDEX IF NOT EXISTS idx_venue_accounts_connection_status
    ON venue_accounts (tenant_id, venue_connection_id, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_venue_accounts_venue_locator
    ON venue_accounts (tenant_id, venue_key, account_ref, subaccount_ref);

CREATE INDEX IF NOT EXISTS idx_venue_accounts_wallet_address
    ON venue_accounts (tenant_id, wallet_address)
    WHERE wallet_address IS NOT NULL;

CREATE TRIGGER trigger_venue_accounts_updated_at
    BEFORE UPDATE ON venue_accounts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

ALTER TABLE venue_accounts ENABLE ROW LEVEL SECURITY;

CREATE POLICY venue_accounts_tenant_isolation ON venue_accounts
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));

COMMENT ON TABLE venue_accounts IS
    'Normalized venue account identifiers such as account refs, wallet addresses, or subaccounts';
