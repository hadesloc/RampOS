# Database Security Audit Handoff

## Task ID: audit_db_task

## Status: COMPLETED

## Summary

Comprehensive security audit of RampOS database schema and SQL queries completed. Examined 8 migration files and 10+ repository/store modules. Identified 22 security findings across all severity levels.

## Key Findings

### Critical Issues (3)
1. **DB-002**: RLS policies fail open if `app.current_tenant` session variable is unset
2. **DB-003**: Missing RLS on 4 tables: `aml_rule_versions`, `risk_score_history`, `case_notes`, `compliance_transactions`
3. **DB-004**: Missing `tenant_id` column in `risk_score_history` and `case_notes` tables

### High Issues (6)
1. **DB-005**: System worker queries bypass tenant isolation (`list_expired`, `get_pending_events`)
2. **DB-006**: `get_case()` in postgres.rs missing tenant validation - cross-tenant access possible
3. **DB-007**: `get_version()` missing tenant validation
4. **DB-008**: `get_notes()` missing tenant validation
5. **DB-012**: KYC `verification_data` contains PII but not encrypted at rest
6. **DB-021**: Weak/predictable secrets in seed data

### Positive Findings
- All SQL queries use prepared statements via sqlx (no SQL injection)
- RLS enabled on 11 core tables with proper policies
- RLS context properly scoped to transactions via `set_config(..., true)`
- API keys and webhook secrets are hashed with bcrypt
- Rails adapter configs are stored encrypted as BYTEA

## Artifacts

Primary audit report:
```
.claude/artifacts/security-audit-database.md
```

## Files Analyzed

### Migrations (8 files)
- `migrations/001_initial_schema.sql` - Core schema with 480 lines
- `migrations/002_seed_data.sql` - Test data
- `migrations/003_rule_versions.sql` - AML rule versioning
- `migrations/004_score_history.sql` - Risk score history (MISSING tenant_id)
- `migrations/005_case_notes.sql` - Case notes (MISSING tenant_id)
- `migrations/006_enable_rls.sql` - RLS policies
- `migrations/007_compliance_transactions.sql` - Compliance txs (MISSING RLS)
- `migrations/999_seed_data.sql` - Extended seed data

### Repository Modules
- `crates/ramp-core/src/repository/*.rs` (7 files)
- `crates/ramp-compliance/src/store/postgres.rs`
- `crates/ramp-compliance/src/transaction_history.rs`
- `crates/ramp-compliance/src/rules/version.rs`

## Recommended Next Steps

### Immediate (Critical)
1. Add RLS policies to missing tables via new migration
2. Add `tenant_id` columns to `risk_score_history` and `case_notes`
3. Update RLS policies to fail closed: use `COALESCE(NULLIF(current_setting('app.current_tenant', true), ''), 'INVALID')`
4. Add `tenant_id` parameter to `get_case()`, `get_version()`, `get_notes()` methods

### Short-term (High)
5. Create database role separation (ramp_app, ramp_readonly, ramp_system)
6. Encrypt KYC verification_data at column level
7. Document and secure system worker queries
8. Add CHECK constraints for transaction limits

### Medium-term
9. Add state validation constraints to intents table
10. Implement JSONB schema validation
11. Add audit log PII sanitization

---

**Completed by:** Worker Agent (Database Security Auditor)
**Date:** 2026-02-02
**Duration:** Complete audit of schema and SQL layer
