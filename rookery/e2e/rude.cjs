// The Rüdenrache Holiday Challenge: play and win levels 100 and 200 (using a real Stockfish to play for us), resign, burn all 8 attempts,
// check the leaderboard, the live game on the Watch tab and the badges. Needs a production server on :3111 with GAME_SERVER_SECRET set.
const { chromium } = require('playwright');
const { spawn } = require('child_process');
const path = require('path');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
const ok = (m) => console.log('OK  ', m);
const FILES = 'abcdefgh';

// A tiny UCI client around the repo's Stockfish build.
function stockfish() {
  const child = spawn(process.execPath, [path.join(__dirname, '../public/engine/stockfish-19-lite-single.js')], { stdio: ['pipe', 'pipe', 'ignore'] });
  let buf = ''; const waiters = [];
  child.stdout.on('data', (d) => { buf += d; let i; while ((i = buf.indexOf('\n')) >= 0) { const line = buf.slice(0, i).trim(); buf = buf.slice(i + 1); for (const w of [...waiters]) if (w.test(line)) { waiters.splice(waiters.indexOf(w), 1); w.resolve(line); } } });
  const send = (c) => child.stdin.write(c + '\n');
  const until = (re) => new Promise((resolve) => waiters.push({ test: (l) => re.test(l), resolve }));
  return {
    // The site writes castling as "king takes its own rook" (e8h8), which is Chess960 notation, so speak that to the engine too.
    async init() { send('uci'); await until(/^uciok/); send('setoption name UCI_Chess960 value true'); send('isready'); await until(/^readyok/); },
    async best(moves) { send('position startpos' + (moves.length ? ' moves ' + moves.join(' ') : '')); send('go movetime 120'); return (await until(/^bestmove/)).split(' ')[1]; },
    quit() { child.kill(); },
  };
}

