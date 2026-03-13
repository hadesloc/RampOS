CREATE TABLE IF NOT EXISTS partners (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    partner_class TEXT NOT NULL,
    code TEXT NOT NULL,
    display_name TEXT NOT NULL,
    legal_name TEXT NULL,
    market TEXT NULL,
    jurisdiction TEXT NULL,
    service_domain TEXT NOT NULL,
    lifecycle_state TEXT NOT NULL DEFAULT 'draft',
    approval_status TEXT NOT NULL DEFAULT 'pending',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_partners_tenant_code
    ON partners (tenant_id, code);

CREATE TABLE IF NOT EXISTS partner_capabilities (
    id TEXT PRIMARY KEY,
    partner_id TEXT NOT NULL REFERENCES partners(id) ON DELETE CASCADE,
    capability_family TEXT NOT NULL,
    environment TEXT NOT NULL DEFAULT 'sandbox',
    adapter_key TEXT NULL,
    provider_key TEXT NULL,
    supported_rails JSONB NOT NULL DEFAULT '[]'::jsonb,
    supported_methods JSONB NOT NULL DEFAULT '[]'::jsonb,
    approval_status TEXT NOT NULL DEFAULT 'pending',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_partner_capabilities_partner_id
    ON partner_capabilities (partner_id, capability_family);

CREATE TABLE IF NOT EXISTS partner_rollout_scopes (
    id TEXT PRIMARY KEY,
    partner_capability_id TEXT NOT NULL REFERENCES partner_capabilities(id) ON DELETE CASCADE,
    tenant_id TEXT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    environment TEXT NOT NULL DEFAULT 'sandbox',
    corridor_code TEXT NULL,
    geography TEXT NULL,
    method_family TEXT NULL,
    rollout_state TEXT NOT NULL DEFAULT 'planned',
    rollback_target TEXT NULL,
    approval_reference TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_partner_rollout_scopes_capability_tenant
    ON partner_rollout_scopes (partner_capability_id, tenant_id, environment);

CREATE TABLE IF NOT EXISTS partner_approval_references (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    action_class TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_partner_approval_references_tenant_status
    ON partner_approval_references (tenant_id, status, action_class);

ALTER TABLE partner_rollout_scopes
    ADD CONSTRAINT fk_partner_rollout_scopes_approval_reference
    FOREIGN KEY (approval_reference) REFERENCES partner_approval_references(id);

CREATE TABLE IF NOT EXISTS partner_health_signals (
    id TEXT PRIMARY KEY,
    partner_capability_id TEXT NOT NULL REFERENCES partner_capabilities(id) ON DELETE CASCADE,
    status TEXT NOT NULL,
    source TEXT NOT NULL,
    score INTEGER NULL,
    incident_summary TEXT NULL,
    evidence JSONB NOT NULL DEFAULT '{}'::jsonb,
    observed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_partner_health_signals_capability_observed
    ON partner_health_signals (partner_capability_id, observed_at DESC);

CREATE TABLE IF NOT EXISTS credential_references (
    id TEXT PRIMARY KEY,
    partner_id TEXT NOT NULL REFERENCES partners(id) ON DELETE CASCADE,
    credential_kind TEXT NOT NULL,
    locator TEXT NOT NULL,
    environment TEXT NOT NULL DEFAULT 'sandbox',
    approval_reference TEXT NULL,
    rotation_metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_credential_references_partner_environment
    ON credential_references (partner_id, environment, credential_kind);

ALTER TABLE credential_references
    ADD CONSTRAINT fk_credential_references_approval_reference
    FOREIGN KEY (approval_reference) REFERENCES partner_approval_references(id);

CREATE TRIGGER trigger_partners_updated_at
    BEFORE UPDATE ON partners
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_partner_capabilities_updated_at
    BEFORE UPDATE ON partner_capabilities
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_partner_rollout_scopes_updated_at
    BEFORE UPDATE ON partner_rollout_scopes
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_partner_approval_references_updated_at
    BEFORE UPDATE ON partner_approval_references
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_credential_references_updated_at
    BEFORE UPDATE ON credential_references
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
