-- Migration: Enterprise SSO
-- Description: Add support for Identity Providers and SSO sessions

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'sso_protocol') THEN
        CREATE TYPE sso_protocol AS ENUM ('oidc', 'saml2');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'sso_provider_type') THEN
        CREATE TYPE sso_provider_type AS ENUM ('okta', 'azure_ad', 'google', 'auth0', 'onelogin', 'custom');
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS identity_providers (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL REFERENCES tenants(id),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) NOT NULL, -- For login URLs: /auth/sso/:slug

    type sso_provider_type NOT NULL,
    protocol sso_protocol NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,

    -- Configuration (Encrypted JSON)
    config JSONB NOT NULL,

    -- Role Mapping
    role_mappings JSONB NOT NULL DEFAULT '[]'::jsonb,
    default_role VARCHAR(64) NOT NULL DEFAULT 'viewer',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_idp_slug UNIQUE (slug),
    CONSTRAINT uq_idp_tenant_slug UNIQUE (tenant_id, slug)
);

CREATE TABLE IF NOT EXISTS sso_sessions (
    id VARCHAR(64) PRIMARY KEY,
    user_id VARCHAR(64) NOT NULL,
    provider_id VARCHAR(64) NOT NULL REFERENCES identity_providers(id),

    idp_session_id VARCHAR(255),
    access_token TEXT,
    refresh_token TEXT,
    id_token TEXT,
    expires_at TIMESTAMPTZ NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_identity_providers_tenant_id ON identity_providers(tenant_id);
CREATE INDEX IF NOT EXISTS idx_sso_sessions_user_id ON sso_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sso_sessions_expires_at ON sso_sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_sso_sessions_provider_id ON sso_sessions(provider_id);

-- RLS
ALTER TABLE identity_providers ENABLE ROW LEVEL SECURITY;
ALTER TABLE sso_sessions ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_idp ON identity_providers
    USING (tenant_id = current_setting('app.current_tenant', true));

-- SSO sessions RLS: isolate by provider's tenant
CREATE POLICY tenant_isolation_sso_sessions ON sso_sessions
    FOR ALL
    USING (provider_id IN (
        SELECT id FROM identity_providers
        WHERE tenant_id = current_setting('app.current_tenant', true)
    ));
