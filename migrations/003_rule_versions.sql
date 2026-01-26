-- ============================================================================
-- AML RULE VERSIONS
-- ============================================================================

CREATE TABLE aml_rule_versions (
    id UUID PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    version_number INT NOT NULL,
    rules_json JSONB NOT NULL,
    is_active BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255),
    activated_at TIMESTAMPTZ,

    -- Ensure unique version numbers per tenant
    UNIQUE (tenant_id, version_number)
);

CREATE INDEX idx_rule_versions_tenant ON aml_rule_versions(tenant_id, version_number DESC);
CREATE INDEX idx_rule_versions_active ON aml_rule_versions(tenant_id) WHERE is_active = true;
