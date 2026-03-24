DROP TRIGGER IF EXISTS trg_wallet_attestations_updated_at ON wallet_attestations;
DROP FUNCTION IF EXISTS update_wallet_attestations_updated_at();
DROP INDEX IF EXISTS idx_wallet_attestations_status;
DROP INDEX IF EXISTS idx_wallet_attestations_wallet_chain;
DROP INDEX IF EXISTS idx_wallet_attestations_tenant_user;
DROP TABLE IF EXISTS wallet_attestations;
