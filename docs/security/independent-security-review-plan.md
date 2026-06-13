# Independent Security Review Plan

This file is retained only as historical context for the RC `268670d74` review package.
The old staffing, kickoff, and handback process scaffolding was removed from the active repo workflow on `2026-04-18`.

## What Still Matters

If a future independent review is commissioned, start from the current code, the current candidate SHA, and fresh evidence. Do not reuse the old RC logistics blindly.

## In-Scope Surfaces For A Future Review

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

## Preserved Historical Evidence

The old RC evidence set remains under `docs/security/reports/2026-03-13-rc-268670d74/`.
Use it as historical reference only.
