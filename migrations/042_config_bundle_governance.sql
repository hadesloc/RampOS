CREATE TABLE IF NOT EXISTS config_bundle_exports (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    tenant_name TEXT NOT NULL,
    action_mode TEXT NOT NULL DEFAULT 'whitelisted_only',
    sections JSONB NOT NULL DEFAULT '[]'::jsonb,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    approval_status TEXT NOT NULL DEFAULT 'approved',
    rollout_scope JSONB NOT NULL DEFAULT '{}'::jsonb,
    provenance JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_config_bundle_exports_tenant_active
    ON config_bundle_exports (tenant_id, is_active, updated_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_config_bundle_exports_one_active_approved_per_tenant
    ON config_bundle_exports (tenant_id)
    WHERE tenant_id IS NOT NULL
      AND is_active = TRUE
      AND approval_status = 'approved';

CREATE UNIQUE INDEX IF NOT EXISTS idx_config_bundle_exports_one_active_approved_default
    ON config_bundle_exports ((1))
    WHERE tenant_id IS NULL
      AND is_active = TRUE
      AND approval_status = 'approved';

CREATE TABLE IF NOT EXISTS whitelisted_extension_actions (
    action_id TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    approval_required BOOLEAN NOT NULL DEFAULT TRUE,
    rollout_scope JSONB NOT NULL DEFAULT '{}'::jsonb,
    source TEXT NOT NULL DEFAULT 'registry_seed',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER trigger_config_bundle_exports_updated_at
    BEFORE UPDATE ON config_bundle_exports
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_whitelisted_extension_actions_updated_at
    BEFORE UPDATE ON whitelisted_extension_actions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

INSERT INTO whitelisted_extension_actions (
    action_id,
    label,
    description,
    enabled,
    approval_required,
    rollout_scope,
    source
)
VALUES
    (
        'branding.apply',
        'Apply branding bundle',
        'Imports approved branding fields from a config bundle.',
        TRUE,
        TRUE,
        '{"scope":"tenant"}'::jsonb,
        'registry_seed'
    ),
    (
        'domains.attach',
        'Attach domain bundle',
        'Imports approved custom-domain configuration.',
        TRUE,
        TRUE,
        '{"scope":"tenant"}'::jsonb,
        'registry_seed'
    ),
    (
        'webhooks.sync',
        'Sync webhook preferences',
        'Imports approved webhook event selections only.',
        TRUE,
        TRUE,
        '{"scope":"tenant"}'::jsonb,
        'registry_seed'
    )
ON CONFLICT (action_id) DO NOTHING;
