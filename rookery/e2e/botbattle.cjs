// Bot battles: one player starts two bots fighting, a signed-out visitor watches live on /watch. Needs a production server on :3111.
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
const ok = (m) => console.log('OK  ', m);
const moves = (p) => p.locator('ol[aria-label=Moves] button').count();
(async () => {
  const browser = await chromium.launch();
  const errors = [];
  const a = await (await browser.newContext({ viewport: { width: 1280, height: 1000 } })).newPage(); a.on('pageerror', (e) => errors.push(String(e)));
  const name = 'e2ebb' + rnd();
  await a.goto(B + '/signup'); await a.fill('#username', name); await a.fill('#password', 'e2e-test-password-1'); await a.click('button[type=submit]');
  await a.waitForURL(B + '/', { timeout: 30000 });

  await a.goto(B + '/play/bots', { waitUntil: 'networkidle' });
  await a.selectOption('select >> nth=0', 'dermann');
  await a.selectOption('select >> nth=1', 'pawnie');
  await a.selectOption('select >> nth=3', 'fast');
  await a.waitForFunction(() => { const x = [...document.querySelectorAll('button')].find((y) => y.textContent === 'Start the battle'); return x && !x.disabled; }, null, { timeout: 30000 });
  await a.click('button:text-is("Start the battle")');
  await a.waitForSelector('.cg-wrap piece');
  await a.waitForFunction(() => document.querySelectorAll('ol[aria-label=Moves] button').length >= 4, null, { timeout: 30000 });
  ok('Der Mann and Pawnie are fighting on the host page');

  // a visitor finds it on the Watch tab
  const v = await (await browser.newContext({ viewport: { width: 1280, height: 1000 } })).newPage(); v.on('pageerror', (e) => errors.push(String(e)));
  await v.goto(B + '/watch', { waitUntil: 'networkidle' });
  await v.waitForSelector(`section[aria-labelledby=botbattles] a:has-text("Der Mann")`, { timeout: 20000 });
  assert((await v.locator('section[aria-labelledby=botbattles] a:has-text("Pawnie")').count()) >= 1);
  ok('the Watch tab lists the battle for a signed-out visitor');
  await v.click(`section[aria-labelledby=botbattles] a:has-text("Pawnie")`);
  await v.waitForSelector('.cg-wrap piece');
  await v.waitForSelector('text=you are watching');
  const n1 = await moves(v);
  await v.waitForFunction((n) => document.querySelectorAll('ol[aria-label=Moves] button').length >= n + 4, n1, { timeout: 30000 });
  ok('the spectator sees the moves arrive live (' + n1 + ' -> ' + (await moves(v)) + ')');
  // read-only: clicking a piece shows no move hints
  const bx = await v.locator('.cg-wrap').boundingBox();
  await v.mouse.click(bx.x + 4.5 * bx.width / 8, bx.y + 6.5 * bx.height / 8);
  assert.equal(await v.locator('square.move-dest').count(), 0);
  ok('the spectator board is read-only');

  // the host leaves and comes back: the battle can be continued
  await a.goto(B + '/play/bots', { waitUntil: 'networkidle' });
  await a.waitForSelector('text=unfinished bot battle');
  await a.click('button:text-is("Continue it")');
  await a.waitForSelector('.cg-wrap piece');
  await a.waitForFunction(() => document.querySelectorAll('ol[aria-label=Moves] button').length >= 4, null, { timeout: 30000 });
  ok('the host leaves and returns: Continue picks the battle up where it was');

  // pause stops the moves, resume restarts them
  await a.click('button:text-is("Pause")');
  await a.waitForSelector('text=Paused');
  const paused = await moves(a);
  await a.waitForTimeout(2500);
  assert.equal(await moves(a), paused);
  await a.click('button:text-is("Resume")');
  await a.waitForFunction((n) => document.querySelectorAll('ol[aria-label=Moves] button').length > n, paused, { timeout: 20000 });
  ok('pause and resume work');

  // stop: the spectator sees it end
  await a.click('button:text-is("Stop")');
  await a.waitForSelector('text=Stopped');
  await v.waitForSelector('text=Stopped by the host', { timeout: 15000 });
  ok('stopping the battle shows as stopped for spectators');
  await a.click('button:text-is("Rematch (swap colours)")');
  await a.waitForFunction(() => document.querySelectorAll('ol[aria-label=Moves] button').length >= 2, null, { timeout: 30000 });
  ok('rematch with swapped colours starts a new battle');
  await a.click('button:text-is("Stop")');

  // a battle that runs to the end: the strongest bot beats the weakest, and the result is recorded for spectators
  await a.click('button:text-is("New battle")');
  await a.selectOption('select >> nth=0', 'dermann');
  await a.selectOption('select >> nth=1', 'ruedenrache');
  await a.selectOption('select >> nth=3', 'fast');
  await a.click('button:text-is("Start the battle")');
  await a.waitForSelector('text=Der Mann (Rüdenrache) wins', { timeout: 150000 });
  ok('a full battle ends by itself: Der Mann (Rüdenrache) wins');
  await v.goto(B + '/watch', { waitUntil: 'networkidle' });
  await v.waitForSelector('section[aria-labelledby=botbattles] a:has-text("Rüdenrache"):has-text("0-1")', { timeout: 20000 });
  ok('and the finished game with its result shows under "Just finished"');
  assert.deepEqual(errors, [], errors.join('\n'));
  await browser.close();
  console.log('BOT BATTLE PASSED (' + name + ')');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
