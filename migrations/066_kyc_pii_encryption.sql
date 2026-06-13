-- KYC PII application-layer encryption marker
--
-- Migration 025 already defines PII columns as VARCHAR/TEXT, which can store the
-- versioned application ciphertext format `enc:v1:<base64(nonce+ciphertext)>`.
-- Keep this migration additive/documentary so existing dev rows remain readable
-- via the application dual-read path and migration 025 remains untouched.

COMMENT ON COLUMN portal_kyc_cases.full_name IS
    'KYC PII. New writes are application-encrypted as enc:v1:<base64(nonce+ciphertext)> when ENCRYPTION_MASTER_KEY is configured; legacy plaintext is dual-read.';
COMMENT ON COLUMN portal_kyc_cases.date_of_birth IS
    'KYC PII. New writes are application-encrypted as enc:v1:<base64(nonce+ciphertext)> when ENCRYPTION_MASTER_KEY is configured; legacy plaintext is dual-read.';
COMMENT ON COLUMN portal_kyc_cases.document_type IS
    'KYC PII. New writes are application-encrypted as enc:v1:<base64(nonce+ciphertext)> when ENCRYPTION_MASTER_KEY is configured; legacy plaintext is dual-read.';
COMMENT ON COLUMN portal_kyc_cases.document_number IS
    'KYC PII. New writes are application-encrypted as enc:v1:<base64(nonce+ciphertext)> when ENCRYPTION_MASTER_KEY is configured; legacy plaintext is dual-read.';
COMMENT ON COLUMN portal_kyc_cases.address IS
    'KYC PII. New writes are application-encrypted as enc:v1:<base64(nonce+ciphertext)> when ENCRYPTION_MASTER_KEY is configured; legacy plaintext is dual-read.';
