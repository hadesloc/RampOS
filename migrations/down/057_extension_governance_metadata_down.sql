ALTER TABLE whitelisted_extension_actions
    DROP COLUMN IF EXISTS provenance;

ALTER TABLE whitelisted_extension_actions
    DROP COLUMN IF EXISTS approval_status;
