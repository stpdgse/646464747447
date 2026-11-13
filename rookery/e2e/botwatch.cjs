// A plays the computer; B (signed out) finds the game on /watch and sees the moves arrive live. Needs a server on :3111.
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
(async () => {
  const browser = await chromium.launch();
  const ca = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const cb = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const a = await ca.newPage(), b = await cb.newPage();
  const errors = []; for (const p of [a, b]) p.on('pageerror', (e) => errors.push(String(e)));
  const name = 'e2ew' + rnd();
  await a.goto(B + '/signup'); await a.fill('#username', name); await a.fill('#password', 'e2e-test-password-1'); await a.click('button[type=submit]');
  await a.waitForURL(B + '/', { timeout: 30000 });
  await a.goto(B + '/play/computer', { waitUntil: 'networkidle' });
  await a.click('[role=radio]:has-text("Der Mann")');
  await a.selectOption('select >> nth=0', 'white');
  await a.waitForFunction(() => { const x = [...document.querySelectorAll('button')].find((y) => y.textContent === 'Play'); return x && !x.disabled; }, null, { timeout: 30000 });
  await a.click('button:text-is("Play")');
  await a.waitForSelector('.cg-wrap piece');
  // the page can re-flow after the board appears, so measure the board fresh for every click
  const sq = async (f, r) => { const box = await a.locator('.cg-wrap').boundingBox(); return { x: box.x + (f + .5) * box.width / 8, y: box.y + (8 - r + .5) * box.height / 8 }; };
  for (const [f, r] of [[4, 2], [4, 4]]) { const s = await sq(f, r); await a.mouse.click(s.x, s.y); await a.waitForTimeout(250); }
  await a.waitForFunction(() => document.querySelectorAll('ol[aria-label=Moves] button').length >= 2, null, { timeout: 40000 });
  console.log('OK   A is playing Der Mann');

  await b.goto(B + '/watch', { waitUntil: 'networkidle' });
  await b.waitForSelector(`a:has-text("${name}")`, { timeout: 20000 });
  console.log('OK   B sees the bot game on /watch without logging in');
  await b.click(`a:has-text("${name}")`);
  await b.waitForSelector('.cg-wrap piece');
  const count = () => b.locator('ol[aria-label=Moves] button').count();
  await b.waitForFunction(() => document.querySelectorAll('ol[aria-label=Moves] button').length >= 2, null, { timeout: 15000 });
  const before = await count();
  console.log('OK   B sees the board and', before, 'moves');
  // A keeps playing: Nf3
  for (const [f1, r1, f2, r2] of [[6, 1, 5, 3]]) { const s1 = await sq(f1, r1); await a.mouse.click(s1.x, s1.y); await a.waitForTimeout(250); const s2 = await sq(f2, r2); await a.mouse.click(s2.x, s2.y); }
  await b.waitForFunction((n) => document.querySelectorAll('ol[aria-label=Moves] button').length > n, before, { timeout: 20000 });
  console.log('OK   a new move appeared for B live');
  assert.equal(await b.locator('.cg-wrap piece').first().evaluate((e) => !!e.closest('.cg-wrap')), true);
  // spectator cannot move: board is read-only (no move-dest hints after clicking a piece)
  const bb = await b.locator('.cg-wrap').boundingBox();
  await b.mouse.click(bb.x + 4.5 * bb.width / 8, bb.y + 4.5 * bb.height / 8); // e4 pawn
  assert.equal(await b.locator('square.move-dest').count(), 0);
  console.log('OK   the spectator board is read-only');
  // A resigns -> B sees the result and the game leaves the live list
  await a.click('button:text-is("Resign")');
  await b.waitForSelector('text=Finished', { timeout: 15000 });
  console.log('OK   B sees the result when A resigns');
  await b.goto(B + '/watch', { waitUntil: 'networkidle' });
  await b.waitForSelector('#botrecent', { timeout: 10000 });
  assert.equal(await b.locator('section[aria-labelledby=botlive] a:has-text("' + name + '")').count(), 0);
  console.log('OK   the finished game moved from live to just-finished');
  assert.deepEqual(errors, []);
  await browser.close();
  console.log('BOT WATCH PASSED  (test account: ' + name + ')');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
