# T-CLOSE-005 Handoff

## Objective
Enable Playwright execution from `frontend/` using the real `frontend/playwright.config.ts` webServer startup path (`npm run dev`) and verify required closeout specs.

## Root cause and fix

### Initial blocker
Running from `frontend` with real config failed at webServer startup:

`Cannot find package 'next-intl' imported from frontend/next.config.mjs`

### Dependency fix applied
Added missing runtime dependency in frontend app package manifest:

- `next-intl: ^3.26.5` (in `dependencies`)

This version is compatible with the current frontend setup because the app uses legacy `src/i18n.ts` request config behavior (supported in v3 with deprecation warning) rather than the newer v4+ request module layout.

## Files changed

1. `C:/Users/hades/OneDrive/Desktop/New folder (6)/frontend/package.json`
2. `C:/Users/hades/OneDrive/Desktop/New folder (6)/frontend/package-lock.json`
3. `C:/Users/hades/OneDrive/Desktop/New folder (6)/.claude/agents/active/T-CLOSE-005.status.json`
4. `C:/Users/hades/OneDrive/Desktop/New folder (6)/.claude/handoffs/T-CLOSE-005.md`

## Verification commands and outcomes

All commands were run from frontend directory context using real Playwright config (`frontend/playwright.config.ts`), which starts `npm run dev` via `webServer`.

### 1) Required closeout spec: widget components
Command:
`cd "C:/Users/hades/OneDrive/Desktop/New folder (6)/frontend" && npx playwright test e2e/widget-components.spec.ts`

Result:
- PASS
- `2 passed (22.4s)`

### 2) Required closeout spec: websocket subscriptions
Command:
`cd "C:/Users/hades/OneDrive/Desktop/New folder (6)/frontend" && npx playwright test e2e/websocket-subscriptions.spec.ts`

Result:
- PASS
- `1 passed (13.8s)`

### 3) Baseline page spec (light existing spec)
Attempted baseline command:
`cd "C:/Users/hades/OneDrive/Desktop/New folder (6)/frontend" && npx playwright test e2e/pages.spec.ts --grep "should display admin login page"`

Result:
- FAIL (assertion in existing test expectation, not webServer startup)
- Failure detail:
  - locator `getByText(/admin/i)` resolved to hidden `<title>RampOS Admin</title>` and timed out

Additional baseline attempt:
`cd "C:/Users/hades/OneDrive/Desktop/New folder (6)/frontend" && npx playwright test e2e/pages.spec.ts --grep "should load /portal/login without errors"`

Result:
- FAIL (existing test assertion mismatch)
- Failure detail:
  - expected text `Welcome back` not found

## Notes observed during runs
- Next.js webServer now starts successfully under Playwright config after dependency fix.
- Existing warnings/errors unrelated to this dependency fix still appear in dev server output:
  - `next-intl` deprecation warning for reading config from `./src/i18n.ts`
  - Existing import warning in admin page for `useRealtimeDashboard` export mismatch
- These do not block the two required closeout specs.

## Scope compliance
- No test spec files were edited.
- No Next.js app source code was edited.
- Dependency-only fix constrained to package manifests/lockfile.
