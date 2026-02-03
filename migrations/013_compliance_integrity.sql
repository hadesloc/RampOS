-- ============================================================================
-- Compliance integrity hardening: tenant_id backfill + constraints
-- ============================================================================

-- Backfill tenant_id for risk_score_history from intents (most reliable)
UPDATE risk_score_history r
SET tenant_id = i.tenant_id
FROM intents i
WHERE r.intent_id = i.id AND r.tenant_id IS NULL;

-- Backfill tenant_id from users where user_id is unique across tenants
WITH single_users AS (
    SELECT id FROM users GROUP BY id HAVING COUNT(*) = 1
)
UPDATE risk_score_history r
SET tenant_id = u.tenant_id
FROM users u
JOIN single_users s ON s.id = u.id
WHERE r.user_id = u.id AND r.tenant_id IS NULL;

-- Backfill tenant_id for case_notes from aml_cases
UPDATE case_notes n
SET tenant_id = c.tenant_id
FROM aml_cases c
WHERE n.case_id = c.id AND n.tenant_id IS NULL;

-- Add foreign key: case_notes.case_id -> aml_cases.id
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'case_notes_case_fk'
        AND table_name = 'case_notes'
    ) THEN
        ALTER TABLE case_notes
            ADD CONSTRAINT case_notes_case_fk
            FOREIGN KEY (case_id) REFERENCES aml_cases(id);
    END IF;
END $$;

-- Add foreign key: risk_score_history (tenant_id, user_id) -> users (tenant_id, id)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE constraint_name = 'risk_score_history_user_fk'
        AND table_name = 'risk_score_history'
    ) THEN
        ALTER TABLE risk_score_history
            ADD CONSTRAINT risk_score_history_user_fk
            FOREIGN KEY (tenant_id, user_id) REFERENCES users(tenant_id, id);
    END IF;
END $$;

-- Enforce NOT NULL on tenant_id when fully backfilled
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM risk_score_history WHERE tenant_id IS NULL) THEN
        ALTER TABLE risk_score_history
            ALTER COLUMN tenant_id SET NOT NULL;
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM case_notes WHERE tenant_id IS NULL) THEN
        ALTER TABLE case_notes
            ALTER COLUMN tenant_id SET NOT NULL;
    END IF;
END $$;
