// Dev-only screenshot + health capture for the RampOS admin dashboard.
// Logs in with the dev admin, visits every admin route, records console/page
// errors per page, and writes crisp 2x screenshots + a JSON health report.
//
// Usage: node scripts/capture-admin.mjs
import { chromium } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const BASE = process.env.BASE_URL || 'http://localhost:3000';
const EMAIL = process.env.ADMIN_EMAIL || 'admin@rampos.local';
const PASSWORD = process.env.ADMIN_PASSWORD || 'DevAdmin123!';
const OUT = resolve(process.cwd(), '..', 'docs', 'screenshots');
mkdirSync(OUT, { recursive: true });

// route (under /en) -> output filename (without .png)
const ROUTES = [
  ['/', 'dashboard'],
  ['/intents', 'intents'],
  ['/users', 'users'],
  ['/offramp', 'offramp'],
  ['/onboarding', 'onboarding'],
  ['/settlement', 'settlement'],
  ['/compliance', 'compliance'],
  ['/compliance/kyb', 'compliance-kyb'],
  ['/compliance/passport', 'compliance-passport'],
  ['/compliance/rescreening', 'compliance-rescreening'],
  ['/compliance/travel-rule', 'compliance-travel-rule'],
  ['/venue', 'venue'],
  ['/risk-lab', 'risk-lab'],
  ['/risk', 'risk'],
  ['/fraud', 'fraud'],
  ['/reports', 'reports'],
  ['/documents', 'documents'],
  ['/ledger', 'ledger'],
  ['/reconciliation', 'reconciliation'],
  ['/treasury', 'treasury'],
  ['/limits', 'limits'],
  ['/rfq', 'rfq'],
  ['/liquidity', 'liquidity'],
  ['/sandbox', 'sandbox'],
  ['/swap', 'swap'],
  ['/bridge', 'bridge'],
  ['/yield', 'yield'],
  ['/custody', 'custody'],
  ['/webhooks', 'webhooks'],
  ['/events', 'events'],
  ['/incidents', 'incidents'],
  ['/monitoring', 'monitoring'],
  ['/licensing', 'licensing'],
  ['/settings', 'settings'],
  ['/settings/audit', 'settings-audit'],
  ['/settings/billing', 'settings-billing'],
  ['/settings/branding', 'settings-branding'],
  ['/settings/config-bundles', 'settings-config-bundles'],
  ['/settings/domains', 'settings-domains'],
  ['/settings/extensions', 'settings-extensions'],
  ['/settings/sso', 'settings-sso'],
];

// Optional subset filter: ONLY=intents,compliance node scripts/capture-admin.mjs
const ONLY = (process.env.ONLY || '')
  .split(',')
  .map((s) => s.trim())
  .filter(Boolean);
const SELECTED = ONLY.length ? ROUTES.filter(([, name]) => ONLY.includes(name)) : ROUTES;

const report = [];

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Kill the Next.js dev indicator / build-activity badge so it never pollutes a
// shot. `display:none` alone can be defeated by the web-component's own shadow
// styles, so we also remove the host elements outright.
// Includes the TanStack React Query Devtools floating button ([class*="tsqd"]),
// which mounts bottom-right in dev and otherwise photobombs every shot.
const DEV_SELECTORS = ['nextjs-portal', '[data-next-badge-root]', '[data-nextjs-toast]', '#__next-build-watcher', '[data-nextjs-dev-tools-button]', 'nextjs-dev-tools-button', '[class*="tsqd"]'];
async function hideDevOverlay(page) {
  await page.addStyleTag({ content: `${DEV_SELECTORS.join(',')}{display:none!important}` }).catch(() => {});
  await page.evaluate((sels) => {
    for (const sel of sels) document.querySelectorAll(sel).forEach((el) => el.remove());
  }, DEV_SELECTORS).catch(() => {});
}

// The admin dashboard holds a persistent WebSocket, so `networkidle` never
// fires and every navigation burned the full timeout — then cascaded into
// "interrupted by another navigation" once the OS throttled network I/O.
// `domcontentloaded` is deterministic; the settle sleep covers data + charts.
async function gotoSettled(page, url, settleMs = 2600) {
  let resp = null;
  for (let attempt = 1; attempt <= 2; attempt++) {
    try {
      resp = await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30000 });
      break;
    } catch (e) {
      if (attempt === 2) throw e;
      await sleep(1200);
    }
  }
  await sleep(settleMs);
  return resp;
}

const run = async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 2,
    locale: 'en-US',
  });
  const page = await context.newPage();

  // ---- Capture the login page (logged out) first ----
  const loginErrors = [];
  page.on('console', (m) => { if (m.type() === 'error') loginErrors.push(m.text()); });
  page.on('pageerror', (e) => loginErrors.push('PAGEERROR: ' + e.message));
  await gotoSettled(page, `${BASE}/en/admin-login`, 1400);
  await hideDevOverlay(page);
  await page.screenshot({ path: resolve(OUT, 'admin-login.png') });
  report.push({ route: '/admin-login', file: 'admin-login.png', errors: [...loginErrors] });

  // ---- Log in ----
  await page.fill('input[type=email]', EMAIL);
  await page.fill('input[type=password]', PASSWORD);
  await page.click('button[type=submit]');
  // wait for redirect away from the login page
  await page.waitForURL((u) => !u.pathname.includes('admin-login'), { timeout: 30000 }).catch(() => {});
  await sleep(2000);
  const loggedIn = !page.url().includes('admin-login');
  console.log('Logged in:', loggedIn, '->', page.url());
  if (!loggedIn) {
    report.push({ route: 'LOGIN', error: 'login failed', url: page.url() });
  }

  // ---- Visit each admin route ----
  for (const [route, name] of SELECTED) {
    const errs = [];
    const onConsole = (m) => { if (m.type() === 'error') errs.push(m.text()); };
    const onPageErr = (e) => errs.push('PAGEERROR: ' + e.message);
    page.on('console', onConsole);
    page.on('pageerror', onPageErr);
    const url = `${BASE}/en${route === '/' ? '' : route}`;
    let status = 'ok';
    try {
      const resp = await gotoSettled(page, url);
      // hide the Next.js dev indicator / error overlay so it never pollutes shots
      await hideDevOverlay(page);
      // detect a demo badge if present
      const demo = await page.locator('text=/\\bdemo\\b/i').count().catch(() => 0);
      await page.screenshot({ path: resolve(OUT, `${name}.png`) });
      report.push({ route, file: `${name}.png`, http: resp?.status() ?? null, demoBadge: demo > 0, errors: errs });
    } catch (e) {
      status = 'error';
      report.push({ route, file: `${name}.png`, error: String(e), errors: errs });
    }
    page.off('console', onConsole);
    page.off('pageerror', onPageErr);
    console.log(`[${status}] ${route} -> ${name}.png (${errs.length} console errs)`);
  }

  await browser.close();
  writeFileSync(resolve(OUT, '_admin-health.json'), JSON.stringify(report, null, 2));
  // summary
  const withErrors = report.filter((r) => (r.errors && r.errors.length) || r.error);
  const demos = report.filter((r) => r.demoBadge);
  console.log(`\n=== SUMMARY ===`);
  console.log(`pages captured: ${report.length}`);
  console.log(`pages with console/page errors: ${withErrors.length}`);
  withErrors.forEach((r) => console.log(`  - ${r.route}: ${r.error || (r.errors.slice(0, 2).join(' | '))}`));
  console.log(`pages showing demo badge: ${demos.length}: ${demos.map((d) => d.route).join(', ')}`);
};

run().catch((e) => { console.error(e); process.exit(1); });
