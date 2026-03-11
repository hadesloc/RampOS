# RampOS CLI Coverage Ledger

_Last updated: 2026-03-11_

This ledger maps the current RampOS product surface to the planned CLI surface for agent-driven use.

## Status Legend

- `READY`: Backend surface exists and should have a CLI command.
- `NEEDS_API`: UI or product surface exists but backend/API contract is incomplete for CLI parity.
- `DEFERRED`: Backend surface exists but is intentionally delayed from the first parity milestone.

## Coverage

| Feature | Frontend Surface | Backend Surface | Protocol | Contract Source | CLI Command | Status | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Intent lifecycle operations | `frontend/src/app/[locale]/(admin)/intents/page.tsx` | `/v1/intents`, `/v1/intents/payin`, `/v1/intents/payout` | REST | OPENAPI | `rampos intents list|get|create-payin|confirm-payin|create-payout` | READY | Core tenant API surface for pay-in and pay-out. |
| User balances and KYC status | `frontend/src/app/[locale]/(admin)/users/page.tsx` | `/v1/users/{tenant_id}/{user_id}/balances`, portal KYC routes | REST | OPENAPI | `rampos users balances|kyc-status` | READY | Existing API/docs and SDK coverage already exist. |
| Ledger explorer | `frontend/src/app/[locale]/(admin)/ledger/page.tsx` | `/v1/admin/ledger/entries`, `/v1/admin/ledger/balances` | REST | OPENAPI | `rampos ledger entries|balances` | READY | Admin accounting surface should remain machine-readable by default. |
| Webhook operations | `frontend/src/app/[locale]/(admin)/webhooks/page.tsx` | `/v1/admin/webhooks`, replay/retry/catalog/history routes | REST | OPENAPI | `rampos admin webhooks list|get|replay|retry|catalog|history` | READY | Useful for operator workflows and AI-driven replay. |
| Sandbox preset operations | `frontend/src/app/[locale]/(admin)/sandbox/page.tsx` | `/v1/admin/sandbox/seed`, replay/export routes | REST | CURATED | `rampos sandbox presets|seed|run|replay` | READY | Current preview CLI already covers part of this surface. |
| Reconciliation workbench | `frontend/src/app/[locale]/(admin)/reconciliation/page.tsx` | `/v1/admin/reconciliation/workbench`, evidence/export routes | REST | CURATED | `rampos reconciliation workbench|evidence|export` | READY | Existing preview CLI command family. |
| Treasury workbench | `frontend/src/app/[locale]/(admin)/treasury/page.tsx` | `/v1/admin/treasury/workbench`, export route | REST | CURATED | `rampos treasury workbench|export` | READY | Existing preview CLI command family. |
| Settlement workbench | `frontend/src/app/[locale]/(admin)/settlement/page.tsx` | `/v1/admin/settlement/workbench`, export route | REST | CURATED | `rampos settlement workbench|export` | READY | Mounted in router but not in the preview CLI yet. |
| RFQ auction marketplace | `frontend/src/app/[locale]/(admin)/rfq/page.tsx` | `/v1/portal/rfq`, `/v1/admin/rfq/open`, `/v1/admin/rfq/:id/finalize` | REST | CURATED | `rampos rfq create|get|accept|cancel|list-open|finalize` | READY | Completed in docs/COMPLETION_STATUS.md with portal + admin flows. |
| LP RFQ bidding | LP dashboard pending | `/v1/lp/rfq/:rfq_id/bid` | REST | CURATED | `rampos lp rfq bid` | READY | Uses `X-LP-Key` auth; backend complete even though LP UI is still pending. |
| Admin bridge operations | `frontend/src/app/[locale]/(admin)/bridge/page.tsx` | `/v1/admin/bridge/chains`, routes, quote, transfer, status, tokens | REST | CURATED | `rampos bridge chains|routes|quote|transfer|status|tokens` | READY | Added in recent audit/update pass and mounted in router. |
| Chain bridge and quotes | No dedicated page; used by operations and SDK | `/v1/chains`, `/v1/chains/:chain_id/quote`, `/v1/chains/bridge` | REST | OPENAPI | `rampos chain list|get|quote|bridge` | READY | Public chain abstraction should remain available alongside admin bridge tools. |
| Swap console | `frontend/src/app/[locale]/(admin)/swap/page.tsx` | `/v1/swap/quote`, `/v1/swap/execute`, `/v1/swap/history` | REST | CURATED | `rampos swap quote|execute|history` | READY | Differential audit confirmed tenant-scoped history store fix. |
| Licensing requirements and status | `frontend/src/app/[locale]/(admin)/licensing/page.tsx` | `/v1/admin/licensing/status`, requirements, deadlines, submissions | REST | CURATED | `rampos licensing status|requirements|submissions|submit|deadlines` | READY | Compatibility endpoints must be callable by agents for ops workflows. |
| Licensing compatibility uploads | `frontend/src/app/[locale]/(admin)/licensing/page.tsx` | `/v1/admin/licensing/upload` | REST | CURATED | `rampos licensing upload` | READY | Residual risk: compatibility storage path, not hardened long-term pipeline. |
| Travel Rule queue | `frontend/src/app/[locale]/(admin)/compliance/travel-rule/page.tsx` | registry, disclosures, exceptions resolve/retry routes under `/v1/admin/travel-rule/*` | REST | CURATED | `rampos compliance travel-rule registry|disclosures|exceptions` | READY | Backed by migration 037_travel_rule.sql. |
| Continuous rescreening | `frontend/src/app/[locale]/(admin)/compliance/rescreening/page.tsx` | `/v1/admin/rescreening/runs`, restriction action route | REST | CURATED | `rampos compliance rescreening runs|restrict-user` | READY | Backed by migration 039_rescreening_runs.sql. |
| KYC passport queue | `frontend/src/app/[locale]/(admin)/compliance/passport/page.tsx` | `/v1/admin/passport/queue`, package detail route | REST | CURATED | `rampos compliance passport queue|get-package` | READY | Backed by migration 040_kyc_passport.sql. |
| KYB corporate graph | `frontend/src/app/[locale]/(admin)/compliance/kyb/page.tsx` | `/v1/admin/kyb/reviews`, `/v1/admin/kyb/graph/:id` | REST | CURATED | `rampos compliance kyb reviews|get-graph` | READY | Backed by migration 041_kyb_graph.sql. |
| Risk Lab replay and comparison | `frontend/src/app/[locale]/(admin)/risk-lab/page.tsx` | `/v1/admin/risk-lab/catalog`, replay, compare, detail, graph | REST | CURATED | `rampos compliance risk-lab catalog|replay|compare|get|graph` | READY | Replay metadata added in migration 038_risk_lab_replay_metadata.sql. |
| Incidents timeline | `frontend/src/app/[locale]/(admin)/incidents/page.tsx` | admin incident routes and incident timeline fan-out | REST | CURATED | `rampos admin incidents list|get-timeline` | READY | RFQ-aware incident timeline is in the recent audit scope. |
| Extensions and config bundles | `frontend/src/app/[locale]/(admin)/settings/extensions/page.tsx`, `settings/config-bundles/page.tsx` | `/v1/admin/extensions`, `/v1/admin/config-bundles/export` | REST | CURATED | `rampos admin extensions list|config-bundles export` | READY | Important for machine-driven tenant configuration workflows. |
| Domain management | `frontend/src/app/[locale]/(admin)/settings/domains/page.tsx` | domain CRUD under admin/domain handlers | REST | OPENAPI | `rampos domain create|get|verify|delete` | READY | White-label operations fit the CLI model well. |
| GraphQL query and mutation | GraphiQL/dev-facing surface | `/graphql` query and mutation handlers | GRAPHQL | GRAPHQL | `rampos graphql query|mutation` | READY | Needed for non-REST consumers and parity beyond route inventory. |
| Portal event streaming | Live portal UX / monitoring | `/v1/portal/ws` and `crates/ramp-api/src/handlers/ws.rs` | WS | WS | `rampos watch portal-events|intents|incidents` | READY | WebSocket watch mode should emit JSONL for agents. |
| RFQ user portal UI | Portal RFQ page not yet shipped | Portal RFQ backend exists; portal UI folder not present yet | REST | CURATED | `rampos rfq create|get|accept|cancel` | READY | CLI can cover the user flow before the portal page lands. |
| LP dashboard UI | LP dashboard pending per completion status | LP bid route exists | UI_ONLY_GAP | CURATED | `rampos lp rfq bid` | READY | CLI should close this gap immediately for agents and LP ops. |
| Portal deposit/withdraw/auth | `frontend/src/app/[locale]/portal/deposit`, `withdraw`, `login` | portal auth/intents/offramp/wallet/transactions routes | REST | CURATED | `rampos portal auth|wallet|transactions|withdraw|kyc` | READY | Portal flows require portal-token auth mode in the packaged CLI. |
| Monitoring dashboard | `frontend/src/app/[locale]/(admin)/monitoring/page.tsx` | metrics endpoint plus derived UI views | REST | CURATED | `rampos admin monitoring metrics` | DEFERRED | Low-value compared with transactional and compliance surfaces. |
