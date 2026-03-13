CREATE TABLE IF NOT EXISTS treasury_evidence_imports (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    source_family TEXT NOT NULL,
    source_ref TEXT NOT NULL,
    account_scope TEXT NOT NULL,
    asset_code TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    snapshot_at TIMESTAMPTZ NOT NULL,
    available_balance NUMERIC NOT NULL DEFAULT 0,
    reserved_balance NUMERIC NOT NULL DEFAULT 0,
    source_lineage JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    imported_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_treasury_evidence_imports_tenant_idempotency
    ON treasury_evidence_imports (tenant_id, idempotency_key);

CREATE INDEX IF NOT EXISTS idx_treasury_evidence_imports_tenant_source_snapshot
    ON treasury_evidence_imports (tenant_id, source_family, snapshot_at DESC);

CREATE INDEX IF NOT EXISTS idx_treasury_evidence_imports_account_scope
    ON treasury_evidence_imports (tenant_id, account_scope, asset_code, snapshot_at DESC);

CREATE TRIGGER trigger_treasury_evidence_imports_updated_at
    BEFORE UPDATE ON treasury_evidence_imports
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
