# Warning Reduction Priorities

## Purpose

This document defines the deterministic warning-reduction order for the three main Rust crates in the March 2026 repo-credibility pass.

It is a prioritization artifact, not a cleanup log. Use it to decide which warning clusters to dispatch first after the internal-readiness baseline is stable.

## Ranked Order

| Rank | Crate | First warning clusters to tackle | Why first |
| --- | --- | --- | --- |
| 1 | `ramp-api` | Doc-comment hygiene, unused helpers, duplicated branch logic | These warnings are visible, low-risk, and directly affect operator and reviewer confidence in the public-facing service layer. |
| 2 | `ramp-core` | Dead-code fields and methods, type-complexity hotspots, `new_without_default` cleanup | This crate has the broadest warning surface and the highest long-term drag, but it touches deeper execution paths, so it should follow the lower-risk `ramp-api` pass. |
| 3 | `ramp-compliance` | `too_many_arguments` on case-listing and store interfaces | The current known warnings are narrower and easier to isolate after the broader credibility work above is complete. |

## Current Evidence Basis

The current ordering is grounded in the warning clusters already visible in `clippy_log.txt`:

- `ramp-api`
  - `clippy::empty_line_after_doc_comments`
  - unused functions such as admin or ownership helpers
  - `clippy::if_same_then_else`
- `ramp-core`
  - dead-code fields and methods in workflow and service structs
  - `clippy::type_complexity`
  - `clippy::new_without_default`
- `ramp-compliance`
  - `clippy::too_many_arguments` in case service and store interfaces

## Dispatch Rule

1. Finish the `ramp-api` warning slice first.
2. Move to `ramp-core` only after the `ramp-api` slice has a clean, reviewed target list.
3. Take `ramp-compliance` after the first two slices unless a narrower compliance warning fix becomes an easier filler task for idle capacity.

## Scope Guardrails

- Do not mix warning cleanup with runtime-semantic changes.
- Prefer clusters that can be reviewed in small, isolated batches.
- Treat this order as the default queue unless a release-blocking compiler or security warning overrides it.
