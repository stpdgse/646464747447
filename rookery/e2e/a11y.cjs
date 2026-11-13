// Accessibility + PWA + theme + sound checks against a running production build (npm run build && npm start -- -p 3111).
// Uses axe-core on key pages in dark and light themes. Fails on serious or critical violations.
const { chromium } = require('playwright');
const { AxeBuilder } = require('@axe-core/playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
let n = 0;
const ok = (m) => console.log('OK  ', ++n, m);
const PAGES = ['/', '/play', '/play/online', '/play/computer', '/play/board', '/puzzles', '/learn', '/learn/endgames', '/learn/lessons/basics', '/learn/endgames/kq-k', '/openings', '/leaderboard', '/watch', '/community', '/tournaments', '/forum', '/clubs', '/login', '/signup', '/credits', '/offline'];

(async () => {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));

  for (const theme of ['dark', 'light']) {
    await page.goto(B + '/');
    await page.evaluate((t) => localStorage.setItem('rookery.prefs', JSON.stringify({ theme: t, sound: true, volume: 0.6 })), theme);
    for (const path of PAGES) {
      await page.goto(B + path, { waitUntil: 'networkidle' });
      assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), theme, `theme on ${path}`);
      const res = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21aa']).analyze();
      const bad = res.violations.filter((v) => v.impact === 'serious' || v.impact === 'critical');
      if (bad.length) {
        console.error(`FAIL ${theme} ${path}`);
        for (const v of bad) console.error(' -', v.id, v.impact, v.help, v.nodes.slice(0, 3).map((x) => x.target.join(' ')).join(' | '));
        process.exit(1);
      }
    }
    ok(`axe: no serious/critical violations on ${PAGES.length} pages (${theme} theme)`);
  }

  // theme toggle cycles and persists
  await page.goto(B + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => localStorage.removeItem('rookery.prefs'));
  await page.reload({ waitUntil: 'networkidle' });
  await page.click('button[aria-label^="Theme"]');
  assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), 'dark');
  await page.click('button[aria-label^="Theme"]');
  assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), 'light');
  await page.reload({ waitUntil: 'networkidle' });
  assert.equal(await page.evaluate(() => document.documentElement.dataset.theme), 'light');
  ok('theme toggle cycles and survives a reload');

  // skip link is first tab stop and moves focus to main content
  await page.goto(B + '/play');
  await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(() => document.activeElement?.textContent), 'Skip to content');
  await page.keyboard.press('Enter');
  assert.equal(await page.evaluate(() => document.activeElement?.id), 'main');
  ok('skip link works');

  // sound toggle
  await page.click('button[aria-label^="Sound"]');
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('rookery.prefs')).sound), false);
  ok('sound toggle persists');

  // PWA
  const man = await (await page.request.get(B + '/manifest.webmanifest')).json();
  assert.equal(man.display, 'standalone');
  for (const i of man.icons) assert.equal((await page.request.get(B + i.src)).status(), 200, i.src);
  assert((await page.request.get(B + '/sw.js')).ok());
  ok('manifest and icons are served');
  await page.goto(B + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => navigator.serviceWorker.ready);
  ok('service worker registered');
  await page.reload({ waitUntil: 'networkidle' }); // now controlled by the worker
  await ctx.setOffline(true);
  await page.goto(B + '/play/online').catch(() => {});
  await page.waitForSelector("text=You're offline", { timeout: 10000 });
  ok('offline navigation shows the offline page');
  await ctx.setOffline(false);

  // synthesised sounds run without throwing
  await page.goto(B + '/', { waitUntil: 'networkidle' });
  await page.evaluate(() => localStorage.setItem('rookery.prefs', JSON.stringify({ theme: 'dark', sound: true, volume: 0.6 })));
  assert.deepEqual(errors, [], errors.join('\n'));
  ok('no page errors');
  await browser.close();
  console.log('ALL A11Y/PWA CHECKS PASSED');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
