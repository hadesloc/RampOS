-- Migration 061: Beneficiary profiles
-- Generic trust records for payout and funding destinations with cooldown state.

CREATE TABLE IF NOT EXISTS beneficiary_profiles (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    subject_type TEXT NOT NULL DEFAULT 'user',
    subject_id TEXT NOT NULL,
    user_id TEXT NULL,
    destination_type TEXT NOT NULL,
    destination_ref TEXT NOT NULL,
    display_name TEXT NULL,
    asset_symbol TEXT NULL,
    network TEXT NULL,
    verification_status TEXT NOT NULL DEFAULT 'pending',
    cooldown_ends_at TIMESTAMPTZ NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_verified_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT beneficiary_profiles_subject_check CHECK (
        (subject_type = 'user' AND user_id IS NOT NULL AND subject_id = user_id)
        OR (subject_type <> 'user' AND user_id IS NULL)
    ),
    CONSTRAINT beneficiary_profiles_verification_status_check CHECK (
        verification_status IN ('pending', 'verified', 'rejected', 'suspended', 'cooldown')
    ),
    CONSTRAINT beneficiary_profiles_metadata_object CHECK (
        jsonb_typeof(metadata) = 'object'
    ),
    CONSTRAINT beneficiary_profiles_user_fk
        FOREIGN KEY (tenant_id, user_id) REFERENCES users(tenant_id, id) ON DELETE CASCADE,
    CONSTRAINT beneficiary_profiles_unique_destination UNIQUE (
        tenant_id,
        subject_type,
        subject_id,
        destination_type,
        destination_ref
    ),
    CONSTRAINT beneficiary_profiles_id_tenant_unique UNIQUE (id, tenant_id)
);

CREATE INDEX IF NOT EXISTS idx_beneficiary_profiles_tenant_subject
    ON beneficiary_profiles (tenant_id, subject_type, subject_id, verification_status);

CREATE INDEX IF NOT EXISTS idx_beneficiary_profiles_destination
    ON beneficiary_profiles (tenant_id, destination_type, destination_ref);

CREATE TRIGGER trigger_beneficiary_profiles_updated_at
    BEFORE UPDATE ON beneficiary_profiles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

ALTER TABLE beneficiary_profiles ENABLE ROW LEVEL SECURITY;

CREATE POLICY beneficiary_profiles_tenant_isolation ON beneficiary_profiles
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));

COMMENT ON TABLE beneficiary_profiles IS
    'Trusted destinations for payout and funding flows, including cooldown and verification state';
