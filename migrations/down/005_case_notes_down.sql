-- Down migration for 005_case_notes.sql
-- Drops case_notes table
-- NOTE: case_notes is also created in ramp-compliance migration,
-- so this only drops if it exists from this migration context

DROP TABLE IF EXISTS case_notes;
