CREATE TABLE IF NOT EXISTS corridor_packs (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    corridor_code TEXT NOT NULL,
    source_market TEXT NOT NULL,
    destination_market TEXT NOT NULL,
    source_currency TEXT NOT NULL,
    destination_currency TEXT NOT NULL,
    settlement_direction TEXT NOT NULL,
    fee_model TEXT NOT NULL DEFAULT 'shared',
    lifecycle_state TEXT NOT NULL DEFAULT 'draft',
    rollout_state TEXT NOT NULL DEFAULT 'planned',
    eligibility_state TEXT NOT NULL DEFAULT 'restricted',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT corridor_packs_metadata_object CHECK (jsonb_typeof(metadata) = 'object'),
    CONSTRAINT corridor_packs_unique_code UNIQUE (tenant_id, corridor_code)
);

CREATE INDEX IF NOT EXISTS idx_corridor_packs_tenant_code
    ON corridor_packs (tenant_id, corridor_code, lifecycle_state);

CREATE TABLE IF NOT EXISTS corridor_pack_endpoints (
    id TEXT PRIMARY KEY,
    corridor_pack_id TEXT NOT NULL REFERENCES corridor_packs(id) ON DELETE CASCADE,
    endpoint_role TEXT NOT NULL,
    partner_id TEXT NULL REFERENCES partners(id) ON DELETE SET NULL,
    provider_key TEXT NULL,
    adapter_key TEXT NULL,
    entity_type TEXT NOT NULL,
    rail TEXT NOT NULL,
    method_family TEXT NULL,
    settlement_mode TEXT NULL,
    instrument_family TEXT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT corridor_pack_endpoints_role_check CHECK (endpoint_role IN ('source', 'destination')),
    CONSTRAINT corridor_pack_endpoints_metadata_object CHECK (jsonb_typeof(metadata) = 'object')
);

CREATE INDEX IF NOT EXISTS idx_corridor_pack_endpoints_pack_role
    ON corridor_pack_endpoints (corridor_pack_id, endpoint_role);

CREATE TABLE IF NOT EXISTS corridor_fee_profiles (
    id TEXT PRIMARY KEY,
    corridor_pack_id TEXT NOT NULL REFERENCES corridor_packs(id) ON DELETE CASCADE,
    fee_currency TEXT NOT NULL,
    base_fee NUMERIC(20, 8) NULL,
    fx_spread_bps INTEGER NULL,
    liquidity_cost_bps INTEGER NULL,
    surcharge_bps INTEGER NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT corridor_fee_profiles_metadata_object CHECK (jsonb_typeof(metadata) = 'object')
);

CREATE INDEX IF NOT EXISTS idx_corridor_fee_profiles_pack
    ON corridor_fee_profiles (corridor_pack_id);

CREATE TABLE IF NOT EXISTS corridor_cutoff_policies (
    id TEXT PRIMARY KEY,
    corridor_pack_id TEXT NOT NULL REFERENCES corridor_packs(id) ON DELETE CASCADE,
    timezone TEXT NOT NULL,
    cutoff_windows JSONB NOT NULL DEFAULT '[]'::jsonb,
    holiday_calendar TEXT NULL,
    retry_rule TEXT NULL,
    exception_policy TEXT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT corridor_cutoff_policies_windows_array CHECK (jsonb_typeof(cutoff_windows) = 'array'),
    CONSTRAINT corridor_cutoff_policies_metadata_object CHECK (jsonb_typeof(metadata) = 'object')
);

CREATE INDEX IF NOT EXISTS idx_corridor_cutoff_policies_pack
    ON corridor_cutoff_policies (corridor_pack_id);

CREATE TABLE IF NOT EXISTS corridor_compliance_hooks (
    id TEXT PRIMARY KEY,
    corridor_pack_id TEXT NOT NULL REFERENCES corridor_packs(id) ON DELETE CASCADE,
    hook_kind TEXT NOT NULL,
    provider_key TEXT NULL,
    required BOOLEAN NOT NULL DEFAULT FALSE,
    config JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT corridor_compliance_hooks_config_object CHECK (jsonb_typeof(config) = 'object'),
    CONSTRAINT corridor_compliance_hooks_metadata_object CHECK (jsonb_typeof(metadata) = 'object')
);

CREATE INDEX IF NOT EXISTS idx_corridor_compliance_hooks_pack
    ON corridor_compliance_hooks (corridor_pack_id, hook_kind);

CREATE TABLE IF NOT EXISTS corridor_rollout_scopes (
    id TEXT PRIMARY KEY,
    corridor_pack_id TEXT NOT NULL REFERENCES corridor_packs(id) ON DELETE CASCADE,
    tenant_id TEXT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    environment TEXT NOT NULL DEFAULT 'sandbox',
    geography TEXT NULL,
    method_family TEXT NULL,
    rollout_state TEXT NOT NULL DEFAULT 'planned',
    approval_reference TEXT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT corridor_rollout_scopes_metadata_object CHECK (jsonb_typeof(metadata) = 'object')
);

CREATE INDEX IF NOT EXISTS idx_corridor_rollout_scopes_pack_tenant
    ON corridor_rollout_scopes (corridor_pack_id, tenant_id, environment);

CREATE TABLE IF NOT EXISTS corridor_eligibility_rules (
    id TEXT PRIMARY KEY,
    corridor_pack_id TEXT NOT NULL REFERENCES corridor_packs(id) ON DELETE CASCADE,
    partner_id TEXT NULL REFERENCES partners(id) ON DELETE SET NULL,
    entity_type TEXT NULL,
    method_family TEXT NULL,
    amount_bounds JSONB NOT NULL DEFAULT '{}'::jsonb,
    compliance_requirements JSONB NOT NULL DEFAULT '[]'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT corridor_eligibility_rules_amount_bounds_object CHECK (jsonb_typeof(amount_bounds) = 'object'),
    CONSTRAINT corridor_eligibility_rules_requirements_array CHECK (jsonb_typeof(compliance_requirements) = 'array'),
    CONSTRAINT corridor_eligibility_rules_metadata_object CHECK (jsonb_typeof(metadata) = 'object')
);

CREATE INDEX IF NOT EXISTS idx_corridor_eligibility_rules_pack_partner
    ON corridor_eligibility_rules (corridor_pack_id, partner_id);

CREATE TRIGGER trigger_corridor_packs_updated_at
    BEFORE UPDATE ON corridor_packs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_corridor_pack_endpoints_updated_at
    BEFORE UPDATE ON corridor_pack_endpoints
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_corridor_fee_profiles_updated_at
    BEFORE UPDATE ON corridor_fee_profiles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_corridor_cutoff_policies_updated_at
    BEFORE UPDATE ON corridor_cutoff_policies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_corridor_compliance_hooks_updated_at
    BEFORE UPDATE ON corridor_compliance_hooks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_corridor_rollout_scopes_updated_at
    BEFORE UPDATE ON corridor_rollout_scopes
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_corridor_eligibility_rules_updated_at
    BEFORE UPDATE ON corridor_eligibility_rules
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
