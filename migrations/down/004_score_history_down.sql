-- Down migration for 004_score_history.sql
-- Drops risk_score_history table

DROP INDEX IF EXISTS idx_score_history_user;
DROP TABLE IF EXISTS risk_score_history;
