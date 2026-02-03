-- ============================================================================
-- SMART ACCOUNTS TABLE
-- Stores mapping between smart account addresses and tenants/users
-- Required for account ownership verification in AA (Account Abstraction) APIs
-- ============================================================================

-- Smart accounts table for ERC-4337 account abstraction
CREATE TABLE IF NOT EXISTS smart_accounts (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    user_id VARCHAR(64) NOT NULL,

    -- Smart account address (Ethereum format: 0x...)
    address VARCHAR(42) NOT NULL,

    -- Owner address (the EOA that controls this smart account)
    owner_address VARCHAR(42) NOT NULL,

    -- Account type (e.g., SimpleAccount, Safe, Biconomy, etc.)
    account_type VARCHAR(64) NOT NULL DEFAULT 'SimpleAccount',

    -- Chain information
    chain_id BIGINT NOT NULL,

    -- Factory address used to deploy this account
    factory_address VARCHAR(42),

    -- Entry point address (ERC-4337)
    entry_point_address VARCHAR(42),

    -- Deployment status
    is_deployed BOOLEAN NOT NULL DEFAULT FALSE,
    deployed_at TIMESTAMPTZ,
    deployment_tx_hash VARCHAR(66),

    -- Status
    status VARCHAR(32) NOT NULL DEFAULT 'ACTIVE',

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT smart_accounts_status_check CHECK (status IN ('ACTIVE', 'DISABLED', 'FROZEN')),
    CONSTRAINT smart_accounts_address_format CHECK (address ~ '^0x[a-fA-F0-9]{40}$'),
    CONSTRAINT smart_accounts_owner_format CHECK (owner_address ~ '^0x[a-fA-F0-9]{40}$')
);

-- Unique constraint: One address per chain (globally unique on each chain)
CREATE UNIQUE INDEX IF NOT EXISTS idx_smart_accounts_address_chain
    ON smart_accounts(address, chain_id);

-- Index for tenant + user lookup (most common query pattern)
CREATE INDEX IF NOT EXISTS idx_smart_accounts_tenant_user
    ON smart_accounts(tenant_id, user_id);

-- Index for tenant + address lookup (ownership verification)
CREATE INDEX IF NOT EXISTS idx_smart_accounts_tenant_address
    ON smart_accounts(tenant_id, address);

-- Index for owner address lookup
CREATE INDEX IF NOT EXISTS idx_smart_accounts_owner
    ON smart_accounts(owner_address);

-- Index for chain filtering
CREATE INDEX IF NOT EXISTS idx_smart_accounts_chain
    ON smart_accounts(chain_id);

-- Trigger for updated_at
CREATE TRIGGER trigger_smart_accounts_updated_at
    BEFORE UPDATE ON smart_accounts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- Enable RLS (Row Level Security) for smart_accounts
ALTER TABLE smart_accounts ENABLE ROW LEVEL SECURITY;

-- RLS Policy: Tenants can only see their own smart accounts
CREATE POLICY smart_accounts_tenant_isolation ON smart_accounts
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));

-- Comment for documentation
COMMENT ON TABLE smart_accounts IS 'ERC-4337 smart account registry for account ownership verification';
COMMENT ON COLUMN smart_accounts.address IS 'Smart account address (counterfactual or deployed)';
COMMENT ON COLUMN smart_accounts.owner_address IS 'EOA address that controls this smart account';
COMMENT ON COLUMN smart_accounts.is_deployed IS 'Whether the smart account has been deployed on-chain';
