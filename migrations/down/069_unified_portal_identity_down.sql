-- Down migration for 069_unified_portal_identity.sql.
--
-- Reconciled portal and financial rows are intentionally preserved.

DROP INDEX IF EXISTS idx_refresh_tokens_active_family;

ALTER TABLE refresh_tokens
    DROP CONSTRAINT IF EXISTS refresh_tokens_rotated_to_fk,
    DROP COLUMN IF EXISTS updated_at,
    DROP COLUMN IF EXISTS revoke_reason,
    DROP COLUMN IF EXISTS revoked_at,
    DROP COLUMN IF EXISTS rotated_to_id,
    DROP COLUMN IF EXISTS tenant_id;

DROP INDEX IF EXISTS idx_portal_users_financial_user;
DROP INDEX IF EXISTS idx_portal_users_wallet;
DROP INDEX IF EXISTS idx_portal_users_normalized_email;

ALTER TABLE portal_users
    DROP CONSTRAINT IF EXISTS portal_users_auth_methods_check,
    DROP CONSTRAINT IF EXISTS portal_users_financial_user_fk,
    DROP COLUMN IF EXISTS push_notifications,
    DROP COLUMN IF EXISTS sms_notifications,
    DROP COLUMN IF EXISTS email_notifications,
    DROP COLUMN IF EXISTS last_password_change,
    DROP COLUMN IF EXISTS two_factor_enabled,
    DROP COLUMN IF EXISTS avatar_url,
    DROP COLUMN IF EXISTS phone,
    DROP COLUMN IF EXISTS full_name,
    DROP COLUMN IF EXISTS auth_methods,
    DROP COLUMN IF EXISTS wallet_address,
    DROP COLUMN IF EXISTS password_hash,
    DROP COLUMN IF EXISTS email_normalized,
    DROP COLUMN IF EXISTS financial_user_id;
