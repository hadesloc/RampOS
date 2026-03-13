CREATE TABLE IF NOT EXISTS kyb_evidence_packages (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    institution_entity_id TEXT NOT NULL REFERENCES kyb_entities(id) ON DELETE CASCADE,
    institution_legal_name TEXT NOT NULL,
    provider_family TEXT NOT NULL,
    provider_policy_id TEXT NULL,
    corridor_code TEXT NULL,
    review_status TEXT NOT NULL DEFAULT 'pending',
    review_notes TEXT NULL,
    export_status TEXT NOT NULL DEFAULT 'not_exported',
    export_artifact_uri TEXT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    exported_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_kyb_evidence_packages_tenant_review
    ON kyb_evidence_packages (tenant_id, review_status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_kyb_evidence_packages_entity
    ON kyb_evidence_packages (institution_entity_id, corridor_code);

CREATE TABLE IF NOT EXISTS kyb_evidence_sources (
    id TEXT PRIMARY KEY,
    package_id TEXT NOT NULL REFERENCES kyb_evidence_packages(id) ON DELETE CASCADE,
    source_kind TEXT NOT NULL,
    source_ref TEXT NOT NULL,
    document_id TEXT NULL,
    collected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_kyb_evidence_sources_package
    ON kyb_evidence_sources (package_id, source_kind, collected_at DESC);

CREATE TABLE IF NOT EXISTS kyb_ubo_evidence_links (
    id TEXT PRIMARY KEY,
    package_id TEXT NOT NULL REFERENCES kyb_evidence_packages(id) ON DELETE CASCADE,
    owner_entity_id TEXT NOT NULL REFERENCES kyb_entities(id) ON DELETE CASCADE,
    ownership_pct NUMERIC(5,2) NULL,
    evidence_source_ref TEXT NULL,
    review_state TEXT NOT NULL DEFAULT 'pending',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_kyb_ubo_evidence_links_package
    ON kyb_ubo_evidence_links (package_id, review_state, owner_entity_id);

CREATE TRIGGER trigger_kyb_evidence_packages_updated_at
    BEFORE UPDATE ON kyb_evidence_packages
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_kyb_evidence_sources_updated_at
    BEFORE UPDATE ON kyb_evidence_sources
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trigger_kyb_ubo_evidence_links_updated_at
    BEFORE UPDATE ON kyb_ubo_evidence_links
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
