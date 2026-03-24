-- Migration 063: Source of funds packages
-- Evidence packages that can attach to either funding or cash-out review subjects.

CREATE TABLE IF NOT EXISTS source_of_funds_packages (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    wallet_attestation_id UUID NULL REFERENCES wallet_attestations(id) ON DELETE SET NULL,
    venue_account_id TEXT NULL,
    venue_transfer_id TEXT NULL,
    review_status TEXT NOT NULL DEFAULT 'pending',
    package_uri TEXT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    reviewed_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT source_of_funds_packages_review_status_check CHECK (
        review_status IN ('pending', 'in_review', 'approved', 'rejected', 'expired')
    ),
    CONSTRAINT source_of_funds_packages_metadata_object CHECK (
        jsonb_typeof(metadata) = 'object'
    ),
    CONSTRAINT source_of_funds_packages_account_fk
        FOREIGN KEY (venue_account_id, tenant_id)
        REFERENCES venue_accounts(id, tenant_id)
        ON DELETE SET NULL,
    CONSTRAINT source_of_funds_packages_transfer_fk
        FOREIGN KEY (venue_transfer_id, tenant_id)
        REFERENCES venue_transfers(id, tenant_id)
        ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_source_of_funds_packages_tenant_subject
    ON source_of_funds_packages (tenant_id, subject_type, subject_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_source_of_funds_packages_review
    ON source_of_funds_packages (tenant_id, review_status, created_at DESC);

CREATE TRIGGER trigger_source_of_funds_packages_updated_at
    BEFORE UPDATE ON source_of_funds_packages
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

ALTER TABLE source_of_funds_packages ENABLE ROW LEVEL SECURITY;

CREATE POLICY source_of_funds_packages_tenant_isolation ON source_of_funds_packages
    USING (tenant_id = current_setting('app.current_tenant', true))
    WITH CHECK (tenant_id = current_setting('app.current_tenant', true));

COMMENT ON TABLE source_of_funds_packages IS
    'Review packages for proving source-of-funds across venue funding and venue-linked cash-out flows';
