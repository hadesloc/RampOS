-- Down migration for 022_licensing_requirements.sql
-- Drops tenant license status and submissions tables

-- Drop RLS policies
DROP POLICY IF EXISTS license_submissions_update ON license_submissions;
DROP POLICY IF EXISTS license_submissions_insert ON license_submissions;
DROP POLICY IF EXISTS license_submissions_select ON license_submissions;
DROP POLICY IF EXISTS tenant_license_status_update ON tenant_license_status;
DROP POLICY IF EXISTS tenant_license_status_insert ON tenant_license_status;
DROP POLICY IF EXISTS tenant_license_status_select ON tenant_license_status;

-- Drop tables
DROP INDEX IF EXISTS idx_license_submissions_status;
DROP INDEX IF EXISTS idx_license_submissions_requirement;
DROP INDEX IF EXISTS idx_license_submissions_tenant;
DROP TABLE IF EXISTS license_submissions;

DROP INDEX IF EXISTS idx_tenant_license_status_status;
DROP INDEX IF EXISTS idx_tenant_license_status_expiry;
DROP INDEX IF EXISTS idx_tenant_license_status_tenant;
DROP TABLE IF EXISTS tenant_license_status;
