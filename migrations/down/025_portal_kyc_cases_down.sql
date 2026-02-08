-- Down migration for 025_portal_kyc_cases.sql
-- Drops portal KYC document and case tables

DROP TRIGGER IF EXISTS trigger_portal_kyc_cases_updated_at ON portal_kyc_cases;

DROP INDEX IF EXISTS idx_portal_kyc_docs_user;
DROP INDEX IF EXISTS idx_portal_kyc_docs_case;
DROP TABLE IF EXISTS portal_kyc_documents;

DROP INDEX IF EXISTS idx_portal_kyc_status;
DROP INDEX IF EXISTS idx_portal_kyc_tenant;
DROP INDEX IF EXISTS idx_portal_kyc_user;
DROP TABLE IF EXISTS portal_kyc_cases;
