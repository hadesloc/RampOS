-- Down migration for 20240924120000_create_compliance_cases.sql
-- Drops compliance_cases and case_notes tables

DROP INDEX IF EXISTS idx_case_notes_case_id;
DROP TABLE IF EXISTS case_notes;

DROP INDEX IF EXISTS idx_compliance_cases_tenant_id;
DROP INDEX IF EXISTS idx_compliance_cases_status;
DROP INDEX IF EXISTS idx_compliance_cases_user_id;
DROP TABLE IF EXISTS compliance_cases;
