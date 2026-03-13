# Release Hardening Evidence

- Release candidate: `rc-m6-migration-dryrun`
- Generated at: `2026-03-13T04:02:12.745064+00:00`
- Mode: `dry-run`

## Groups
- `migration-rehearsal`: Manual-only migration and rollback rehearsal in an isolated database.

## Results

| Step | Status | Evidence | Log |
|---|---|---|---|
| `sqlx-migrate-run` | `planned` | Forward migration rehearsal. | `docs\operations\evidence\rc-m6-migration-dryrun\sqlx-migrate-run.log` |
| `sqlx-migrate-revert` | `planned` | Rollback rehearsal. | `docs\operations\evidence\rc-m6-migration-dryrun\sqlx-migrate-revert.log` |
| `post-migration-regression` | `planned` | Cheap DB-backed post-migration regression. | `docs\operations\evidence\rc-m6-migration-dryrun\post-migration-regression.log` |
