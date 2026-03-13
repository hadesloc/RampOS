# Release Hardening Evidence

- Release candidate: `rc-m6-migration-live-4`
- Generated at: `2026-03-13T05:04:46.801374+00:00`
- Mode: `execute`

## Groups
- `migration-rehearsal`: Manual-only migration and rollback rehearsal in an isolated database.

## Results

| Step | Status | Evidence | Log |
|---|---|---|---|
| `sqlx-migrate-run` | `passed` | Forward migration rehearsal. | `docs\operations\evidence\rc-m6-migration-live-4\sqlx-migrate-run.log` |
| `sqlx-migrate-revert` | `passed` | Rollback rehearsal. | `docs\operations\evidence\rc-m6-migration-live-4\sqlx-migrate-revert.log` |
| `post-migration-regression` | `passed` | Cheap DB-backed post-migration regression. | `docs\operations\evidence\rc-m6-migration-live-4\post-migration-regression.log` |
