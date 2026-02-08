-- Down migration for 008_add_missing_rls.sql
-- Removes RLS policies and columns added in 008

-- Drop comments
COMMENT ON POLICY tenant_isolation_compliance_transactions ON compliance_transactions IS NULL;
COMMENT ON POLICY tenant_isolation_case_notes ON case_notes IS NULL;
COMMENT ON POLICY tenant_isolation_risk_score_history ON risk_score_history IS NULL;
COMMENT ON POLICY tenant_isolation_aml_rule_versions ON aml_rule_versions IS NULL;

-- Drop RLS policies
DROP POLICY IF EXISTS tenant_isolation_compliance_transactions ON compliance_transactions;
DROP POLICY IF EXISTS tenant_isolation_case_notes ON case_notes;
DROP POLICY IF EXISTS tenant_isolation_risk_score_history ON risk_score_history;
DROP POLICY IF EXISTS tenant_isolation_aml_rule_versions ON aml_rule_versions;

-- Disable RLS on these tables
ALTER TABLE compliance_transactions DISABLE ROW LEVEL SECURITY;
ALTER TABLE case_notes DISABLE ROW LEVEL SECURITY;
ALTER TABLE risk_score_history DISABLE ROW LEVEL SECURITY;
ALTER TABLE aml_rule_versions DISABLE ROW LEVEL SECURITY;

-- Drop tenant_id columns that were added
DROP INDEX IF EXISTS idx_case_notes_tenant;
ALTER TABLE case_notes DROP COLUMN IF EXISTS tenant_id;

DROP INDEX IF EXISTS idx_risk_score_history_tenant;
ALTER TABLE risk_score_history DROP COLUMN IF EXISTS tenant_id;

-- Revoke system role
REVOKE rampos_system FROM rampos;
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'rampos_system') THEN
        DROP ROLE rampos_system;
    END IF;
END $$;
