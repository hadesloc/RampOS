CREATE TABLE IF NOT EXISTS provider_routing_policies (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider_family TEXT NOT NULL,
    policy_name TEXT NOT NULL,
    corridor_code TEXT NULL,
    entity_type TEXT NULL,
    risk_tier TEXT NULL,
    partner_key TEXT NULL,
    asset_code TEXT NULL,
    amount_min NUMERIC NULL,
    amount_max NUMERIC NULL,
    fallback_order JSONB NOT NULL DEFAULT '[]'::jsonb,
    scorecard JSONB NOT NULL DEFAULT '{}'::jsonb,
    provider_weights JSONB NOT NULL DEFAULT '{}'::jsonb,
    lifecycle_state TEXT NOT NULL DEFAULT 'active',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_provider_routing_policies_lookup
    ON provider_routing_policies (
        tenant_id,
        provider_family,
        corridor_code,
        entity_type,
        risk_tier,
        partner_key,
        asset_code
    );

CREATE INDEX IF NOT EXISTS idx_provider_routing_policies_state
    ON provider_routing_policies (tenant_id, provider_family, lifecycle_state, policy_name);

CREATE TRIGGER trigger_provider_routing_policies_updated_at
    BEFORE UPDATE ON provider_routing_policies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
