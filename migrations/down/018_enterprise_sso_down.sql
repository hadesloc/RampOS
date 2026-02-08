-- Down migration for 018_enterprise_sso.sql
-- Drops SSO tables and related objects

-- Drop RLS policies
DROP POLICY IF EXISTS tenant_isolation_sso_sessions ON sso_sessions;
DROP POLICY IF EXISTS tenant_isolation_idp ON identity_providers;

-- Drop indexes
DROP INDEX IF EXISTS idx_sso_sessions_provider_id;
DROP INDEX IF EXISTS idx_sso_sessions_expires_at;
DROP INDEX IF EXISTS idx_sso_sessions_user_id;
DROP INDEX IF EXISTS idx_identity_providers_tenant_id;

-- Drop tables (sso_sessions references identity_providers)
DROP TABLE IF EXISTS sso_sessions;
DROP TABLE IF EXISTS identity_providers;

-- Drop enum types
DROP TYPE IF EXISTS sso_provider_type;
DROP TYPE IF EXISTS sso_protocol;
