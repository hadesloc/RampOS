-- Down migration for 015_license_management.sql
-- Drops license management tables and seed data

-- Drop triggers
DROP TRIGGER IF EXISTS trigger_tenant_license_documents_updated_at ON tenant_license_documents;
DROP TRIGGER IF EXISTS trigger_tenant_licenses_updated_at ON tenant_licenses;
DROP TRIGGER IF EXISTS trigger_license_requirements_updated_at ON license_requirements;
DROP TRIGGER IF EXISTS trigger_license_types_updated_at ON license_types;

-- Drop RLS policies
DROP POLICY IF EXISTS tenant_license_documents_isolation ON tenant_license_documents;
DROP POLICY IF EXISTS tenant_licenses_isolation ON tenant_licenses;

-- Drop indexes
DROP INDEX IF EXISTS idx_tenant_license_documents_status;
DROP INDEX IF EXISTS idx_tenant_license_documents_requirement;
DROP INDEX IF EXISTS idx_tenant_license_documents_license;
DROP INDEX IF EXISTS idx_tenant_license_documents_tenant;
DROP INDEX IF EXISTS idx_tenant_licenses_expires;
DROP INDEX IF EXISTS idx_tenant_licenses_status;
DROP INDEX IF EXISTS idx_tenant_licenses_type;
DROP INDEX IF EXISTS idx_tenant_licenses_tenant;
DROP INDEX IF EXISTS idx_license_requirements_mandatory;
DROP INDEX IF EXISTS idx_license_requirements_type;
DROP INDEX IF EXISTS idx_license_types_jurisdiction;
DROP INDEX IF EXISTS idx_license_types_code;

-- Drop tables (in dependency order)
DROP TABLE IF EXISTS tenant_license_documents;
DROP TABLE IF EXISTS tenant_licenses;
DROP TABLE IF EXISTS license_requirements;
DROP TABLE IF EXISTS license_types;
