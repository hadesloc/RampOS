# Recent Roadmap and Security Hardening (March 2026)

This document summarizes the major roadmap delivery and security hardening work landed in the repository during the recent W1-W16 buildout and the follow-up audit remediation pass. It is intended as a handoff for engineers, reviewers, and auditors working from the current workspace state.

## Scope

The changes covered here span:

- new admin, frontend, core, compliance, widget, SDK, CLI, and workflow surfaces added during the roadmap execution
- targeted audit findings fixed after the build
- follow-up guardrails added to keep the new surfaces fail-closed

This document is repository-grounded. It describes the current code and workflow state rather than aspirational architecture.

## Roadmap Delivery Areas

### Sandbox and Replay

The sandbox layer was added to seed deterministic demo/test tenants and expose replay-ready fixtures for operators and integrators.

Key repo surfaces:

- Migration: [035_sandbox_presets.sql](/C:/Users/hades/OneDrive/Desktop/p2p/migrations/035_sandbox_presets.sql)
- Core service: [sandbox.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/sandbox.rs)
- Admin API: [sandbox.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/sandbox.rs)
- Replay assembly: [replay.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/replay.rs)
- Frontend page: [page.tsx](/C:/Users/hades/OneDrive/Desktop/p2p/frontend/src/app/[locale]/(admin)/sandbox/page.tsx)

The design intentionally keeps sandbox behavior additive. It does not fork the main onboarding flow; it reuses shared bootstrap primitives in core services.

### Webhook Event Catalog and Replay Controls

Webhook delivery moved toward a contract-driven model:

- Event catalog: [event_catalog.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/event_catalog.rs)
- Delivery and replay logic: [webhook_delivery.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/webhook_delivery.rs)
- Admin handlers: [webhooks.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/webhooks.rs)
- Contract docs: [API.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/API.md), [webhooks.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/api/webhooks.md)

This batch also introduced filtered delivery history and replay-by-event controls so operators can inspect and requeue specific webhook flows with better provenance.

### Incident Timeline and SLA Guardian

An incident workbench was built to correlate webhook, RFQ, settlement, and reconciliation signals into one operator timeline.

Key repo surfaces:

- Timeline domain: [incident_timeline.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/incident_timeline.rs)
- Admin API: [incidents.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/incidents.rs)
- Frontend component: [IncidentTimeline.tsx](/C:/Users/hades/OneDrive/Desktop/p2p/frontend/src/components/incidents/IncidentTimeline.tsx)
- SLA routing: [sla_guardian.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/sla_guardian.rs)

The resulting operator model is recommendation-only. It does not auto-execute remediations from the timeline.

### Liquidity, Risk Lab, Reconciliation, Treasury, and Net Settlement

Several operator workbenches were added as bounded admin surfaces:

- Liquidity reliability snapshots: [036_lp_reliability_snapshots.sql](/C:/Users/hades/OneDrive/Desktop/p2p/migrations/036_lp_reliability_snapshots.sql), [liquidity_reliability.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/liquidity_reliability.rs)
- Risk Lab: [038_risk_lab_replay_metadata.sql](/C:/Users/hades/OneDrive/Desktop/p2p/migrations/038_risk_lab_replay_metadata.sql), [risk_lab.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-compliance/src/risk_lab.rs), [risk_lab.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/risk_lab.rs)
- Reconciliation: [reconciliation.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/reconciliation.rs), [reconciliation_export.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/reconciliation_export.rs)
- Treasury: [treasury.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/treasury.rs)
- Net settlement: [net_settlement.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/net_settlement.rs)

Guardrails kept these slices bounded:

- risk lab is replay/explainability, not a second production risk engine
- net settlement is bilateral and approval-gated, not multilateral netting
- reconciliation remains operator-guided, not auto-clearing

### Travel Rule, Rescreening, Passport, and KYB

Compliance delivery added multiple new bounded subsystems:

- Travel Rule: [037_travel_rule.sql](/C:/Users/hades/OneDrive/Desktop/p2p/migrations/037_travel_rule.sql), [travel_rule/](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-compliance/src/travel_rule)
- Rescreening: [039_rescreening_runs.sql](/C:/Users/hades/OneDrive/Desktop/p2p/migrations/039_rescreening_runs.sql), [rescreening.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-compliance/src/rescreening.rs)
- KYC Passport: [040_kyc_passport.sql](/C:/Users/hades/OneDrive/Desktop/p2p/migrations/040_kyc_passport.sql), [passport.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-compliance/src/passport.rs)
- KYB graph: [041_kyb_graph.sql](/C:/Users/hades/OneDrive/Desktop/p2p/migrations/041_kyb_graph.sql), [graph.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-compliance/src/kyb/graph.rs)

