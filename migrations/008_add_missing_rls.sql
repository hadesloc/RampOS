-- ============================================================================
-- SECURITY FIX: Add RLS to tables that were missing tenant isolation
-- Issue: CRITICAL - RLS bypass potential on aml_rule_versions, risk_score_history,
--        case_notes, and compliance_transactions tables
-- ============================================================================

-- Enable RLS on aml_rule_versions
ALTER TABLE aml_rule_versions ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_aml_rule_versions ON aml_rule_versions
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Add tenant_id column to risk_score_history if it doesn't exist
-- First check if column exists and add it
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'risk_score_history'
        AND column_name = 'tenant_id'
    ) THEN
        ALTER TABLE risk_score_history ADD COLUMN tenant_id VARCHAR(64);
        -- Create index for performance
        CREATE INDEX IF NOT EXISTS idx_risk_score_history_tenant
            ON risk_score_history(tenant_id);
    END IF;
END $$;

-- Enable RLS on risk_score_history
ALTER TABLE risk_score_history ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_risk_score_history ON risk_score_history
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Add tenant_id column to case_notes if it doesn't exist
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'case_notes'
        AND column_name = 'tenant_id'
    ) THEN
        ALTER TABLE case_notes ADD COLUMN tenant_id VARCHAR(64);
        -- Create index for performance
        CREATE INDEX IF NOT EXISTS idx_case_notes_tenant
            ON case_notes(tenant_id);
    END IF;
END $$;

-- Enable RLS on case_notes
ALTER TABLE case_notes ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_case_notes ON case_notes
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- Enable RLS on compliance_transactions
ALTER TABLE compliance_transactions ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_compliance_transactions ON compliance_transactions
    USING (tenant_id = current_setting('app.current_tenant')::VARCHAR);

-- ============================================================================
-- Create a system role for background workers that need to bypass RLS
-- This addresses the race condition in list_expired() which needs cross-tenant access
-- ============================================================================

-- Create a system role for background workers
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'rampos_system') THEN
        CREATE ROLE rampos_system WITH BYPASSRLS NOLOGIN;
    END IF;
END $$;

-- Grant the role to the main application user for use in background tasks
-- Note: In production, this should be a separate connection pool
GRANT rampos_system TO rampos;

-- Add comments for documentation
COMMENT ON POLICY tenant_isolation_aml_rule_versions ON aml_rule_versions
    IS 'Enforces tenant isolation for AML rule versions';
COMMENT ON POLICY tenant_isolation_risk_score_history ON risk_score_history
    IS 'Enforces tenant isolation for risk score history';
COMMENT ON POLICY tenant_isolation_case_notes ON case_notes
    IS 'Enforces tenant isolation for case notes';
COMMENT ON POLICY tenant_isolation_compliance_transactions ON compliance_transactions
    IS 'Enforces tenant isolation for compliance transactions';
