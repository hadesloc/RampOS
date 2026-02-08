-- Down migration for 003_rule_versions.sql
-- Drops aml_rule_versions table

DROP INDEX IF EXISTS idx_rule_versions_active;
DROP INDEX IF EXISTS idx_rule_versions_tenant;
DROP TABLE IF EXISTS aml_rule_versions;
