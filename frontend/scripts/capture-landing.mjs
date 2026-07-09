// Dev-only screenshot capture for the RampOS marketing landing site (:3002).
// Captures the hero and the bento features grid.
// Usage: node scripts/capture-landing.mjs
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const BASE = process.env.LANDING_URL || 'http://localhost:3002';
const OUT = resolve(process.cwd(), '..', 'docs', 'screenshots');
mkdirSync(OUT, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const HIDE = 'nextjs-portal,[data-nextjs-toast],[data-next-badge-root],#__next-build-watcher{display:none!important}';

const run = async () => {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2, locale: 'en-US' });
  const page = await ctx.newPage();

  await page.goto(BASE, { waitUntil: 'networkidle', timeout: 60000 });
  await sleep(2500); // hero framer-motion entrance
  await page.addStyleTag({ content: HIDE }).catch(() => {});
  await page.evaluate(() => window.scrollTo(0, 0));
  await sleep(400);
  await page.screenshot({ path: resolve(OUT, 'landing-hero.png') });
  console.log('captured landing-hero.png');

  // Scroll the features section heading into view and let whileInView animate in
  const heading = page.getByText('Build an Exchange', { exact: false }).first();
  await heading.scrollIntoViewIfNeeded().catch(() => {});
  // nudge so the section title sits near the top with cards visible below
  await page.evaluate(() => window.scrollBy(0, -80));
  await sleep(1800);
  await page.addStyleTag({ content: HIDE }).catch(() => {});
  await page.screenshot({ path: resolve(OUT, 'landing-features.png') });
  console.log('captured landing-features.png');

  await browser.close();
};

run().catch((e) => { console.error(e); process.exit(1); });
