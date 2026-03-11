CREATE TABLE IF NOT EXISTS kyc_passport_vault (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    source_tenant_id TEXT NOT NULL,
    status TEXT NOT NULL,
    kyc_tier SMALLINT NOT NULL DEFAULT 0,
    fields_shared JSONB NOT NULL DEFAULT '[]'::jsonb,
    verified_at TIMESTAMPTZ NULL,
    expires_at TIMESTAMPTZ NULL,
    revoked_at TIMESTAMPTZ NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_kyc_passport_vault_tenant_user
    ON kyc_passport_vault (tenant_id, user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS kyc_passport_consent_grants (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    passport_id TEXT NOT NULL REFERENCES kyc_passport_vault(id) ON DELETE CASCADE,
    target_tenant_id TEXT NOT NULL,
    consent_status TEXT NOT NULL,
    scope JSONB NOT NULL DEFAULT '{}'::jsonb,
    granted_at TIMESTAMPTZ NULL,
    revoked_at TIMESTAMPTZ NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_kyc_passport_consent_target
    ON kyc_passport_consent_grants (tenant_id, target_tenant_id, consent_status);

CREATE TABLE IF NOT EXISTS kyc_passport_acceptance_policies (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    min_tier SMALLINT NOT NULL DEFAULT 0,
    max_age_days INTEGER NOT NULL DEFAULT 30,
    allowed_source_tenants JSONB NOT NULL DEFAULT '[]'::jsonb,
    requires_manual_review BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
