-- Migration: Custom Domains
-- Description: Add support for tenant custom domains

CREATE TYPE domain_status AS ENUM (
    'pending_dns_verification',
    'pending_ssl',
    'provisioning_ssl',
    'active',
    'expiring_soon',
    'expired',
    'dns_verification_failed',
    'ssl_provisioning_failed',
    'disabled'
);

CREATE TABLE custom_domains (
    id VARCHAR(64) PRIMARY KEY,
    tenant_id VARCHAR(64) NOT NULL,
    domain VARCHAR(255) NOT NULL,
    status domain_status NOT NULL DEFAULT 'pending_dns_verification',

    -- Verification
    dns_verification_token VARCHAR(255),
    dns_verification_record VARCHAR(255),

    -- SSL Info (JSON)
    ssl_certificate JSONB,

    -- Health & Config
    health_check_path VARCHAR(255) NOT NULL DEFAULT '/health',
    last_health_check JSONB,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    custom_headers JSONB DEFAULT '{}'::jsonb,
    redirects JSONB DEFAULT '[]'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT uq_domain UNIQUE (domain)
);

-- Indexes
CREATE INDEX idx_custom_domains_tenant_id ON custom_domains(tenant_id);
CREATE INDEX idx_custom_domains_status ON custom_domains(status);

-- RLS
ALTER TABLE custom_domains ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_custom_domains ON custom_domains
    USING (tenant_id = current_setting('app.current_tenant_id', true));
