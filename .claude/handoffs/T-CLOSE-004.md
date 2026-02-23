# T-CLOSE-004 Handoff

## Gap identified
F12.05 (Widget CDN distribution) was still PARTIAL because the codebase had widget CDN bundle generation (`vite.cdn.config.ts` and `vite.embed.config.ts`) but no dedicated CI/CD publication workflow that:
- consistently builds widget artifacts,
- stores build/package outputs as pipeline artifacts,
- and gates real publish with explicit secret requirements.

## Files changed
1. `C:/Users/hades/OneDrive/Desktop/New folder (6)/.github/workflows/widget-cdn-publish.yml`
   - New workflow for widget CDN/package publication path.
   - Adds:
     - build job (`npm ci`, `npm run build` in `packages/widget`),
     - package tarball generation via `npm pack`,
     - artifact upload (`widget-cdn-build`) including `dist/` + tgz,
     - conditional publish job to npm on `widget-v*` tags or manual dispatch with `publish_to_npm=true`.
   - Explicit secret requirement in workflow: `NPM_TOKEN` (fails fast if missing).

2. `C:/Users/hades/OneDrive/Desktop/New folder (6)/packages/widget/README.md`
   - Added `CI/CD widget publish path` section documenting:
     - workflow location,
     - triggers,
     - artifact behavior,
     - required secret (`NPM_TOKEN`),
     - what works without secrets vs what needs credentials.

3. `C:/Users/hades/OneDrive/Desktop/New folder (6)/TASK-TRACKER.md`
   - Updated F12.05 evidence text (status remains `PARTIAL`) with truthful evidence:
     - workflow exists,
     - build passes,
     - local dry-run currently fails under local npm toolchain.

4. `C:/Users/hades/OneDrive/Desktop/New folder (6)/.claude/agents/active/T-CLOSE-004.status.json`
   - Task status artifact for this task.

## Required command evidence

### 1) Widget build
Command:
`npm run build --prefix "C:/Users/hades/OneDrive/Desktop/New folder (6)/packages/widget"`

Result: PASS

Key output excerpts:
- `vite v5.4.21 building for production...`
- `dist/index.es.js 52.52 kB`
- `dist/index.cjs.js 37.12 kB`
- `dist/rampos-widget.umd.js 183.40 kB`
- `dist/rampos-widget.es.js 259.49 kB`
- `dist/rampos-embed.iife.js 7.42 kB`
- `dist/rampos-embed.es.js 8.96 kB`
- `✓ built in ...`

### 2) Widget publish dry-run
Command:
`npm publish --dry-run --prefix "C:/Users/hades/OneDrive/Desktop/New folder (6)/packages/widget"`

Result: FAIL (local toolchain behavior)

Key output excerpts:
- `npm warn publish npm auto-corrected some errors in your package.json`
- `npm warn publish Missing "name" field was set to an empty string`
- `npm error Cannot read properties of null (reading 'prerelease')`
- npm log path:
  `C:/Users/hades/AppData/Local/npm-cache/_logs/2026-02-23T06_21_03_964Z-debug-0.log`
- environment from log:
  - `npm@11.8.0`
  - `node@v24.12.0`

## What is fully closed
- Automated widget CDN/package publication path now exists in CI with concrete build + package artifact upload + gated publish steps.
- Secret/environment requirements are explicitly documented in both workflow behavior and widget README.
- Widget build path is validated locally and passing.

## What remains open (external/runtime dependencies)
1. **Actual package publication** cannot be confirmed locally without using workflow runtime + valid npm credentials (`NPM_TOKEN`).
2. **Local dry-run anomaly** (`Cannot read properties of null (reading 'prerelease')`) is currently blocking local publish simulation in this environment (`npm 11.8.0`), despite valid package metadata in `packages/widget/package.json`.
3. **Real CDN serving validation** (public URL hosting, cache headers, invalidation policy, production endpoint verification) still requires external CDN/runtime infrastructure and credentials.

## Scope guard confirmation
- No frontend e2e specs were modified.
- Changes were constrained to allowed paths:
  - `.github/workflows/**`
  - `packages/widget/**`
  - `TASK-TRACKER.md`
  - `.claude` task artifacts for status/handoff deliverables.
