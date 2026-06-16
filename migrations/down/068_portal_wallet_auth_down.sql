-- Down migration 068: Portal Wallet Auth (SIWE)
-- Reverses migration 068.

DROP INDEX IF EXISTS idx_portal_auth_nonces_expires;
DROP INDEX IF EXISTS idx_portal_auth_nonces_nonce;
DROP TABLE IF EXISTS portal_auth_nonces;

DROP INDEX IF EXISTS idx_users_wallet;

ALTER TABLE users
    DROP COLUMN IF EXISTS auth_method,
    DROP COLUMN IF EXISTS wallet_address,
    DROP COLUMN IF EXISTS email;

-- Note: the portal tenant seed row is intentionally left in place during rollback
-- to avoid breaking FK references from users that may have been created during this migration.
