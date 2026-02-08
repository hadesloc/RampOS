-- Portal KYC Cases table
-- Stores KYC submissions from portal users (separate from tenant-scoped kyc_records)

CREATE TABLE IF NOT EXISTS portal_kyc_cases (
    id VARCHAR(64) PRIMARY KEY DEFAULT gen_random_uuid()::text,
    user_id VARCHAR(64) NOT NULL REFERENCES portal_users(id),
    tenant_id VARCHAR(64) NOT NULL DEFAULT '00000000-0000-0000-0000-000000000001',

    -- Status and tier
    status VARCHAR(32) NOT NULL DEFAULT 'PENDING',  -- PENDING, APPROVED, REJECTED
    tier SMALLINT NOT NULL DEFAULT 1,                -- 0=none, 1=basic, 2=enhanced, 3=full

    -- Personal info
    full_name VARCHAR(200) NOT NULL,
    date_of_birth VARCHAR(10) NOT NULL,              -- YYYY-MM-DD
    document_type VARCHAR(50) NOT NULL,              -- PASSPORT, DRIVERS_LICENSE, NATIONAL_ID
    document_number VARCHAR(50),
    address TEXT NOT NULL,

    -- Review
    reviewer_notes TEXT,
    reviewed_at TIMESTAMPTZ,

    -- Timestamps
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT portal_kyc_status_check CHECK (status IN ('PENDING', 'APPROVED', 'REJECTED')),
    CONSTRAINT portal_kyc_tier_check CHECK (tier BETWEEN 0 AND 3)
);

CREATE INDEX idx_portal_kyc_user ON portal_kyc_cases(user_id);
CREATE INDEX idx_portal_kyc_tenant ON portal_kyc_cases(tenant_id);
CREATE INDEX idx_portal_kyc_status ON portal_kyc_cases(status);

-- Portal KYC Documents table
-- Stores document file references for KYC submissions

CREATE TABLE IF NOT EXISTS portal_kyc_documents (
    id VARCHAR(64) PRIMARY KEY DEFAULT gen_random_uuid()::text,
    case_id VARCHAR(64) NOT NULL REFERENCES portal_kyc_cases(id) ON DELETE CASCADE,
    user_id VARCHAR(64) NOT NULL REFERENCES portal_users(id),

    -- File info
    document_type VARCHAR(32) NOT NULL,  -- front, back, selfie
    filename VARCHAR(255) NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    file_size BIGINT NOT NULL,
    file_path TEXT NOT NULL,             -- local path (S3 path in production)

    -- Timestamps
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT portal_kyc_doc_type_check CHECK (document_type IN ('front', 'back', 'selfie'))
);

CREATE INDEX idx_portal_kyc_docs_case ON portal_kyc_documents(case_id);
CREATE INDEX idx_portal_kyc_docs_user ON portal_kyc_documents(user_id);

-- Trigger to auto-update updated_at
CREATE TRIGGER trigger_portal_kyc_cases_updated_at
    BEFORE UPDATE ON portal_kyc_cases
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
