// Leaving, closing the app, losing the connection and switching device never loses a game. Needs a production server on :3111.
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
const ok = (m) => console.log('OK  ', m);
const PW = 'e2e-test-password-1';
const moveCount = (p) => p.locator('ol[aria-label=Moves] button').count();
const waitMoves = (p, n, ms = 40000) => p.waitForFunction((n) => document.querySelectorAll('ol[aria-label=Moves] button').length >= n, n, { timeout: ms });
async function sq(p, f, r) { const box = await p.locator('.cg-wrap').boundingBox(); const black = await p.evaluate(() => document.querySelector('.cg-wrap').classList.contains('orientation-black')); const col = black ? 7 - f : f, row = black ? r - 1 : 8 - r; return { x: box.x + (col + .5) * box.width / 8, y: box.y + (row + .5) * box.height / 8 }; }
async function mv(p, f1, r1, f2, r2) { for (const [f, r] of [[f1, r1], [f2, r2]]) { const s = await sq(p, f, r); await p.mouse.click(s.x, s.y); await p.waitForTimeout(250); } }

(async () => {
  const browser = await chromium.launch();
  const errors = [];
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const name = 'e2er' + rnd();
  let p = await ctx.newPage(); p.on('pageerror', (e) => errors.push(String(e)));
  await p.goto(B + '/signup'); await p.fill('#username', name); await p.fill('#password', PW); await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });

  // ---- bot game, timed, White
  await p.goto(B + '/play/computer', { waitUntil: 'networkidle' });
  await p.click('[role=radio]:has-text("Der Mann")');
  await p.selectOption('select >> nth=0', 'white');
  await p.selectOption('select >> nth=2', '10+0').catch(() => {});
  await p.waitForFunction(() => { const x = [...document.querySelectorAll('button')].find((y) => y.textContent === 'Play'); return x && !x.disabled; }, null, { timeout: 30000 });
  await p.click('button:text-is("Play")');
  await p.waitForSelector('.cg-wrap piece');
  await mv(p, 4, 2, 4, 4); await waitMoves(p, 2);
  await mv(p, 6, 1, 5, 3); await waitMoves(p, 4);
  const n1 = await moveCount(p);
  ok('played ' + n1 + ' moves against Der Mann');

  // ---- leave and come back (reload)
  await p.reload({ waitUntil: 'networkidle' });
  await p.waitForSelector('text=Welcome back');
  await waitMoves(p, n1, 15000);
  assert.equal(await moveCount(p), n1);
  ok('reloading the tab restores the same game and moves');

  // ---- close the "app" completely, reopen at the start page
  await p.close();
  p = await ctx.newPage(); p.on('pageerror', (e) => errors.push(String(e)));
  await p.goto(B + '/', { waitUntil: 'networkidle' });
  await p.waitForSelector('text=unfinished game against Der Mann');
  await p.click('a:text-is("Resume")');
  await p.waitForSelector('text=Welcome back');
  await waitMoves(p, n1, 15000);
  ok('after closing everything, the start page offers Resume and returns to the game');

  // ---- lose the internet
  await p.evaluate(() => navigator.serviceWorker.ready);
  await p.reload({ waitUntil: 'networkidle' }); // now controlled by the worker, and this page is cached
  await waitMoves(p, n1, 15000);
  await ctx.setOffline(true);
  await p.waitForSelector('text=You\'re offline');
  ok('offline banner appears');
  await p.reload().catch(() => {});
  await p.waitForSelector('.cg-wrap piece', { timeout: 20000 });
  await waitMoves(p, n1, 15000);
  ok('offline: the game page still opens from the cache with its moves');
  const white = await p.evaluate(() => !document.querySelector('.cg-wrap').classList.contains('orientation-black'));
  assert(white);
  // play on offline: d4 (queen pawn), bot must answer with no connection
  await mv(p, 3, 2, 3, 4);
  await waitMoves(p, n1 + 2, 45000);
  ok('offline: the move was accepted and the bot answered');
  await ctx.setOffline(false);
  await p.waitForSelector('text=You\'re offline', { state: 'detached', timeout: 10000 });
  ok('back online: banner disappears');

  // ---- another device (fresh browser, same account) can continue from the published copy
  await p.waitForTimeout(2500);
  const ctx2 = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const q = await ctx2.newPage(); q.on('pageerror', (e) => errors.push(String(e)));
  await q.goto(B + '/login'); await q.fill('#username', name); await q.fill('#password', PW); await q.click('button[type=submit]');
  await q.waitForURL(B + '/', { timeout: 30000 });
  await q.goto(B + '/play/computer', { waitUntil: 'networkidle' });
  await q.waitForSelector('text=You have an unfinished game against Der Mann');
  await q.click('button:text-is("Continue")');
  await q.waitForSelector('.cg-wrap piece');
  await waitMoves(q, n1 + 2, 15000);
  ok('on a second device, "Continue" brings back the game');
  await ctx2.close();

  // ---- live online game: leave and return
  const ctxA = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const ctxB = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const mk = async (c, n) => { const pg = await c.newPage(); pg.on('pageerror', (e) => errors.push(String(e))); await pg.goto(B + '/signup'); await pg.fill('#username', n); await pg.fill('#password', PW); await pg.click('button[type=submit]'); await pg.waitForURL(B + '/', { timeout: 30000 }); return pg; };
  const a = await mk(ctxA, 'e2eq' + rnd()), b = await mk(ctxB, 'e2eq' + rnd());
  await a.goto(B + '/play/online'); await b.goto(B + '/play/online');
  await a.waitForSelector('button:text-is("3 + 2")'); await b.waitForSelector('button:text-is("3 + 2")');
  await a.click('button:text-is("3 + 2")'); await a.waitForSelector('text=Looking for an opponent');
  await b.click('button:text-is("3 + 2")');
  await a.waitForURL(/\/game\//, { timeout: 30000 }); await b.waitForURL(/\/game\//, { timeout: 30000 });
  const gameUrl = a.url();
  await a.close();
  const a2 = await ctxA.newPage(); a2.on('pageerror', (e) => errors.push(String(e)));
  await a2.goto(B + '/', { waitUntil: 'networkidle' });
  await a2.waitForSelector('text=is still going');
  await a2.click('a:text-is("Resume")');
  await a2.waitForURL(gameUrl, { timeout: 15000 });
  await a2.waitForSelector('.cg-wrap piece');
  ok('online game: the start page offers Resume and returns to the running game');
  assert.deepEqual(errors, [], errors.join('\n'));
  await browser.close();
  console.log('RESUME PASSED  (accounts: ' + name + ' and e2eq*)');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
