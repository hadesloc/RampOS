-- Migration 059: Venue connections
-- Tenant-scoped logical links between a subject and a venue without hard-coding
-- any connector-specific account semantics.

CREATE TABLE IF NOT EXISTS venue_connections (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    subject_type TEXT NOT NULL DEFAULT 'user',
    subject_id TEXT NOT NULL,
    user_id TEXT NULL,
    venue_key TEXT NOT NULL,
    connection_mode TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_verified_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT venue_connections_subject_check CHECK (
        (subject_type = 'user' AND user_id IS NOT NULL AND subject_id = user_id)
        OR (subject_type <> 'user' AND user_id IS NULL)
    ),
    CONSTRAINT venue_connections_connection_mode_check CHECK (
        connection_mode IN ('read_only', 'wallet_linked', 'api_key', 'operator')
    ),
    CONSTRAINT venue_connections_status_check CHECK (
        status IN ('pending', 'active', 'restricted', 'revoked', 'archived')
    ),
    CONSTRAINT venue_connections_metadata_object CHECK (
        jsonb_typeof(metadata) = 'object'
    ),
    CONSTRAINT venue_connections_user_fk
        FOREIGN KEY (tenant_id, user_id) REFERENCES users(tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT venue_connections_unique_subject UNIQUE (
        tenant_id,
        subject_type,
        subject_id,
        venue_key
    ),
    CONSTRAINT venue_connections_id_tenant_unique UNIQUE (id, tenant_id)
);

CREATE INDEX IF NOT EXISTS idx_venue_connections_tenant_subject
    ON venue_connections (tenant_id, subject_type, subject_id, status);

CREATE INDEX IF NOT EXISTS idx_venue_connections_tenant_venue
    ON venue_connections (tenant_id, venue_key, status, created_at DESC);

CREATE TRIGGER trigger_venue_connections_updated_at
    BEFORE UPDATE ON venue_connections
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

ALTER TABLE venue_connections ENABLE ROW LEVEL SECURITY;

CREATE POLICY venue_connections_tenant_isolation ON venue_connections
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));

COMMENT ON TABLE venue_connections IS
    'Logical subject-to-venue links that stay generic across CEX, DEX, and operator-style connectors';
