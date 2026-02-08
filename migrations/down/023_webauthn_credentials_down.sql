-- Down migration for 023_webauthn_credentials.sql
-- Drops WebAuthn and portal users tables

DROP INDEX IF EXISTS idx_portal_users_tenant;
DROP INDEX IF EXISTS idx_portal_users_email;
DROP TABLE IF EXISTS portal_users;

DROP INDEX IF EXISTS idx_webauthn_challenges_expires;
DROP INDEX IF EXISTS idx_webauthn_challenges_key;
DROP TABLE IF EXISTS webauthn_challenges;

DROP INDEX IF EXISTS idx_webauthn_credentials_credential_id;
DROP INDEX IF EXISTS idx_webauthn_credentials_tenant_id;
DROP INDEX IF EXISTS idx_webauthn_credentials_user_id;
DROP TABLE IF EXISTS webauthn_credentials;