(async () => {
  const browser = await chromium.launch();
  const mk = () => browser.newContext({ viewport: { width: 1280, height: 1100 } });
  const errors = [];
  const p = await (await mk()).newPage(); p.setDefaultTimeout(30000); p.on('pageerror', (e) => errors.push(String(e)));
  const name = 'e2erd' + rnd();
  await p.goto(B + '/signup'); await p.fill('#username', name); await p.fill('#password', 'e2e-test-password-1'); await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });

  await p.goto(B + '/play', { waitUntil: 'networkidle' });
  await p.waitForSelector('text=Limited time');
  ok('the Play page advertises the limited-time holiday event');
  await p.goto(B + '/play/rudenrache', { waitUntil: 'networkidle' });
  await p.waitForSelector('text=Holiday event');
  assert.equal(await p.locator('ol[aria-label="The Rüden"] li').count(), 20);
  await p.waitForSelector('text=Attempts used: 0 of 8');
  ok('the page shows the holiday banner, the ladder of 20 Rüden and 0 of 8 attempts used');

  const sq = async (s) => {
    await p.locator('.cg-wrap').scrollIntoViewIfNeeded();
    const box = await p.locator('.cg-wrap').boundingBox();
    const black = await p.evaluate(() => document.querySelector('.cg-wrap').classList.contains('orientation-black'));
    const f = FILES.indexOf(s[0]), r = Number(s[1]);
    const col = black ? 7 - f : f, row = black ? r - 1 : 8 - r;
    return { x: box.x + (col + .5) * box.width / 8, y: box.y + (row + .5) * box.height / 8 };
  };
  const movesNow = () => p.locator('[data-moves]').first().getAttribute('data-moves').then((m) => (m ? m.split(' ') : []));
  const sf = stockfish(); await sf.init();

  // Play one level as the side shown on the board until the page says we won (or lost).
  async function playLevel(expectColor) {
    await p.waitForSelector('[data-phase=playing]', { timeout: 30000 });
    assert.equal(await p.locator('[data-moves]').first().getAttribute('data-color'), expectColor);
    for (let guard = 0; guard < 200; guard++) {
      if (await p.locator('text=You beat Rüde').count() || await p.locator('text=Attempt over').count()) return;
      const moves = await movesNow();
      const whiteToMove = moves.length % 2 === 0;
      if ((expectColor === 'white') !== whiteToMove) { await p.waitForTimeout(400); continue; } // the Rüde is thinking
      const best = await sf.best(moves);
      if (process.env.DEBUG_RUDE) console.log('     ply', moves.length, 'best', best, 'moves', moves.join(' ').slice(-40));
      // a castling move is clicked as the king stepping two squares
      const CASTLES = { e1h1: 'e1g1', e1a1: 'e1c1', e8h8: 'e8g8', e8a8: 'e8c8' };
      const usedOrigins = new Set(moves.map((m) => m.slice(0, 2)));
      const click = CASTLES[best] && !usedOrigins.has(best.slice(0, 2)) ? CASTLES[best] : best;
      for (const s of [click.slice(0, 2), click.slice(2, 4)]) { const q = await sq(s); await p.mouse.click(q.x, q.y); await p.waitForTimeout(200); }
      if (best.length === 5) await p.click('button[aria-label=Queen]');
      await p.waitForFunction((n) => { const m = document.querySelector('[data-moves]').getAttribute('data-moves'); return (m ? m.split(' ').length : 0) > n; }, moves.length, { timeout: 20000 }).catch(() => {});
    }
    throw new Error('the level did not finish');
  }

  // ---- level 100 as White
  await p.click('button:has-text("Start your first attempt")');
  await p.waitForSelector('[data-level="100"]');
  ok('attempt 1 starts against Rüde 100');
  // a visitor can watch the game
  const v = await (await mk()).newPage(); v.on('pageerror', (e) => errors.push(String(e)));
  await p.waitForFunction(() => document.querySelector('[data-moves]')?.getAttribute('data-moves'), null, { timeout: 30000 }).catch(() => {});
    await v.goto(B + '/watch', { waitUntil: 'networkidle' });
  await v.waitForSelector(`section[aria-labelledby=botlive] a:has-text("${name}")`, { timeout: 30000 });
  assert((await v.locator(`section[aria-labelledby=botlive] li:has-text("${name}")`).first().textContent()).includes('🐺'));
  ok('a visitor sees the game on the Watch tab, marked with the wolf');
  await playLevel('white');
  await p.waitForSelector('text=You beat Rüde 100!', { timeout: 120000 });
  ok('Rüde 100 beaten: the server checked the game and moved me up');

  // ---- level 200 as Black
  await p.click('button:has-text("Face Rüde 200")');
  await p.waitForSelector('[data-level="200"]');
  await playLevel('black');
  await p.waitForSelector('text=You beat Rüde 200!', { timeout: 150000 });
  ok('Rüde 200 beaten as Black');

  // ---- level 300: resign ends the attempt
  await p.click('button:has-text("Face Rüde 300")');
  await p.waitForSelector('[data-level="300"]');
  await p.click('button:has-text("Resign")');
  await p.waitForSelector('text=Attempt over');
  await p.waitForSelector('text=This attempt: Rüde 200');
  ok('resigning ends the attempt and keeps the best level (Rüde 200)');
  sf.quit();
  await p.click('button:text-is("Back to the ladder")');
  await p.waitForSelector('text=Attempts used: 1 of 8');
  await p.waitForSelector('text=your best: Rüde 200');
  ok('the lobby shows 1 of 8 attempts used and a best of Rüde 200');

  // ---- use up all 8 attempts
  for (let n = 2; n <= 8; n++) {
    await p.click(`button:has-text("Start attempt ${n} of 8")`);
    await p.waitForSelector('[data-level="100"]');
    await p.click('button:has-text("Resign")');
    await p.waitForSelector('text=Attempt over');
    await p.click('button:text-is("Back to the ladder")');
  }
  await p.waitForSelector('text=used all 8 attempts');
  assert.equal(await p.locator('button:has-text("Start attempt")').count(), 0);
  ok('after 8 attempts there is no Start button any more');

  // ---- the leaderboard (a visitor sees it too) and the badges
  await v.goto(B + '/play/rudenrache', { waitUntil: 'networkidle' });
  const row = v.locator(`table tr:has-text("${name}")`);
  await row.waitFor({ timeout: 15000 });
  const text = (await row.textContent()).replace(/\s+/g, ' ');
  assert(/Rüde 200/.test(text) && /8 \/ 8/.test(text), text);
  ok('the leaderboard shows me with Rüde 200 and 8 / 8 attempts, visible to everyone: ' + text.trim());
  await p.goto(B + '/u/' + name, { waitUntil: 'networkidle' });
  await p.waitForSelector('li:has-text("Holiday hunter") >> text=Earned', { timeout: 30000 });
  await p.waitForSelector('li:has-text("Wolf bait") >> text=Earned');
  assert((await p.locator('li:has-text("Holiday hunter")').textContent()).includes('Limited'));
  assert((await p.locator('li:has-text("Pack leader")').textContent()).includes('Beat Rüde 500'));
  ok('the limited-edition badges "Holiday hunter" and "Wolf bait" are earned; "Pack leader" (500) is not yet');
  assert.deepEqual(errors, [], errors.join('\n'));
  await browser.close();
  console.log('RUDE CHALLENGE PASSED (' + name + ')');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
