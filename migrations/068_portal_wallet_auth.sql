-- Migration 068: Portal Wallet Auth (SIWE)
-- Adds wallet_address / auth_method columns to users, portal nonce table, and seeds portal tenant.

-- ============================================================================
-- 1. Extend users table with wallet / auth-method columns
-- ============================================================================

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS email          VARCHAR(255),
    ADD COLUMN IF NOT EXISTS wallet_address VARCHAR(42),
    ADD COLUMN IF NOT EXISTS auth_method    VARCHAR(32);

-- Unique index: one wallet per tenant (case-insensitive), only when not null
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_wallet
    ON users(tenant_id, lower(wallet_address))
    WHERE wallet_address IS NOT NULL;

-- ============================================================================
-- 2. Portal auth nonces (single-use, 10-min TTL, address-bound)
-- ============================================================================

CREATE TABLE IF NOT EXISTS portal_auth_nonces (
    id         UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    address    VARCHAR(42)   NOT NULL,
    nonce      VARCHAR(128)  NOT NULL,
    domain     VARCHAR(255),
    issued_at  TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ   NOT NULL,
    used_at    TIMESTAMPTZ
);

-- Fast lookup by nonce (used during verify)
CREATE UNIQUE INDEX IF NOT EXISTS idx_portal_auth_nonces_nonce
    ON portal_auth_nonces(nonce);

-- Cleanup index for expired nonces
CREATE INDEX IF NOT EXISTS idx_portal_auth_nonces_expires
    ON portal_auth_nonces(expires_at);

-- ============================================================================
-- 3. Seed the portal tenant (fixed UUID, all NOT NULL columns filled)
-- ============================================================================
-- Columns of tenants (from 001_initial_schema.sql):
--   id, name, status, api_key_hash, webhook_secret_hash, webhook_url, config,
--   daily_payin_limit_vnd, daily_payout_limit_vnd, created_at, updated_at
INSERT INTO tenants (
    id,
    name,
    status,
    api_key_hash,
    webhook_secret_hash,
    webhook_url,
    config
)
VALUES (
    '11111111-1111-1111-1111-111111111111',
    'RampOS Portal',
    'ACTIVE',
    -- sha256 of a placeholder internal key; never used for real auth
    'portal_internal_noop_key_hash_placeholder_not_for_external_auth',
    'portal_internal_noop_webhook_hash_placeholder',
    NULL,
    '{}'::jsonb
)
ON CONFLICT (id) DO NOTHING;
