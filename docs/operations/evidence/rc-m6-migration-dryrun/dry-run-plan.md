# Planned Release Hardening Run

## migration-rehearsal
- Purpose: Manual-only migration and rollback rehearsal in an isolated database.
- `sqlx-migrate-run` -> `sqlx migrate run`
- `sqlx-migrate-revert` -> `sqlx migrate revert`
- `post-migration-regression` -> `cargo test -p ramp-api --test partner_registry_test -- --nocapture`

