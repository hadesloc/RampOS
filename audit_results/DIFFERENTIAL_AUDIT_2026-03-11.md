# Differential Audit - 2026-03-11

## Scope
- RFQ backend/service/handlers
- LP RFQ auth
- Admin RFQ frontend/client
- Admin licensing compatibility endpoints/client mapping
- Admin swap endpoints/page
- Admin bridge route exposure

## Findings
- No remaining blocking findings in the patched scope after verification.
- One intermediate issue was found during audit: swap history used a process-global store and risked cross-tenant leakage. This was fixed by tenant-scoping the history store before completion.

## Residual Risks
- RFQ finalize flow still uses multiple repository writes rather than one transaction.
- Licensing upload endpoint currently provides a compatibility storage path rather than a hardened persistent document pipeline.
- There are unrelated compile warnings elsewhere in the repo outside this patch scope.

## Verification Evidence
- cargo test -p ramp-core service::rfq::tests:: -- --nocapture
- cargo test -p ramp-api --lib handlers::lp::rfq::tests:: -- --nocapture
- cargo test -p ramp-api --lib test_tenant_scope_guard_rejects_cross_tenant_status_reads -- --nocapture
- cargo test -p ramp-api --lib handlers::swap::tests:: -- --nocapture
- cargo test -p ramp-api --lib handlers::admin::licensing_tests::tests:: -- --nocapture
- cargo test -p ramp-api --lib --no-run
- npm run test:run -- src/__tests__/rfq-page.test.tsx
- npx tsc --noEmit
