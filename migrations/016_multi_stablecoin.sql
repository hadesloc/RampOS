-- Multi-Stablecoin Support Migration
-- Adds supported_tokens table and token_balances view for multi-token wallet support

-- Supported tokens table - stores token configurations per tenant
CREATE TABLE IF NOT EXISTS supported_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(255) NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,

    -- Token identification
    symbol VARCHAR(20) NOT NULL,
    name VARCHAR(100) NOT NULL,
    decimals SMALLINT NOT NULL DEFAULT 18,

    -- Token metadata
    logo_url TEXT,
    website TEXT,
    description TEXT,

    -- Configuration
    enabled BOOLEAN NOT NULL DEFAULT true,
    min_deposit NUMERIC(78, 0) NOT NULL DEFAULT 0, -- U256 max is 78 digits
    max_deposit NUMERIC(78, 0),
    min_withdraw NUMERIC(78, 0) NOT NULL DEFAULT 0,
    max_withdraw NUMERIC(78, 0),
    deposit_fee_bps SMALLINT NOT NULL DEFAULT 0,
    withdraw_fee_bps SMALLINT NOT NULL DEFAULT 10,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_tenant_token UNIQUE (tenant_id, symbol)
);

-- Token chain deployments - stores contract addresses per chain
CREATE TABLE IF NOT EXISTS token_chain_deployments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_id UUID NOT NULL REFERENCES supported_tokens(id) ON DELETE CASCADE,

    -- Chain info
    chain_id BIGINT NOT NULL,
    chain_name VARCHAR(50) NOT NULL,
    contract_address VARCHAR(66) NOT NULL, -- 0x + 64 hex chars or similar

    -- Deployment status
    is_native BOOLEAN NOT NULL DEFAULT false,
    bridge_contract VARCHAR(66),
    enabled BOOLEAN NOT NULL DEFAULT true,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_token_chain UNIQUE (token_id, chain_id)
);

-- Token balances table - tracks user token balances per chain
CREATE TABLE IF NOT EXISTS token_balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(255) NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id VARCHAR(255) NOT NULL,

    -- Token and chain
    symbol VARCHAR(20) NOT NULL,
    chain_id BIGINT NOT NULL,

    -- Balance (stored as string for U256 precision)
    balance NUMERIC(78, 0) NOT NULL DEFAULT 0,
    pending_deposits NUMERIC(78, 0) NOT NULL DEFAULT 0,
    pending_withdrawals NUMERIC(78, 0) NOT NULL DEFAULT 0,

    -- Last update info
    last_synced_block BIGINT,
    last_sync_at TIMESTAMPTZ,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_user_token_chain UNIQUE (tenant_id, user_id, symbol, chain_id),
    CONSTRAINT fk_user FOREIGN KEY (tenant_id, user_id)
        REFERENCES users(tenant_id, id) ON DELETE CASCADE
);

-- Token transactions table - tracks on-chain token movements
CREATE TABLE IF NOT EXISTS token_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id VARCHAR(255) NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id VARCHAR(255) NOT NULL,
    intent_id VARCHAR(255),

    -- Transaction info
    tx_hash VARCHAR(66) NOT NULL,
    chain_id BIGINT NOT NULL,
    block_number BIGINT,

    -- Token info
    symbol VARCHAR(20) NOT NULL,
    amount NUMERIC(78, 0) NOT NULL,

    -- Addresses
    from_address VARCHAR(66) NOT NULL,
    to_address VARCHAR(66) NOT NULL,

    -- Type and status
    tx_type VARCHAR(20) NOT NULL, -- DEPOSIT, WITHDRAW, TRANSFER, SWAP
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING', -- PENDING, CONFIRMED, FAILED
    confirmations INTEGER NOT NULL DEFAULT 0,

    -- Fees
    gas_used NUMERIC(78, 0),
    gas_price NUMERIC(78, 0),
    fee_amount NUMERIC(78, 0),
    fee_currency VARCHAR(20),

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ,

    CONSTRAINT fk_user_tx FOREIGN KEY (tenant_id, user_id)
        REFERENCES users(tenant_id, id) ON DELETE CASCADE
);

