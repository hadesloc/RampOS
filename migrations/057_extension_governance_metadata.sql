ALTER TABLE whitelisted_extension_actions
    ADD COLUMN IF NOT EXISTS approval_status TEXT NOT NULL DEFAULT 'approved';

ALTER TABLE whitelisted_extension_actions
    ADD COLUMN IF NOT EXISTS provenance JSONB NOT NULL DEFAULT '{}'::jsonb;

UPDATE whitelisted_extension_actions
SET approval_status = COALESCE(NULLIF(approval_status, ''), 'approved');

UPDATE whitelisted_extension_actions
SET provenance = CASE
    WHEN provenance = '{}'::jsonb THEN jsonb_build_object(
        'mode', 'registry',
        'sourceClass', 'persisted_registry'
    )
    ELSE provenance
END;
