-- Migration 070: Bind SIWE nonces to a purpose and optional portal identity.

ALTER TABLE portal_auth_nonces
    ADD COLUMN IF NOT EXISTS purpose VARCHAR(16) NOT NULL DEFAULT 'login',
    ADD COLUMN IF NOT EXISTS portal_user_id VARCHAR(64);

ALTER TABLE portal_auth_nonces
    ADD CONSTRAINT portal_auth_nonces_purpose_check
        CHECK (purpose IN ('login', 'link')),
    ADD CONSTRAINT portal_auth_nonces_portal_user_fk
        FOREIGN KEY (portal_user_id)
        REFERENCES portal_users(id)
        ON DELETE CASCADE;

CREATE INDEX idx_portal_auth_nonces_portal_user
    ON portal_auth_nonces(portal_user_id, purpose, expires_at)
    WHERE portal_user_id IS NOT NULL;

