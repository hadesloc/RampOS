DROP INDEX IF EXISTS idx_portal_auth_nonces_portal_user;

ALTER TABLE portal_auth_nonces
    DROP CONSTRAINT IF EXISTS portal_auth_nonces_portal_user_fk,
    DROP CONSTRAINT IF EXISTS portal_auth_nonces_purpose_check,
    DROP COLUMN IF EXISTS portal_user_id,
    DROP COLUMN IF EXISTS purpose;
