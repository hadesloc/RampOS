// Dev-only screenshot + health capture for the RampOS user portal.
// Expects a minted wallet session via env AUTH_TOKEN (+ optional REFRESH_TOKEN);
// injects the cookies, visits every portal route, records errors, saves 2x shots.
//
// Usage: AUTH_TOKEN=... REFRESH_TOKEN=... node scripts/capture-portal.mjs
import { chromium } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const BASE = process.env.BASE_URL || 'http://localhost:3000';
const AUTH_TOKEN = process.env.AUTH_TOKEN || '';
const REFRESH_TOKEN = process.env.REFRESH_TOKEN || '';
const OUT = resolve(process.cwd(), '..', 'docs', 'screenshots');
mkdirSync(OUT, { recursive: true });

// locale-prefixed portal routes -> output filename
const ROUTES = [
  ['/en/portal', 'portal'],
  ['/en/portal/deposit', 'portal-deposit'],
  ['/en/portal/withdraw', 'portal-withdraw'],
  ['/en/portal/assets', 'portal-assets'],
  ['/en/portal/transactions', 'portal-transactions'],
  ['/en/portal/kyc', 'portal-kyc'],
  ['/en/portal/kyc/passport', 'portal-kyc-passport'],
  ['/en/portal/settings', 'portal-settings'],
  ['/en/portal/venues', 'portal-venues'],
  ['/portal/offramp', 'portal-offramp'],
];

const report = [];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// `domcontentloaded` + settle is far more reliable than `networkidle` on pages
// holding live connections (which never go idle). Retry once on transient
// navigation errors (e.g. ERR_NETWORK_IO_SUSPENDED after machine throttling).
async function gotoSettled(page, url, settleMs = 2200) {
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

// Hide dev-only overlays (Next.js indicator + TanStack Query Devtools button).
const DEV_SELECTORS = ['nextjs-portal', '[data-next-badge-root]', '[data-nextjs-toast]', '#__next-build-watcher', '[data-nextjs-dev-tools-button]', 'nextjs-dev-tools-button', '[class*="tsqd"]'];
async function hideDevOverlay(page) {
  await page.addStyleTag({ content: `${DEV_SELECTORS.join(',')}{display:none!important}` }).catch(() => {});
  await page.evaluate((sels) => {
    for (const sel of sels) document.querySelectorAll(sel).forEach((el) => el.remove());
  }, DEV_SELECTORS).catch(() => {});
}

const run = async () => {
  const browser = await chromium.launch();

  // ---- 1) Logged-out login page (fresh context, no cookies) ----
  {
    const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2, locale: 'en-US' });
    const page = await ctx.newPage();
    const errs = [];
    page.on('console', (m) => { if (m.type() === 'error') errs.push(m.text()); });
    page.on('pageerror', (e) => errs.push('PAGEERROR: ' + e.message));
    await gotoSettled(page, `${BASE}/en/portal/login`, 1400);
    await hideDevOverlay(page);
    await page.screenshot({ path: resolve(OUT, 'portal-login.png') });
    report.push({ route: '/en/portal/login', file: 'portal-login.png', errors: errs });
    await ctx.close();
  }

  // ---- 2) Authed portal (inject minted wallet session cookies) ----
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2, locale: 'en-US' });
  if (AUTH_TOKEN) {
    const cookies = [{ name: 'auth_token', value: AUTH_TOKEN, domain: 'localhost', path: '/', httpOnly: true, sameSite: 'Lax' }];
    if (REFRESH_TOKEN) cookies.push({ name: 'refresh_token', value: REFRESH_TOKEN, domain: 'localhost', path: '/', httpOnly: true, sameSite: 'Lax' });
    await context.addCookies(cookies);
  }
  const page = await context.newPage();

  for (const [route, name] of ROUTES) {
    const errs = [];
    const onConsole = (m) => { if (m.type() === 'error') errs.push(m.text()); };
    const onPageErr = (e) => errs.push('PAGEERROR: ' + e.message);
    page.on('console', onConsole);
    page.on('pageerror', onPageErr);
    let status = 'ok';
    try {
      const resp = await gotoSettled(page, `${BASE}${route}`);
      const redirectedToLogin = page.url().includes('/login');
      await hideDevOverlay(page);
      await page.screenshot({ path: resolve(OUT, `${name}.png`) });
      report.push({ route, file: `${name}.png`, http: resp?.status() ?? null, redirectedToLogin, errors: errs });
      if (redirectedToLogin) status = 'REDIRECTED-TO-LOGIN';
    } catch (e) {
      status = 'error';
      report.push({ route, file: `${name}.png`, error: String(e), errors: errs });
    }
    page.off('console', onConsole);
    page.off('pageerror', onPageErr);
    console.log(`[${status}] ${route} -> ${name}.png (${errs.length} console errs)`);
  }

  await browser.close();
  writeFileSync(resolve(OUT, '_portal-health.json'), JSON.stringify(report, null, 2));
  const withErrors = report.filter((r) => (r.errors && r.errors.length) || r.error);
  const redirected = report.filter((r) => r.redirectedToLogin);
  console.log(`\n=== PORTAL SUMMARY ===`);
  console.log(`pages captured: ${report.length}`);
  console.log(`pages with errors: ${withErrors.length}`);
  withErrors.forEach((r) => console.log(`  - ${r.route}: ${r.error || r.errors.slice(0, 2).join(' | ')}`));
  console.log(`pages redirected to login (auth failed): ${redirected.length}: ${redirected.map((r) => r.route).join(', ')}`);
};

run().catch((e) => { console.error(e); process.exit(1); });