-- Create view for aggregated token balances per user
CREATE OR REPLACE VIEW user_token_balances AS
SELECT
    tb.tenant_id,
    tb.user_id,
    tb.symbol,
    st.name as token_name,
    st.decimals,
    st.logo_url,
    SUM(tb.balance) as total_balance,
    SUM(tb.pending_deposits) as total_pending_deposits,
    SUM(tb.pending_withdrawals) as total_pending_withdrawals,
    jsonb_agg(jsonb_build_object(
        'chain_id', tb.chain_id,
        'balance', tb.balance::text,
        'pending_deposits', tb.pending_deposits::text,
        'pending_withdrawals', tb.pending_withdrawals::text
    )) as chain_balances
FROM token_balances tb
JOIN supported_tokens st ON st.tenant_id = tb.tenant_id AND st.symbol = tb.symbol
WHERE st.enabled = true
GROUP BY tb.tenant_id, tb.user_id, tb.symbol, st.name, st.decimals, st.logo_url;

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_supported_tokens_tenant ON supported_tokens(tenant_id);
CREATE INDEX IF NOT EXISTS idx_supported_tokens_symbol ON supported_tokens(symbol);
CREATE INDEX IF NOT EXISTS idx_token_deployments_chain ON token_chain_deployments(chain_id);
CREATE INDEX IF NOT EXISTS idx_token_balances_user ON token_balances(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_token_balances_symbol ON token_balances(symbol, chain_id);
CREATE INDEX IF NOT EXISTS idx_token_transactions_user ON token_transactions(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_token_transactions_hash ON token_transactions(tx_hash);
CREATE INDEX IF NOT EXISTS idx_token_transactions_intent ON token_transactions(intent_id);
CREATE INDEX IF NOT EXISTS idx_token_transactions_status ON token_transactions(status, chain_id);

-- Enable RLS
ALTER TABLE supported_tokens ENABLE ROW LEVEL SECURITY;
ALTER TABLE token_chain_deployments ENABLE ROW LEVEL SECURITY;
ALTER TABLE token_balances ENABLE ROW LEVEL SECURITY;
ALTER TABLE token_transactions ENABLE ROW LEVEL SECURITY;

-- RLS policies for supported_tokens
CREATE POLICY supported_tokens_tenant_isolation ON supported_tokens
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true));

-- RLS policies for token_chain_deployments (via token)
CREATE POLICY token_deployments_tenant_isolation ON token_chain_deployments
    FOR ALL
    USING (token_id IN (
        SELECT id FROM supported_tokens
        WHERE tenant_id = current_setting('app.current_tenant', true)
    ));

-- RLS policies for token_balances
CREATE POLICY token_balances_tenant_isolation ON token_balances
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true));

-- RLS policies for token_transactions
CREATE POLICY token_transactions_tenant_isolation ON token_transactions
    FOR ALL
    USING (tenant_id = current_setting('app.current_tenant', true));

-- Update timestamp trigger function (if not exists)
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply update triggers
CREATE TRIGGER supported_tokens_updated_at
    BEFORE UPDATE ON supported_tokens
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER token_chain_deployments_updated_at
    BEFORE UPDATE ON token_chain_deployments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER token_balances_updated_at
    BEFORE UPDATE ON token_balances
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER token_transactions_updated_at
    BEFORE UPDATE ON token_transactions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default supported tokens for new tenants (can be customized per tenant)
-- This would typically be done via application logic, not migration
COMMENT ON TABLE supported_tokens IS 'Stablecoin configurations per tenant';
COMMENT ON TABLE token_chain_deployments IS 'Token contract addresses per chain';
COMMENT ON TABLE token_balances IS 'User token balances per chain';
COMMENT ON TABLE token_transactions IS 'On-chain token transaction history';
COMMENT ON VIEW user_token_balances IS 'Aggregated token balances view per user';