These features landed across both admin and portal surfaces, with supporting OpenAPI and frontend pages where applicable.

### Config Bundles, Extensions, Widget, SDK, and CLI

The later waves added productization and integration surfaces:

- Config bundles / extension registry: [config_bundle.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/config_bundle.rs), [config_bundle.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/config_bundle.rs), [extensions.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/extensions.rs)
- Widget headless/export surfaces: [index.ts](/C:/Users/hades/OneDrive/Desktop/p2p/packages/widget/src/index.ts), [RampOSCheckout.tsx](/C:/Users/hades/OneDrive/Desktop/p2p/packages/widget/src/components/RampOSCheckout.tsx), [headless.test.ts](/C:/Users/hades/OneDrive/Desktop/p2p/packages/widget/tests/headless.test.ts)
- CLI: [rampos-cli.py](/C:/Users/hades/OneDrive/Desktop/p2p/scripts/rampos-cli.py)
- SDK docs/examples: [SDK.md](/C:/Users/hades/OneDrive/Desktop/p2p/docs/SDK.md), [README.md](/C:/Users/hades/OneDrive/Desktop/p2p/sdk-go/README.md), [README.md](/C:/Users/hades/OneDrive/Desktop/p2p/sdk-python/README.md)

## Major Security Issues Found and Fixed

The build was followed by a broad audit pass over new and legacy logic. The highest-signal remediations already landed in the repo.

### Fixed: Admin Frontend Auth Bypass

The admin layout now enforces a valid admin session before rendering protected admin pages.

Key file:

- [layout.tsx](/C:/Users/hades/OneDrive/Desktop/p2p/frontend/src/app/[locale]/(admin)/layout.tsx)

### Fixed: GraphQL Cross-Tenant Access

GraphQL no longer trusts client-controlled `tenantId` when an authenticated tenant context exists. Query, mutation, and subscription resolvers are now tenant-scoped from server-side auth context.

Key files:

- [mod.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/graphql/mod.rs)
- [query.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/graphql/query.rs)
- [mutation.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/graphql/mutation.rs)
- [subscription.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/graphql/subscription.rs)

### Fixed: Admin RBAC Escalation and Weak Mutate Gates

Admin role derivation was moved server-side and multiple mutate endpoints were raised from viewer-level access to operator-level access.

Key files:

- [tier.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/tier.rs)
- [onboarding.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/onboarding.rs)
- [webhooks.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/webhooks.rs)
- [mod.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/mod.rs)
- [reports.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/reports.rs)
- [risk_lab.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/handlers/admin/risk_lab.rs)

### Fixed: Travel Rule Fail-Open and Destination Override

Travel Rule no longer allows request-supplied endpoint override, and policy fallback no longer silently allows when no valid policy survives evaluation.

Key files:

- [exchange.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-compliance/src/travel_rule/exchange.rs)
- [policy.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-compliance/src/travel_rule/policy.rs)

### Fixed: Webhook Replay Duplication

Replay from DLQ now consumes the original dead-letter entry and replay-by-event deduplicates endpoints instead of compounding prior replay records.

Key file:

- [webhook_delivery.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/webhook_delivery.rs)

### Fixed: Portal Auth Fail-Open and Admin-Proxy Boundary Leak

Portal frontend auth no longer bootstraps a fake guest-authenticated session, and the portal passport page no longer goes through the admin proxy path.

Key files:

- [auth-context.tsx](/C:/Users/hades/OneDrive/Desktop/p2p/frontend/src/contexts/auth-context.tsx)
- [PassportPortalView.tsx](/C:/Users/hades/OneDrive/Desktop/p2p/frontend/src/components/compliance/PassportPortalView.tsx)
- [route.ts](/C:/Users/hades/OneDrive/Desktop/p2p/frontend/src/app/api/admin-login/route.ts)

### Fixed: Secret Storage Ambiguity

Stored API and webhook secrets now use explicit storage markers and shared decoding logic. Production mode rejects legacy unversioned raw secret storage.

Key files:

- [crypto.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/crypto.rs)
- [auth.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-api/src/middleware/auth.rs)
- [webhook.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/webhook.rs)
- [onboarding.rs](/C:/Users/hades/OneDrive/Desktop/p2p/crates/ramp-core/src/service/onboarding.rs)

## Guardrails and Intentional Fail-Closed Behaviors

Several current behaviors are intentionally strict:

- admin pages fail closed when no valid admin session exists
- settings UI fails closed when multiple tenants are visible instead of guessing `tenants[0]`
- GraphQL rejects authenticated tenant mismatches rather than falling back to client input
- document download paths require tenant-scoped storage keys
- Travel Rule falls back to review, not allow
- secret decoding rejects legacy raw production storage
- `bankReference` incident lookups avoid the old unsafe correlation path and now stay internally consistent
- OIDC token exchange now uses the configured redirect URI rather than trusting callback-supplied redirect data

