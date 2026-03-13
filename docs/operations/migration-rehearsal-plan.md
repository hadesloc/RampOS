# Migration Rehearsal Plan

This plan covers the additive release-candidate schema bundle introduced by the control-plane expansion:

- `042_config_bundle_governance.sql`
- `043_partner_registry.sql`
- `044_corridor_packs.sql`
- `045_payment_method_capabilities.sql`
- `046_provider_routing.sql`
- `047_kyb_evidence_packages.sql`
- `048_treasury_evidence_imports.sql`

Rollback counterparts are expected under `migrations/down/`.

## Purpose

Release candidates cannot skip database proof.
This plan defines the minimum forward, rollback, and seed-validation evidence required before staging or production promotion.

## Preconditions

- release candidate SHA is frozen
- DB engine and version are identified
- restore checkpoint exists before forward rehearsal
- operator knows whether rollback is app-only, schema-only, or full restore
- seed fixture source is identified

## Static Precheck

Run:

```powershell
python scripts/release_hardening.py --group migration-rehearsal --dry-run --include-manual --release-candidate <sha>
```

Expected result:

- manual rehearsal plan is written into the release evidence directory
- `sqlx`, `DATABASE_URL`, and rollback ownership requirements are made explicit before execution
- no destructive DB step runs unless `--include-manual` is acknowledged by the operator

## Forward Rehearsal

Minimum sequence:

1. Restore a clean rehearsal database or snapshot.
2. Record pre-migration schema version and timestamp.
3. Apply the release-candidate migration set.
4. Record success or failure and elapsed time.
5. Run targeted smoke checks for admin and export surfaces.

Suggested command pattern:

```powershell
$env:DATABASE_URL = "<isolated-rehearsal-db>"
python scripts/release_hardening.py --group migration-rehearsal --include-manual --release-candidate <sha> --stop-on-failure
```

## Rollback Rehearsal

Rollback is only considered complete when the team explicitly records whether the safe path is:

- app-only rollback to a previous image tag
- down-migration rollback using the files in `migrations/down/`
- restore from backup or snapshot

Minimum sequence:

1. Record the rollback checkpoint before applying forward changes.
2. Execute the approved rollback path in the rehearsal environment.
3. Verify the target image or schema state is restored.
4. Re-run minimum health and export checks.
5. Record any data loss or incompatibility risk.

## Seed and Fixture Validation

Validate that the data required for smoke flows is present after migration or restore:

- KYB evidence package rows or fixture equivalents
- treasury evidence import sources or representative fixtures
- reconciliation evidence rows or representative fixtures
- audit or export paths needed for operator evidence

If synthetic or sample data is used, the checklist must label it explicitly and explain why it is acceptable for the rehearsal.

## Required Evidence

Attach the following to the release packet:

- rehearsal environment identifier
- forward migration output
- rollback output or restore log
- pre- and post-migration schema timestamps
- seed or fixture validation notes
- operator and reviewer identity
- release candidate SHA
- blocking issues or waivers

## Fail-Closed Rules

The migration slice remains open if:

- any forward migration in scope was not rehearsed
- rollback path was not recorded
- seed or fixture validation is missing
- admin smoke checks after migration or rollback were not run
- evidence does not point to the same candidate SHA as the release checklist
