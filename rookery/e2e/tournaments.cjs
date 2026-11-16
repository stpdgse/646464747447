// Tournaments. Phase 1: `node e2e/tournaments.cjs setup`, then make the printed moderator one:
//   update public.profiles set is_moderator = true where username = '<m>';
// Phase 2: `node e2e/tournaments.cjs run`. Needs a production server on :3111 with GAME_SERVER_SECRET set.
const { chromium } = require('playwright');
const fs = require('fs');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const F = '/tmp/tournaments-users.json';
const PW = 'e2e-test-password-1';
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
const ok = (m) => console.log('OK  ', m);
async function signup(ctx, name) {
  const p = await ctx.newPage();
  await p.goto(B + '/signup'); await p.fill('#username', name); await p.fill('#password', PW); await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });
  return p;
}
async function login(ctx, name) {
  const p = await ctx.newPage();
  await p.goto(B + '/login'); await p.fill('#username', name); await p.fill('#password', PW); await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });
  return p;
}
const FILES = 'abcdefgh';
async function move(p, uci) {
  for (const s of [uci.slice(0, 2), uci.slice(2, 4)]) {
    const box = await p.locator('.cg-wrap').boundingBox();
    const black = await p.evaluate(() => document.querySelector('.cg-wrap').classList.contains('orientation-black'));
    const f = FILES.indexOf(s[0]), r = Number(s[1]);
    const col = black ? 7 - f : f, row = black ? r - 1 : 8 - r;
    await p.mouse.click(box.x + (col + .5) * box.width / 8, box.y + (row + .5) * box.height / 8);
    await p.waitForTimeout(250);
  }
}
const nMoves = (p) => p.locator('ol[aria-label=Moves] button').count();

(async () => {
  const browser = await chromium.launch();
  const mk = () => browser.newContext({ viewport: { width: 1280, height: 1000 } });
  if (process.argv[2] === 'setup') {
    const names = { m: 'e2etm' + rnd(), a: 'e2eta' + rnd(), b: 'e2etb' + rnd() };
    for (const k of Object.keys(names)) { const c = await mk(); await signup(c, names[k]); await c.close(); }
    fs.writeFileSync(F, JSON.stringify(names)); console.log(JSON.stringify(names));
    return browser.close();
  }
  const names = JSON.parse(fs.readFileSync(F, 'utf8'));
  const errors = [];
  const [cm, ca, cb] = [await mk(), await mk(), await mk()];
  const m = await login(cm, names.m), a = await login(ca, names.a), b = await login(cb, names.b);
  for (const p of [m, a, b]) p.on('pageerror', (e) => errors.push(String(e)));
  const tname = 'E2E Arena ' + rnd();

  // moderator creates one that starts now
  await m.goto(B + '/tournaments', { waitUntil: 'networkidle' });
  await m.fill('input[minlength="3"]', tname);
  await m.click('button:text-is("Create tournament")');
  await m.waitForSelector(`a:has-text("${tname}")`, { timeout: 15000 });
  ok('moderator created a tournament and it shows as running');
  // players cannot create
  await a.goto(B + '/tournaments', { waitUntil: 'networkidle' });
  assert.equal(await a.locator('button:text-is("Create tournament")').count(), 0);
  ok('regular players do not get the create form');

  await a.click(`a:has-text("${tname}")`);
  await b.goto(B + '/tournaments', { waitUntil: 'networkidle' });
  await b.click(`a:has-text("${tname}")`);
  const url = a.url();
  for (const p of [a, b]) { await p.click('button:text-is("Join the tournament")'); await p.waitForSelector('button:text-is("Play")'); }
  ok('two players joined');
  await a.waitForSelector(`td:has-text("${names.b}")`, { timeout: 15000 });
  ok('standings list both players');

  // both press Play: they are paired and land in the same game
  await a.click('button:text-is("Play")');
  await a.waitForSelector('text=Looking for an opponent');
  await b.click('button:text-is("Play")');
  await a.waitForURL(/\/game\//, { timeout: 30000 });
  await b.waitForURL(/\/game\//, { timeout: 30000 });
  assert.equal(a.url(), b.url());
  ok('Play pairs two waiting players into one game');
  await a.waitForSelector('.cg-wrap piece'); await b.waitForSelector('.cg-wrap piece');
  const aWhite = await a.evaluate(() => !document.querySelector('.cg-wrap').classList.contains('orientation-black'));
  const [w, k] = aWhite ? [a, b] : [b, a];
  const winnerName = aWhite ? names.a : names.b;

  // four plies, then black resigns: the game counts, white gets 2 points
  for (const [pl, mv, n] of [[w, 'e2e4', 1], [k, 'e7e5', 2], [w, 'g1f3', 3], [k, 'b8c6', 4]]) {
    await pl.waitForFunction((n) => document.querySelectorAll('ol[aria-label=Moves] button').length >= n - 1, n, { timeout: 15000 });
    await move(pl, mv);
    await pl.waitForFunction((n) => document.querySelectorAll('ol[aria-label=Moves] button').length >= n, n, { timeout: 15000 });
  }
  await k.click('button:text-is("Resign")'); await k.click('button:text-is("Really resign")');
  await w.waitForSelector('text=You win', { timeout: 15000 });
  ok('a four-move game was played and black resigned');

  await a.goto(url, { waitUntil: 'networkidle' });
  await a.waitForSelector(`tr:has-text("${winnerName}")`);
  const row = await a.locator(`tr:has-text("${winnerName}")`).textContent();
  assert(/2\s*1\s*1 \/ 0 \/ 0/.test(row.replace(/\s+/g, ' ')) || /2.*1.*1 \/ 0 \/ 0/.test(row), 'winner row: ' + row);
  ok('the standings give the winner 2 points (1 win)');

  // after the game both are free: Play again works
  await a.click('button:text-is("Play")');
  await a.waitForSelector('text=Looking for an opponent');
  await a.click('button:text-is("Stop looking")');
  await a.waitForSelector('button:text-is("Play")');
  ok('a player can stop looking for a game');

  assert.deepEqual(errors, [], errors.join('\n'));
  await browser.close();
  console.log('TOURNAMENTS PASSED (' + tname + ')');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
