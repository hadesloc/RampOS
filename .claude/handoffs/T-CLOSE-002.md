# T-CLOSE-002 Handoff

## Root cause gap

Playwright E2E coverage for RampOS web components was missing in `frontend/e2e/`.
Existing specs focused on page-level static/assertive checks and did not validate real browser custom-element registration/mounting/runtime behavior (events + attribute-driven behavior).

## Files changed

- `C:/Users/hades/OneDrive/Desktop/New folder (6)/frontend/e2e/widget-components.spec.ts`
- `C:/Users/hades/OneDrive/Desktop/New folder (6)/.claude/agents/active/T-CLOSE-002.status.json`
- `C:/Users/hades/OneDrive/Desktop/New folder (6)/.claude/handoffs/T-CLOSE-002.md`

## What was implemented

Created a dedicated Playwright browser spec for web components:

- Verifies custom element registration for:
  - `rampos-checkout`
  - `rampos-kyc`
  - `rampos-wallet`
- Mounts `rampos-checkout` in browser DOM, confirms shadow DOM render (`[data-testid="rampos-checkout"]`), and asserts emitted `rampos-close` event after clicking close button.
- Mounts `rampos-wallet` with attribute `allow-send="false"`, connects wallet at runtime, and verifies attribute-driven runtime behavior:
  - network reflects `arbitrum`
  - `Send` button is absent
  - `Receive` button remains available

Implementation is deterministic (poll/wait loops with bounded timeout), and fully scoped to `frontend/e2e/**`.

## Verification command (exact)

`npm exec --prefix "C:/Users/hades/OneDrive/Desktop/New folder (6)/frontend" playwright test "e2e/widget-components.spec.ts"`

## Exact verification output (pass/fail)

```text
Running 2 tests using 1 worker

  ✓  1 frontend\e2e\widget-components.spec.ts:20:7 › RampOS web components › registers and mounts checkout element, then emits close event (223ms)
  ✓  2 frontend\e2e\widget-components.spec.ts:78:7 › RampOS web components › mounts wallet element and respects allow-send attribute at runtime (1.4s)

  2 passed (5.7s)
```
