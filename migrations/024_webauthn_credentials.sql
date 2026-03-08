-- WebAuthn Credentials table
-- Stores registered WebAuthn/Passkey credentials for portal users

CREATE TABLE IF NOT EXISTS webauthn_credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id VARCHAR(64) NOT NULL,
    tenant_id VARCHAR(64) NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',

    -- Credential data (serialized Passkey from webauthn-rs)
    credential_id BYTEA NOT NULL,
    credential_json JSONB NOT NULL,

    -- Metadata
    name VARCHAR(255),
    aaguid UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    sign_count BIGINT NOT NULL DEFAULT 0,

    CONSTRAINT uq_webauthn_credential_id UNIQUE (credential_id)
);

CREATE INDEX idx_webauthn_credentials_user_id ON webauthn_credentials(user_id);
CREATE INDEX idx_webauthn_credentials_tenant_id ON webauthn_credentials(tenant_id);
CREATE INDEX idx_webauthn_credentials_credential_id ON webauthn_credentials(credential_id);

-- Challenge store for in-progress registrations/authentications
-- These are ephemeral and should be cleaned up periodically
CREATE TABLE IF NOT EXISTS webauthn_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_key VARCHAR(255) NOT NULL,
    challenge_type VARCHAR(32) NOT NULL, -- 'registration' or 'authentication'
    state_json JSONB NOT NULL,
    email VARCHAR(255),
    user_id VARCHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,

    CONSTRAINT uq_webauthn_challenge_key UNIQUE (challenge_key)
);

CREATE INDEX idx_webauthn_challenges_key ON webauthn_challenges(challenge_key);
CREATE INDEX idx_webauthn_challenges_expires ON webauthn_challenges(expires_at);

-- Portal users table (if not exists) for WebAuthn-based authentication
-- This stores users who authenticate via the portal (separate from tenant-scoped users)
CREATE TABLE IF NOT EXISTS portal_users (
    id VARCHAR(64) PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    tenant_id VARCHAR(64) NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',
    kyc_status VARCHAR(32) NOT NULL DEFAULT 'NONE',
    kyc_tier SMALLINT NOT NULL DEFAULT 0,
    status VARCHAR(32) NOT NULL DEFAULT 'ACTIVE',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_portal_users_email_tenant UNIQUE (email, tenant_id)
);

CREATE INDEX idx_portal_users_email ON portal_users(email);
CREATE INDEX idx_portal_users_tenant ON portal_users(tenant_id);
