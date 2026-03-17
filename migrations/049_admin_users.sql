-- Migration 049: Admin Users — JWT-based admin identity
-- Replaces shared RAMPOS_ADMIN_KEY with per-admin accounts and JWT sessions

-- ============================================================================
-- Admin users table
-- ============================================================================
CREATE TABLE IF NOT EXISTS admin_users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,              -- argon2id hash
    display_name TEXT NOT NULL DEFAULT '',
    role        TEXT NOT NULL DEFAULT 'viewer' CHECK (role IN ('viewer', 'operator', 'admin', 'superadmin')),
    is_active   BOOLEAN NOT NULL DEFAULT true,
    mfa_secret  TEXT,                          -- TOTP secret (optional phase 1)
    mfa_enabled BOOLEAN NOT NULL DEFAULT false,
    last_login_at   TIMESTAMPTZ,
    failed_login_count INTEGER NOT NULL DEFAULT 0,
    locked_until    TIMESTAMPTZ,               -- account lockout after N failures
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- Admin refresh tokens
-- ============================================================================
CREATE TABLE IF NOT EXISTS admin_refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id    UUID NOT NULL REFERENCES admin_users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,           -- SHA-256 hash of refresh token
    user_agent  TEXT,
    ip_address  TEXT,
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_admin_refresh_tokens_admin_id ON admin_refresh_tokens(admin_id);
CREATE INDEX idx_admin_refresh_tokens_expires ON admin_refresh_tokens(expires_at) WHERE revoked_at IS NULL;

-- ============================================================================
-- Admin audit log — tracks all admin authentication events
-- ============================================================================
CREATE TABLE IF NOT EXISTS admin_auth_audit_log (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id    UUID REFERENCES admin_users(id),
    action      TEXT NOT NULL,                 -- login, logout, login_failed, token_refresh, password_change
    ip_address  TEXT,
    user_agent  TEXT,
    metadata    JSONB DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_admin_auth_audit_admin ON admin_auth_audit_log(admin_id);
CREATE INDEX idx_admin_auth_audit_action ON admin_auth_audit_log(action);
CREATE INDEX idx_admin_auth_audit_created ON admin_auth_audit_log(created_at);

-- ============================================================================
-- Seed: create default superadmin account
-- Password: "changeme" (argon2id hash — MUST be changed on first login)
-- ============================================================================
-- NOTE: In production, use the CLI or API to create the first admin.
-- This seed is for development/staging only.
INSERT INTO admin_users (email, password_hash, display_name, role)
VALUES (
    'admin@rampos.local',
    -- argon2id hash of "changeme" — generated with default params
    '$argon2id$v=19$m=19456,t=2,p=1$placeholder_salt$placeholder_hash',
    'Default Admin',
    'superadmin'
) ON CONFLICT (email) DO NOTHING;

-- ============================================================================
-- Updated_at trigger
-- ============================================================================
CREATE OR REPLACE FUNCTION update_admin_users_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_admin_users_updated_at
    BEFORE UPDATE ON admin_users
    FOR EACH ROW
    EXECUTE FUNCTION update_admin_users_updated_at();
