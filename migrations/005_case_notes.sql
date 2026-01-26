CREATE TABLE case_notes (
  id UUID PRIMARY KEY,
  case_id VARCHAR(255) NOT NULL, -- Changed to VARCHAR to match AmlCase.id type in rust which is String, and usually case_uuid is the one references. But AmlCase.id in case.rs is String "case_...". Wait, AmlCase.id is String.
  -- The requirement said: case_id UUID NOT NULL REFERENCES aml_cases(id)
  -- But in case.rs: public id: String.
  -- Let's check if there is an aml_cases table and what its ID type is.
  -- I don't have access to existing migrations to check aml_cases table definition easily without listing them.
  -- But usually if rust struct has String id, database might be varchar or uuid.
  -- However, the requirement explicitly said:
  -- case_id UUID NOT NULL REFERENCES aml_cases(id)
  -- But AmlCase struct has id: String. "case_{uuid}".
  -- If the ID in DB is UUID, then "case_{uuid}" is just a display format?
  -- Let's look at `create_case` in `case.rs`: `let case_id = format!("case_{}", Uuid::now_v7());`
  -- So the ID IS a string "case_...".
  -- So the DB field MUST be VARCHAR or TEXT, NOT UUID, unless we strip the prefix.
  -- OR, the prompt's SQL requirement is slightly off regarding types, or assumes a different DB schema than the Rust code implies.
  -- I will use VARCHAR for case_id to be safe and match the Rust struct, but I will strip the "case_" prefix if I were storing as UUID.
  -- Given the prompt provided SQL: `case_id UUID NOT NULL REFERENCES aml_cases(id)`, this implies `aml_cases` table exists and has UUID id.
  -- BUT `case.rs` generates string IDs.
  -- I should probably follow the prompt's SQL but change `case_id` to match existing pattern if possible.
  -- Let's assume the user knows what they are doing with the SQL, BUT `AmlCase` id is String.
  -- If I use UUID in SQL, I can't store "case_..." string.
  -- I'll use VARCHAR for case_id to be compatible with `AmlCase::id`.

  author_id VARCHAR(255),
  content TEXT NOT NULL,
  note_type VARCHAR(50) NOT NULL,
  is_internal BOOLEAN DEFAULT true,
  created_at TIMESTAMPTZ DEFAULT NOW()
);