These are security-first behaviors, even when they expose unfinished integration debt rather than papering over it.

## CI and Release Hardening

Workflow and release posture were tightened after the audit:

- mutable action refs in `.github/workflows` were pinned to concrete commit SHAs
- Python SDK lint/type-check in [sdk-generate.yml](/C:/Users/hades/OneDrive/Desktop/p2p/.github/workflows/sdk-generate.yml) no longer uses `|| true`
- drift detection now fails when contract surface changes without corresponding generated SDK/CLI artifact changes
- [validate-openapi.sh](/C:/Users/hades/OneDrive/Desktop/p2p/scripts/validate-openapi.sh) no longer auto-blesses a baseline or ignores OpenAPI validation failure
- widget npm publish now requests OIDC token permission and uses `npm publish --provenance`

## Verification Commands and What They Proved

The following commands were run during the recent hardening pass:

- `cargo +stable-x86_64-pc-windows-msvc test -p ramp-api --test graphql_runtime_tests test_graphql_rejects_cross_tenant_query_even_with_valid_auth -- --nocapture`
  - proved authenticated GraphQL requests cannot spoof `tenantId` to read another tenant

- `cargo +stable-x86_64-pc-windows-msvc test -p ramp-api --test webhook_admin_test webhook_admin_retry_requires_operator_role -- --exact --nocapture`
  - proved viewer-level admin keys can no longer retry webhook deliveries

- `cargo +stable-x86_64-pc-windows-msvc test -p ramp-api --test reconciliation_admin_test reconciliation_batch_creation_requires_operator_role -- --exact --nocapture`
  - proved reconciliation batch creation is no longer open to viewer-level admin keys

- `cargo +stable-x86_64-pc-windows-msvc test -p ramp-api test_tenant_scope_guard_rejects_cross_tenant_status_reads --lib -- --nocapture`
  - proved licensing status reads are tenant-bound

- `cargo +stable-x86_64-pc-windows-msvc test -p ramp-api test_document_key_scope_guard_only_accepts_current_tenant_prefix --lib -- --nocapture`
  - proved document download path accepts only tenant-scoped storage keys

- `cargo +stable-x86_64-pc-windows-msvc test -p ramp-api test_decode_stored_api_secret_rejects_legacy_raw_secret_in_production_without_key --lib -- --nocapture`
  - proved production mode rejects legacy unversioned API secret storage

- `cargo +stable-x86_64-pc-windows-msvc test -p ramp-core test_validate_webhook_url_rejects_private_and_credentialed_targets --lib -- --nocapture`
  - proved webhook destination validation rejects private-address and embedded-credential targets

- `cargo +stable-x86_64-pc-windows-msvc test -p ramp-api --test incidents_admin_test incidents_bank_reference_lookup_no_longer_hits_dead_path_without_db -- --exact --nocapture`
  - proved the `bankReference` incident route no longer hits the previous dead path in no-DB mode

- `cargo +stable-x86_64-pc-windows-msvc check -p ramp-core --quiet`
  - proved the core crate still compiles after secret-storage and OIDC hardening

- `frontend: npx tsc --noEmit`
  - proved recent auth and admin layout changes are type-safe in the frontend workspace

- `frontend: npm run test:run -- src/__tests__/passport-page.test.tsx src/__tests__/admin-layout.test.tsx`
  - proved admin redirect behavior and portal passport page still behave after fail-closed auth changes

- `packages/widget: npm run test:run -- communication.test.ts`
  - proved widget message-path changes did not break the existing communication test surface

- `bash -n scripts/validate-openapi.sh`
  - proved the hardened validation script remains syntactically valid

## Remaining Non-Critical Cleanup Debt

The main remaining debt is not emergency security work. It is cleanup and consistency work:

- repo-wide compiler/test warnings remain across many legacy modules
- some historical tests and fixtures still use raw secret blobs in non-production harnesses
- workflow hardening now pins actions, but the repo could still benefit from a smaller and more curated CI surface
- tracked Python bytecode artifacts and other local hygiene issues should be cleaned from the repository state
- documentation can still be improved around the exact operational guardrails of each new admin surface

## Recommended Next Steps

If continuing from the current state, the highest-value follow-up work is:

1. remove tracked local artifact noise such as bytecode and temporary verification outputs from the repository snapshot
2. do a repo-wide warning reduction pass, starting with `ramp-api`, `ramp-core`, and `ramp-compliance`
3. tighten old test fixtures so they use the new secret-storage encoding helpers by default
4. add a short docs index link from the main engineering docs to this report and related operator references

This repo is now materially safer than the initial post-build state, and the highest-severity audit findings from the recent pass have been addressed in code and in CI guardrails.
