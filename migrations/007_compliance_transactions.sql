CREATE TABLE compliance_transactions (
    id UUID PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    user_id VARCHAR(64) NOT NULL,
    intent_id VARCHAR(64) NOT NULL,
    transaction_type VARCHAR(32) NOT NULL,
    amount_vnd DECIMAL(30, 8) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_compliance_tx_tenant_user_time
    ON compliance_transactions (tenant_id, user_id, created_at DESC);

CREATE INDEX idx_compliance_tx_type
    ON compliance_transactions (transaction_type);
