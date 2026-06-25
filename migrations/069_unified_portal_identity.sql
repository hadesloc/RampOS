-- Migration 069: Unified portal identity
--
-- portal_users is the authenticated human identity and users remains the
-- tenant-scoped financial identity. This migration reconciles the two without
-- deleting legacy rows.

-- ============================================================================
-- 1. Reconcile portal_users with the fields already consumed by the API
-- ============================================================================

ALTER TABLE portal_users
    ALTER COLUMN email DROP NOT NULL,
    ADD COLUMN IF NOT EXISTS financial_user_id VARCHAR(64),
    ADD COLUMN IF NOT EXISTS email_normalized VARCHAR(255),
    ADD COLUMN IF NOT EXISTS password_hash TEXT,
    ADD COLUMN IF NOT EXISTS wallet_address VARCHAR(42),
    ADD COLUMN IF NOT EXISTS auth_methods TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    ADD COLUMN IF NOT EXISTS full_name VARCHAR(200),
    ADD COLUMN IF NOT EXISTS phone VARCHAR(32),
    ADD COLUMN IF NOT EXISTS avatar_url TEXT,
    ADD COLUMN IF NOT EXISTS two_factor_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS last_password_change TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS email_notifications BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS sms_notifications BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS push_notifications BOOLEAN NOT NULL DEFAULT TRUE;

UPDATE portal_users
SET email = NULL
WHERE btrim(COALESCE(email, '')) = '';

UPDATE users
SET email = NULL
WHERE btrim(COALESCE(email, '')) = '';

UPDATE portal_users
SET email_normalized = lower(btrim(email))
WHERE email IS NOT NULL;

