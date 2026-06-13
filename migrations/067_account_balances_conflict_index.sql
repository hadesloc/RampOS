-- Fix ON CONFLICT inference for account_balances upserts
--
-- The application performs upserts using:
--   ON CONFLICT (tenant_id, COALESCE(user_id, ''), account_type, currency)
-- in crates/ramp-core/src/repository/ledger.rs (lines 192, 516) and
-- crates/ramp-core/src/service/payin.rs, payout.rs, trade.rs.
--
-- PostgreSQL ON CONFLICT inference requires a unique index whose definition
-- matches the inference target EXACTLY (character-for-character). The plain
-- UNIQUE constraint defined in migration 001 —
--   CONSTRAINT account_balances_unique UNIQUE (tenant_id, user_id, account_type, currency)
-- — uses bare column names and treats NULL user_id values as distinct from each
-- other, so it does NOT satisfy the COALESCE expression. As a result every
-- ledger/payin/payout/trade upsert fails at runtime with:
--   "there is no unique or exclusion constraint matching the ON CONFLICT specification"
-- and NULL user_id system-account rows are never deduplicated.
--
-- Fix:
--   1. Normalise existing NULL system-account user_id to '' so the new
--      expression index has no NULL-vs-'' duplicate key conflicts.
--   2. Drop the plain unique constraint that ON CONFLICT cannot match.
--   3. Create a UNIQUE expression index that matches the inference clause
--      character-for-character.

-- Step 1: Normalise NULL user_id to '' for system-account rows
UPDATE account_balances SET user_id = '' WHERE user_id IS NULL;

-- Step 2: Drop the plain unique constraint (replaced by the expression index below)
ALTER TABLE account_balances DROP CONSTRAINT IF EXISTS account_balances_unique;

-- Step 3: Create a UNIQUE expression index matching the application ON CONFLICT
-- inference exactly — expression must be character-for-character identical to
-- COALESCE(user_id, '') as written in the application SQL.
CREATE UNIQUE INDEX IF NOT EXISTS account_balances_unique_idx
    ON account_balances (tenant_id, COALESCE(user_id, ''), account_type, currency);
