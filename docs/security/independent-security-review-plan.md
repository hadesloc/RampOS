# Independent Security Review Plan

## Purpose

This plan turns the current RampOS control-plane implementation into an externally reviewable release candidate. It defines what must be reviewed, how findings are recorded and closed, and how unresolved security risk can block bank-grade signoff.

## Release Candidate Metadata

Fill this section before an external review starts.

| Field | Value |
| --- | --- |
| Release candidate SHA | `TBD` |
| Review window start | `TBD` |
| Review window end | `TBD` |
| Review coordinator | `TBD` |
| Engineering owner | `TBD` |
| Security owner | `TBD` |
| Auditor / firm | `TBD` |
| Artifact root | `docs/security/reports/<yyyy-mm-dd-rc>/` |
| Signoff ledger | `docs/operations/bank-grade-signoff-ledger.md` |

## In-Scope Surfaces

The external review must stay grounded in the implemented M0-M6 control-plane seams. The minimum in-scope surfaces are:

| Surface | Repo seam | Primary files / evidence anchors |
| --- | --- | --- |
| Partner and config governance | Admin + service governance seam | `crates/ramp-api/src/handlers/admin/partners.rs`, `crates/ramp-core/src/service/partner_registry.rs`, `crates/ramp-core/src/service/config_bundle.rs` |
| Corridor and canonical payment paths | Adapter ingress + workflow seam | `crates/ramp-api/src/handlers/bank_webhooks.rs`, `crates/ramp-core/src/service/canonical_payment.rs`, `crates/ramp-core/src/workflows/activities.rs` |
| Compliance routing and institutional evidence | Compliance provider + admin seam | `crates/ramp-api/src/handlers/admin/travel_rule.rs`, `crates/ramp-api/src/handlers/admin/kyb.rs`, `crates/ramp-compliance/src/provider_routing.rs`, `crates/ramp-compliance/src/kyb/evidence_package.rs` |
| Treasury and reconciliation controls | Admin + evidence import seam | `crates/ramp-api/src/handlers/admin/treasury.rs`, `crates/ramp-api/src/handlers/admin/reconciliation.rs`, `crates/ramp-core/src/service/treasury_evidence.rs`, `crates/ramp-core/src/service/reconciliation.rs` |
| Liquidity scoring and explainability | RFQ + solver + admin seam | `crates/ramp-core/src/service/rfq.rs`, `crates/ramp-core/src/chain/solver.rs`, `crates/ramp-api/src/handlers/admin/liquidity.rs` |
| Break-glass and audit export | Admin + audit seam | `crates/ramp-api/src/handlers/admin/audit.rs`, break-glass audit tests in `crates/ramp-api` |
| Release and certification controls | CLI + compatibility seam | `sdk-python/src/rampos/cli/app.py`, `scripts/rampos-cli.py`, `scripts/validate-openapi.sh`, `docs/operations/release-checklist.md` |
| Security workflow and dependency health | Repo security automation seam | `.github/workflows/security-audit.yml`, `Cargo.lock` |

## Explicitly Out of Scope

These areas may still be inspected opportunistically, but they are not part of the required M6 closure package unless a finding crosses into an in-scope seam:

- marketing/landing content
- unrelated smart-contract work outside the active control plane
- historical security reports that predate the current release candidate unless still unresolved

## Review Inputs Required Before Kickoff

The review coordinator must attach these inputs before external review begins:

| Input | Required evidence |
| --- | --- |
| Release candidate freeze | Commit SHA, lockfiles, migration set, environment versions |
| Verification baseline | Latest completed run of `docs/operations/full-verification-matrix.md` |
| Staging baseline | Latest attributable staging rehearsal evidence from `docs/operations/staging-validation-plan.md` |
| Operations baseline | Current runbooks, rollback guidance, and DR plan in `docs/operations/` |
| Threat context | Relevant architecture and hardening notes from `docs/security/threat-model.md`, `docs/security/hardening.md`, and current admin/control-plane flows |

## Finding Record Schema

Every finding must be recorded with the full schema below. Do not treat screenshots, chat logs, or email as the primary record.