-- Fail explicitly instead of silently choosing an owner for legacy duplicate
-- identities.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM portal_users
        WHERE email_normalized IS NOT NULL
        GROUP BY tenant_id, email_normalized
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION
            'cannot unify portal identities: duplicate normalized portal email';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM users
        WHERE email IS NOT NULL
        GROUP BY tenant_id, lower(btrim(email))
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION
            'cannot unify portal identities: duplicate normalized financial email';
    END IF;

    IF EXISTS (
        SELECT 1
        FROM users
        WHERE wallet_address IS NOT NULL
        GROUP BY tenant_id, lower(wallet_address)
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION
            'cannot unify portal identities: duplicate financial wallet';
    END IF;
END $$;

-- Prefer an existing same-ID financial row, then a normalized-email match.
UPDATE portal_users portal
SET financial_user_id = financial.id
FROM users financial
WHERE portal.financial_user_id IS NULL
  AND financial.tenant_id = portal.tenant_id
  AND financial.id = portal.id;

UPDATE portal_users portal
SET financial_user_id = financial.id
FROM users financial
WHERE portal.financial_user_id IS NULL
  AND portal.email_normalized IS NOT NULL
  AND financial.tenant_id = portal.tenant_id
  AND lower(btrim(financial.email)) = portal.email_normalized;

-- Create a financial row for any remaining legacy portal identity.
INSERT INTO users (
    id,
    tenant_id,
    kyc_tier,
    kyc_status,
    status,
    risk_flags,
    created_at,
    updated_at,
    email,
    wallet_address,
    auth_method
)
SELECT
    portal.id,
    portal.tenant_id,
    portal.kyc_tier,
    CASE
        WHEN portal.kyc_status = 'NONE' THEN 'PENDING'
        ELSE portal.kyc_status
    END,
    portal.status,
    '[]'::jsonb,
    portal.created_at,
    portal.updated_at,
    portal.email,
    portal.wallet_address,
    CASE
        WHEN portal.password_hash IS NOT NULL THEN 'password'
        WHEN portal.wallet_address IS NOT NULL THEN 'wallet'
        ELSE NULL
    END
FROM portal_users portal
WHERE portal.financial_user_id IS NULL
ON CONFLICT (tenant_id, id) DO NOTHING;

UPDATE portal_users
SET financial_user_id = id
WHERE financial_user_id IS NULL;

-- Materialize portal identities for wallet/email financial users created by
-- the existing SIWE implementation.
INSERT INTO portal_users (
    id,
    email,
    tenant_id,
    kyc_status,
    kyc_tier,
    status,
    created_at,
    updated_at,
    financial_user_id,
    email_normalized,
    wallet_address,
    auth_methods
)
SELECT
    financial.id,
    financial.email,
    financial.tenant_id,
    financial.kyc_status,
    financial.kyc_tier,
    financial.status,
    financial.created_at,
    financial.updated_at,
    financial.id,
    lower(btrim(financial.email)),
    lower(financial.wallet_address),
    CASE
        WHEN financial.wallet_address IS NOT NULL
            THEN ARRAY['wallet']::TEXT[]
        ELSE ARRAY[]::TEXT[]
    END
FROM users financial
WHERE (financial.wallet_address IS NOT NULL OR financial.email IS NOT NULL)
  AND NOT EXISTS (
      SELECT 1
      FROM portal_users portal
      WHERE portal.tenant_id = financial.tenant_id
        AND portal.financial_user_id = financial.id
  );

-- Pull authoritative financial state and compatibility identifiers into the
-- portal identity.
UPDATE portal_users portal
SET
    kyc_status = financial.kyc_status,
    kyc_tier = financial.kyc_tier,
    status = financial.status,
    email = COALESCE(portal.email, financial.email),
    email_normalized = COALESCE(
        portal.email_normalized,
        lower(btrim(financial.email))
    ),
    wallet_address = COALESCE(
        lower(portal.wallet_address),
        lower(financial.wallet_address)
    ),
    auth_methods = (
        SELECT ARRAY(
            SELECT DISTINCT method
            FROM unnest(
                portal.auth_methods
                || CASE
                    WHEN portal.password_hash IS NOT NULL
                        THEN ARRAY['password']::TEXT[]
                    ELSE ARRAY[]::TEXT[]
                END
                || CASE
                    WHEN portal.wallet_address IS NOT NULL
                         OR financial.wallet_address IS NOT NULL
                        THEN ARRAY['wallet']::TEXT[]
                    ELSE ARRAY[]::TEXT[]
                END
            ) AS method
            ORDER BY method
        )
    )
FROM users financial
WHERE financial.tenant_id = portal.tenant_id
  AND financial.id = portal.financial_user_id;

ALTER TABLE portal_users
    ALTER COLUMN financial_user_id SET NOT NULL;

ALTER TABLE portal_users
    ADD CONSTRAINT portal_users_financial_user_fk
        FOREIGN KEY (tenant_id, financial_user_id)
        REFERENCES users(tenant_id, id)
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    ADD CONSTRAINT portal_users_auth_methods_check
        CHECK (auth_methods <@ ARRAY['password', 'wallet']::TEXT[]);

CREATE UNIQUE INDEX idx_portal_users_normalized_email
    ON portal_users(tenant_id, email_normalized)
    WHERE email_normalized IS NOT NULL;

CREATE UNIQUE INDEX idx_portal_users_wallet
    ON portal_users(tenant_id, lower(wallet_address))
    WHERE wallet_address IS NOT NULL;

CREATE UNIQUE INDEX idx_portal_users_financial_user
    ON portal_users(tenant_id, financial_user_id);

-- ============================================================================
-- 2. Add refresh-token rotation and replay metadata
-- ============================================================================

ALTER TABLE refresh_tokens
    ADD COLUMN IF NOT EXISTS tenant_id VARCHAR(64),
    ADD COLUMN IF NOT EXISTS rotated_to_id UUID,
    ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS revoke_reason VARCHAR(64),
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

UPDATE refresh_tokens token
SET tenant_id = portal.tenant_id
FROM portal_users portal
WHERE token.tenant_id IS NULL
  AND token.user_id = portal.id;

UPDATE refresh_tokens token
SET tenant_id = financial.tenant_id
FROM users financial
WHERE token.tenant_id IS NULL
  AND token.user_id = financial.id;

UPDATE refresh_tokens
SET tenant_id = '11111111-1111-1111-1111-111111111111'
WHERE tenant_id IS NULL;

UPDATE refresh_tokens
SET
    revoked_at = COALESCE(revoked_at, created_at),
    revoke_reason = COALESCE(revoke_reason, 'legacy_revoked')
WHERE revoked = TRUE;

ALTER TABLE refresh_tokens
    ALTER COLUMN tenant_id SET NOT NULL,
    ADD CONSTRAINT refresh_tokens_rotated_to_fk
        FOREIGN KEY (rotated_to_id)
        REFERENCES refresh_tokens(id)
        ON DELETE SET NULL;

CREATE INDEX idx_refresh_tokens_active_family
    ON refresh_tokens(tenant_id, user_id, family_id)
    WHERE revoked = FALSE;
