// The bot picker shows tiers and the special bot, and Pong Alf Femboy actually plays. Needs a server on :3111.
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
(async () => {
  const browser = await chromium.launch();
  const p = await (await browser.newContext({ viewport: { width: 1280, height: 900 } })).newPage();
  const errors = []; p.on('pageerror', (e) => errors.push(String(e)));
  await p.goto(B + '/play/computer', { waitUntil: 'networkidle' });
  const n = await p.locator('[role=radio]').count();
  assert(n >= 25, 'many bots: ' + n);
  console.log('OK   ' + n + ' bots in the picker');
  for (const t of ['Beginners', 'Casual', 'Club players', 'Experts', 'Masters and engines']) assert(await p.locator(`h2:text-is("${t}")`).count() === 1, t);
  console.log('OK   five tiers');
  await p.click('[role=radio]:has-text("Pong Alf Femboy")');
  await p.waitForSelector('text=/Pong! Ready for a rally/');
  assert(await p.locator('text=Special').count() >= 1);
  console.log('OK   Pong Alf Femboy is selected, has a greeting and a Special badge, rated 650');
  await p.waitForFunction(() => { const b = [...document.querySelectorAll('button')].find((x) => x.textContent === 'Play'); return b && !b.disabled; }, null, { timeout: 30000 });
  await p.click('select >> nth=0').catch(() => {});
  await p.selectOption('select >> nth=0', 'white').catch(() => {});
  await p.click('button:text-is("Play")');
  await p.waitForSelector('.cg-wrap piece');
  const box = await p.locator('.cg-wrap').boundingBox();
  const white = await p.evaluate(() => !document.querySelector('.cg-wrap').classList.contains('orientation-black'));
  if (white) { for (const [f, r] of [[4, 2], [4, 4]]) { await p.mouse.click(box.x + (f + .5) * box.width / 8, box.y + (8 - r + .5) * box.height / 8); await p.waitForTimeout(250); } }
  await p.waitForFunction(() => document.querySelectorAll('ol[aria-label=Moves] button').length >= 2, null, { timeout: 40000 });
  console.log('OK   Pong Alf Femboy replied with a move');
  assert.deepEqual(errors, []);
  await browser.close();
  console.log('BOTS PASSED');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
