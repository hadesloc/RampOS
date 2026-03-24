CREATE TABLE IF NOT EXISTS commercialization_packs (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NULL,
    pack_code TEXT NOT NULL,
    partner_id TEXT NOT NULL REFERENCES partners(id) ON DELETE CASCADE,
    partner_capability_id TEXT NOT NULL REFERENCES partner_capabilities(id) ON DELETE CASCADE,
    commercial_extension_id TEXT NOT NULL,
    corridor_code TEXT NOT NULL,
    approval_reference TEXT NULL REFERENCES partner_approval_references(id) ON DELETE SET NULL,
    lifecycle_state TEXT NOT NULL,
    rollout_state TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT commercialization_packs_metadata_object CHECK (jsonb_typeof(metadata) = 'object'),
    CONSTRAINT commercialization_packs_unique_code UNIQUE (tenant_id, pack_code)
);

CREATE INDEX IF NOT EXISTS idx_commercialization_packs_tenant_code
    ON commercialization_packs (tenant_id, pack_code, rollout_state);

CREATE INDEX IF NOT EXISTS idx_commercialization_packs_partner
    ON commercialization_packs (partner_id, partner_capability_id);

CREATE INDEX IF NOT EXISTS idx_commercialization_packs_corridor
    ON commercialization_packs (corridor_code, commercial_extension_id);

CREATE TRIGGER trigger_commercialization_packs_updated_at
    BEFORE UPDATE ON commercialization_packs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
