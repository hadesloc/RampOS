-- Down migration for 020_compliance_audit_trail.sql
-- Drops compliance audit log table and related objects

-- Revoke/re-grant permissions (reverse of up migration)
-- Note: ramp_app role may not exist in all environments
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ramp_app') THEN
        REVOKE ALL ON compliance_audit_log FROM ramp_app;
    END IF;
END $$;

-- Drop RLS policies
DROP POLICY IF EXISTS compliance_audit_insert_only ON compliance_audit_log;
DROP POLICY IF EXISTS compliance_audit_tenant_isolation ON compliance_audit_log;

-- Drop triggers
DROP TRIGGER IF EXISTS trigger_prevent_audit_delete ON compliance_audit_log;
DROP TRIGGER IF EXISTS trigger_prevent_audit_update ON compliance_audit_log;

-- Drop function
DROP FUNCTION IF EXISTS prevent_audit_modification();

-- Drop indexes
DROP INDEX IF EXISTS idx_compliance_audit_sequence;
DROP INDEX IF EXISTS idx_compliance_audit_resource;
DROP INDEX IF EXISTS idx_compliance_audit_actor;
DROP INDEX IF EXISTS idx_compliance_audit_event_type;
DROP INDEX IF EXISTS idx_compliance_audit_tenant;

-- Drop table
DROP TABLE IF EXISTS compliance_audit_log;

-- Drop enum type
DROP TYPE IF EXISTS compliance_event_type;
