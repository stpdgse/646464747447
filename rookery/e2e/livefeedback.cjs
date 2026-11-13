// Move feedback in online games: shown in casual games, never in rated ones. Needs a production server on :3111.
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
const PW = 'e2e-test-password-1';
const LABELS = ['Brilliant', 'Great', 'Best', 'Good', 'Inaccuracy', 'Mistake', 'Blunder'];
const ok = (m) => console.log('OK  ', m);

async function signup(ctx, name) {
  const p = await ctx.newPage();
  await p.goto(B + '/signup'); await p.fill('#username', name); await p.fill('#password', PW); await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });
  return p;
}
async function pair(browser, casual) {
  const ca = await browser.newContext({ viewport: { width: 1280, height: 1000 } }), cb = await browser.newContext({ viewport: { width: 1280, height: 1000 } });
  const a = await signup(ca, 'e2ef' + rnd()), b = await signup(cb, 'e2ef' + rnd());
  await a.goto(B + '/play/online'); await b.goto(B + '/play/online');
  for (const p of [a, b]) {
    await p.waitForSelector('button:text-is("3 + 2")');
    if (casual) await p.click('[role=radio]:text-is("Casual")');
  }
  await a.click('button:text-is("3 + 2")'); await a.waitForSelector('text=Looking for an opponent');
  await b.click('button:text-is("3 + 2")');
  await a.waitForURL(/\/game\//, { timeout: 30000 }); await b.waitForURL(/\/game\//, { timeout: 30000 });
  await a.waitForSelector('.cg-wrap piece'); await b.waitForSelector('.cg-wrap piece');
  const white = (await a.evaluate(() => !document.querySelector('.cg-wrap').classList.contains('orientation-black'))) ? a : b;
  return { a, b, white };
}
async function e4(p) {
  const box = await p.locator('.cg-wrap').boundingBox();
  for (const [f, r] of [[4, 2], [4, 4]]) { const bx = await p.locator('.cg-wrap').boundingBox(); await p.mouse.click(bx.x + (f + .5) * bx.width / 8, bx.y + (8 - r + .5) * bx.height / 8); await p.waitForTimeout(250); }
  void box;
}

(async () => {
  const browser = await chromium.launch();
  const errors = [];
  // casual
  const c = await pair(browser, true);
  for (const p of [c.a, c.b]) p.on('pageerror', (e) => errors.push(String(e)));
  await c.white.waitForSelector('[aria-label="Move feedback"]:has-text("how good it was")');
  ok('casual game: the feedback bar is there');
  await e4(c.white);
  await c.white.waitForFunction((labels) => { const t = document.querySelector('[aria-label="Move feedback"]')?.textContent || ''; return labels.some((l) => t.includes(l)); }, LABELS, { timeout: 30000 });
  ok('casual game: your move gets rated: ' + (await c.white.locator('[aria-label="Move feedback"]').textContent()).replace(/\s+/g, ' ').trim().slice(0, 60));
  // the opponent's own bar does not show your move's rating
  const other = c.white === c.a ? c.b : c.a;
  const otherText = await other.locator('[aria-label="Move feedback"]').textContent();
  assert(!LABELS.some((l) => otherText.includes(l)), 'the opponent must not see your rating: ' + otherText);
  ok("the opponent doesn't see your rating");

  // rated
  const r = await pair(browser, false);
  for (const p of [r.a, r.b]) p.on('pageerror', (e) => errors.push(String(e)));
  await r.white.waitForSelector('text=only available in casual games');
  assert.equal(await r.white.locator('[aria-label="Move feedback"]').count(), 0);
  await e4(r.white);
  await r.white.waitForTimeout(4000);
  assert.equal(await r.white.locator('[aria-label="Move feedback"]').count(), 0);
  ok('rated game: no feedback bar, just a note');
  assert.deepEqual(errors, [], errors.join('\n'));
  await browser.close();
  console.log('LIVE FEEDBACK PASSED');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