| Field | Required? | Notes |
| --- | --- | --- |
| Finding ID | Yes | Stable ID such as `RC-2026-03-F01` |
| Title | Yes | Short, specific issue name |
| Severity | Yes | `critical`, `high`, `medium`, `low`, or `informational` |
| CWE / category | Yes | Use a concrete weakness or security category |
| Affected seam | Yes | Must map to one row in the in-scope table above |
| Affected files / surfaces | Yes | File paths, endpoints, commands, or workflow names |
| Exploit preconditions | Yes | What attacker/operator position is required |
| Impact | Yes | Concrete blast radius, not generic wording |
| Reproduction / evidence | Yes | Steps, commands, payloads, or report reference |
| Suggested remediation | Yes | Specific fix or compensating control |
| Owner | Yes | Named engineering owner |
| Target milestone | Yes | Fix now, defer with expiry, or accept risk |
| Due date | Yes | Required for all non-informational findings |
| Status | Yes | `open`, `triaged`, `in_remediation`, `fixed_pending_verification`, `closed`, `risk_accepted` |
| Closure evidence | Conditionally | Mandatory for `closed` |
| Risk acceptance approver | Conditionally | Mandatory for `risk_accepted` |
| Risk acceptance expiry | Conditionally | Mandatory for `risk_accepted` |
| Verification date | Conditionally | Mandatory for `closed` and `risk_accepted` |

## Severity Handling Rules

| Severity | Initial SLA | Bank-grade rule |
| --- | --- | --- |
| `critical` | triage same day | Must be fixed and re-verified before signoff |
| `high` | triage within 1 business day | Must be fixed and re-verified before signoff unless named risk acceptance exists with explicit expiry |
| `medium` | triage within 2 business days | May remain open only with owner, due date, and accepted remediation plan |
| `low` | triage within 5 business days | Can remain open if tracked and not bank-grade blocking |
| `informational` | best effort | Does not block signoff alone |

## Finding Lifecycle

1. Intake
   - Coordinator records the finding in the finding ledger and links raw evidence.
2. Triage
   - Security owner confirms severity, affected seam, exploitability, and owning team.
3. Remediation
   - Engineering owner ships a fix or compensating control tied to the release candidate or an explicit deferred plan.
4. Verification
   - A different reviewer, or the external reviewer when available, checks the fix using concrete evidence.
5. Closure
   - Only then can the record move to `closed`.
6. Exception handling
   - If a finding is not fixed, it can only move to `risk_accepted` with named approver, rationale, expiry, and review date.

## Closure Evidence Requirements

Closure evidence must include:

- commit or PR reference implementing the fix
- test or verification command proving the fix
- environment used for verification
- date and reviewer identity
- note confirming whether staging or production-like rehearsal was re-run

Examples of acceptable closure evidence:

- targeted test output tied to the affected seam
- updated release or staging evidence showing the fix on the release candidate
- workflow run URL or exported CI log reference

Examples of unacceptable closure evidence:

- "looks good now"
- a chat acknowledgement without commands or artifacts
- a stale run from a different commit SHA

## Risk Acceptance Rules

Risk acceptance is allowed only for `high`, `medium`, or `low` findings. `critical` findings cannot be waived for bank-grade signoff.

Every risk acceptance must include:

- explicit business rationale
- compensating controls already in place
- named approver from security
- named approver from engineering or product
- expiry date
- required re-review trigger

Risk acceptance expires automatically on the earliest of:

- the stated expiry date
- release-candidate SHA changing
- the affected seam changing materially
- a new external review beginning

## Required Outputs

The review coordinator must produce these artifacts under `docs/security/reports/<yyyy-mm-dd-rc>/` or an equivalent immutable evidence location:

- `review-summary.md`
- `finding-ledger.csv` or `finding-ledger.md`
- raw evidence references for each finding
- closure evidence references for each fixed finding
- exception register for all risk acceptances

## Signoff Gate

The bank-grade signoff gate remains closed until:

- all `critical` findings are `closed`
- all `high` findings are either `closed` or `risk_accepted` with named approver and unexpired exception
- every open `medium` or lower finding has an owner and due date
- the current review summary is linked from `docs/operations/bank-grade-signoff-ledger.md`

## Review Handoff Checklist

- release candidate metadata filled
- reviewer firm or individual named
- in-scope seams confirmed
- finding ledger path created
- signoff ledger updated with current security-review status
- release checklist reflects the same review window and SHA
