# Release Checklist

This checklist fails closed. If a required artifact or evidence file is missing, the release candidate does not advance.

## 1. Freeze The Candidate

- Record the candidate SHA and use it as `--release-candidate <sha>` for all `scripts/release_hardening.py` runs.
- Record dependency baselines:
  - `Cargo.lock`
  - `sdk-python/pyproject.toml`
  - `sdk/package-lock.json` or current Node lockfile if present
- Record the migration window in scope, including additive migrations `043` through `048` if they are part of the candidate.
- Create or identify the evidence directory:

```powershell
python scripts/release_hardening.py --dry-run --release-candidate <sha>
```

Required evidence:
- `docs/operations/evidence/<sha>/dry-run-plan.md`
- `docs/operations/evidence/<sha>/summary.md`

## 2. Run The Non-Destructive Verification Matrix

Execute the baseline matrix:

```powershell
python scripts/release_hardening.py --group contract-surface --group backend-admin --group core-services --group cli-certification --group audit-controls --release-candidate <sha> --stop-on-failure
```

Blocking rules:
- Any `failed` result in `summary.md` blocks release promotion.
- Any `skipped` result blocks release promotion unless it is formally waived in the bank-grade signoff ledger with named approver and expiry.
- Do not replace the matrix with ad hoc command subsets.

Required evidence:
- `docs/operations/evidence/<sha>/summary.md`
- `docs/operations/evidence/<sha>/summary.json`
- step logs for all executed commands

## 3. Rehearse Migrations And Rollback

Only in an isolated rehearsal database:

```powershell
$env:DATABASE_URL = "<isolated-rehearsal-db>"
python scripts/release_hardening.py --group migration-rehearsal --include-manual --release-candidate <sha> --stop-on-failure
```

Blocking rules:
- Missing `DATABASE_URL` or missing `sqlx` CLI is a blocked rehearsal, not a waived pass.
- Do not run this group against production.
- Record the rehearsal database identifier, operator, and rollback checkpoint in the signoff package.

Required evidence:
- migration and rollback logs
- post-migration regression log

## 4. Validate Seeds And Fixtures

- Confirm the smoke paths required by the release candidate still have valid seeds or fixtures after migration rehearsal.
- At minimum, verify the DB-backed seams exercised by the release candidate still pass their regression entries in the evidence set.
- If fixture state had to be recreated manually, record exactly what changed and who approved it.

Blocking rules:
- Unrepeatable fixture bootstraps block the release.

## 5. Link To Staging, Ops, And Security Proof

- Attach the staging validation evidence package for the same candidate SHA.
- Confirm release, rollback, incident, and on-call runbooks are current for the candidate scope.
- Confirm backup/restore and disaster recovery rehearsal evidence is current.
- Confirm the independent security review ledger is linked and has no unresolved `high` or `critical` findings without explicit risk acceptance.

Blocking rules:
- Missing staging evidence blocks promotion.
- Missing ops runbook coverage blocks promotion.
- Missing DR proof blocks promotion.
- Missing security closure blocks promotion.

## 6. Complete Final Signoff

- Record approvers, timestamps, scope, exceptions, and expiry in the bank-grade signoff ledger.
- Confirm every waiver in the release evidence has a named approver and expiry.
- Confirm the ledger references the exact candidate SHA and evidence directory.

The candidate is not bank-grade ready until every blocking section above is satisfied and linked in the signoff ledger.
