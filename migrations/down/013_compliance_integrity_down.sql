-- Down migration for 013_compliance_integrity.sql
-- Reverts foreign key constraints and NOT NULL enforcement on tenant_id

-- Drop foreign key constraints
ALTER TABLE case_notes DROP CONSTRAINT IF EXISTS case_notes_case_fk;
ALTER TABLE risk_score_history DROP CONSTRAINT IF EXISTS risk_score_history_user_fk;

-- Revert NOT NULL on tenant_id columns (make nullable again)
ALTER TABLE risk_score_history ALTER COLUMN tenant_id DROP NOT NULL;
ALTER TABLE case_notes ALTER COLUMN tenant_id DROP NOT NULL;

-- Note: We do NOT revert the backfilled data - tenant_id values remain
-- as they were populated. This is a non-destructive down migration.
