-- Down migration for 067_account_balances_conflict_index.sql
--
-- Restores the original plain UNIQUE constraint and removes the expression
-- index. Note: the NULL -> '' normalisation applied to user_id in the forward
-- migration is intentionally NOT reversed; '' is the canonical system-account
-- key and reversing it would require distinguishing originally-NULL rows from
-- legitimately-empty-string rows, which is not safe to do automatically.

DROP INDEX IF EXISTS account_balances_unique_idx;

ALTER TABLE account_balances
    ADD CONSTRAINT account_balances_unique UNIQUE (tenant_id, user_id, account_type, currency);
