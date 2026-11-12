// Two-player end-to-end test for live games against the real Supabase project.
// Needs a running server (npm run build && npm start -- -p 3111) with GAME_SERVER_SECRET set, and Playwright.
// Creates two throwaway accounts named e2eaNNNNN / e2ebNNNNN. Delete them afterwards (see README).
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const SP = process.argv[2] || '/tmp';
let n = 0;
const ok = (m) => console.log('OK  ', ++n, m);
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));

async function signup(ctx, name) {
  const p = await ctx.newPage();
  await p.goto(B + '/signup');
  await p.fill('#username', name); await p.fill('#password', 'e2e-test-password-1');
  await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });
  return p;
}
const orient = (p) => p.evaluate(() => (document.querySelector('.cg-wrap')?.classList.contains('orientation-black') ? 'black' : 'white'));
async function sq(p, name) {
  const o = await orient(p);
  const box = await p.locator('.cg-wrap').boundingBox();
  const f = name.charCodeAt(0) - 97, r = Number(name[1]);
  const col = o === 'white' ? f : 7 - f, row = o === 'white' ? 8 - r : r - 1;
  return { x: box.x + (col + 0.5) * box.width / 8, y: box.y + (row + 0.5) * box.height / 8 };
}
async function move(p, from, to) {
  for (const s of [from, to]) { const q = await sq(p, s); await p.mouse.click(q.x, q.y); await p.waitForTimeout(250); }
}
const moves = (p) => p.locator('ol[aria-label=Moves] button').allTextContents().catch(() => []);
const waitMoves = (p, k, ms = 15000) => p.waitForFunction((k) => document.querySelectorAll('ol[aria-label=Moves] button').length >= k, k, { timeout: ms });
const status = async (p) => (await p.locator('[aria-live=polite]').first().textContent()).trim();

(async () => {
  const browser = await chromium.launch();
  const A = 'e2ea' + rnd(), Bn = 'e2eb' + rnd();
  const ctxA = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const ctxB = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const errors = [];
  const watch = (p, who) => { p.on('pageerror', (e) => errors.push(who + ': ' + e.message)); p.on('console', (m) => m.type() === 'error' && !/favicon|WebSocket|realtime/i.test(m.text()) && errors.push(who + ': ' + m.text())); };

  const pa = await signup(ctxA, A); watch(pa, 'A');
  const pb = await signup(ctxB, Bn); watch(pb, 'B');
  ok(`signed up ${A} and ${Bn}`);

  // ---------------------------------------------------------------- game 1: quick pair, realtime
  await pa.goto(B + '/play/online'); await pb.goto(B + '/play/online');
  await pa.waitForSelector('button:text-is("3 + 2")'); await pb.waitForSelector('button:text-is("3 + 2")');
  await pa.click('button:text-is("3 + 2")');
  await pa.waitForSelector('text=Looking for an opponent'); ok('A is waiting in the queue');
  await pb.waitForSelector(`text=${A}`, { timeout: 10000 }); ok('B sees A in Open games');
  await pb.click('button:text-is("3 + 2")');
  await Promise.all([pa.waitForURL(/\/game\//, { timeout: 15000 }), pb.waitForURL(/\/game\//, { timeout: 15000 })]);
  assert.equal(pa.url(), pb.url()); ok('quick pairing put both players in the same game');
  await Promise.all([pa.waitForSelector('.cg-wrap piece'), pb.waitForSelector('.cg-wrap piece')]);
  const gameUrl = pa.url();
  const [oa, ob] = [await orient(pa), await orient(pb)];
  assert.notEqual(oa, ob); ok(`colours assigned: ${A}=${oa}, ${Bn}=${ob}`);
  const [W, Bk] = oa === 'white' ? [pa, pb] : [pb, pa];
  assert.match(await status(W), /Your move/); assert.match(await status(Bk), /Waiting for/); ok('turn indicators are right');
  assert.equal(await W.locator('[aria-label$="clock"]').count(), 2); ok('both clocks shown');

  // illegal move is refused client-side
  await move(W, 'a2', 'a5'); await W.waitForTimeout(400);
  assert.equal((await moves(W)).length, 0); ok('illegal move rejected');
  // wrong player cannot move
  await move(Bk, 'e7', 'e5'); await Bk.waitForTimeout(400);
  assert.equal((await moves(Bk)).length, 0); ok('cannot move out of turn');

  await move(W, 'e2', 'e4');
  await waitMoves(W, 1); ok('white\'s move appears for white at once');
  const t0 = Date.now(); await waitMoves(Bk, 1, 8000); ok(`black sees the move (${Date.now() - t0} ms)`);
  await move(Bk, 'e7', 'e5'); await waitMoves(W, 2, 8000); ok('white sees black\'s reply');
  await move(W, 'g1', 'f3'); await waitMoves(Bk, 3, 8000);
  await move(Bk, 'b8', 'c6'); await waitMoves(W, 4, 8000);

  // draw offer: decline, then accept
  await W.click('button:text-is("Offer draw")');
  await Bk.waitForSelector('text=offers a draw', { timeout: 8000 }); ok('draw offer reaches the opponent');
  await Bk.click('button:text-is("Decline")');
  await W.waitForSelector('button:text-is("Offer draw")', { timeout: 8000 }); ok('declined draw offer clears');
  await W.click('button:text-is("Offer draw")'); await Bk.waitForSelector('text=offers a draw');
  await Bk.click('button:text-is("Accept")');
  await Promise.all([W.waitForSelector('text=by agreement', { timeout: 8000 }), Bk.waitForSelector('text=by agreement', { timeout: 8000 })]);
  ok('accepted draw ends the game for both');
  await W.screenshot({ path: SP + '/live-draw.png' });

  // after the game ends, nothing more can be played
  await move(W, 'd2', 'd4'); await W.waitForTimeout(400);
  assert.equal((await moves(W)).length, 4); ok('finished game is frozen');

  // ---------------------------------------------------------------- game 2: challenge, WebSockets blocked, fool's mate
  for (const ctx of [ctxA, ctxB]) await ctx.routeWebSocket(/realtime/, (ws) => ws.close());
  await pa.goto(B + '/play/online'); await pb.goto(B + '/play/online');
  await pa.waitForSelector('#chal');
  await pa.fill('input[autocapitalize=none]', Bn);
  await pa.selectOption('select >> nth=0', '1+0');
  await pa.click('button:text-is("Send challenge")');
  await pa.waitForSelector('text=Challenge sent', { timeout: 10000 }); ok('challenge sent');
  await pb.waitForSelector('text=Challenges for you', { timeout: 10000 }); ok('challenge arrives for B (polling, WebSockets blocked)');
  await pb.click('button:text-is("Accept")');
  await Promise.all([pa.waitForURL(/\/game\//, { timeout: 15000 }), pb.waitForURL(/\/game\//, { timeout: 15000 })]);
  await Promise.all([pa.waitForSelector('.cg-wrap piece'), pb.waitForSelector('.cg-wrap piece')]);
  const game2 = pa.url(); assert.notEqual(game2, gameUrl); ok('challenge accepted: new game');
  await pa.waitForSelector('text=Live connection unavailable', { timeout: 12000 }); ok('app notices WebSockets are blocked and falls back to polling');
  const [W2, B2] = (await orient(pa)) === 'white' ? [pa, pb] : [pb, pa];
  const wName = W2 === pa ? A : Bn;
  const seq = [[W2, 'f2', 'f3'], [B2, 'e7', 'e5'], [W2, 'g2', 'g4'], [B2, 'd8', 'h4']];
  for (let i = 0; i < seq.length; i++) { await move(seq[i][0], seq[i][1], seq[i][2]); await waitMoves(W2, i + 1, 10000); await waitMoves(B2, i + 1, 10000); }
  ok('four moves relayed through polling');
  await Promise.all([W2.waitForSelector('text=You lose', { timeout: 10000 }), B2.waitForSelector('text=You win', { timeout: 10000 })]);
  assert.match(await W2.locator('[aria-live=polite]').first().textContent(), /by checkmate/); ok("fool's mate: checkmate decided on the server, both see the result");
  await W2.screenshot({ path: SP + '/live-mate.png' });

  // ---------------------------------------------------------------- game 3: rematch (1+0, colours swapped), flag fall, spectator
  await W2.click('button:text-is("Offer rematch")');
  await B2.goto(B + '/play/online');
  await B2.waitForSelector('text=Challenges for you', { timeout: 10000 }); ok('rematch offer appears for the opponent');
  await B2.click('button:text-is("Accept")');
  await Promise.all([W2.goto(B + '/play/online'), B2.waitForURL(/\/game\//, { timeout: 15000 })]);
  await W2.waitForSelector('text=Resume', { timeout: 10000 });
  await W2.click('a:text-is("Resume")'); await W2.waitForURL(/\/game\//);
  await Promise.all([W2.waitForSelector('.cg-wrap piece'), B2.waitForSelector('.cg-wrap piece')]);
  assert.notEqual(await orient(W2), 'white'); ok('rematch swaps colours');
  const [W3, B3] = [B2, W2];
  const spec = await browser.newPage();
  await spec.goto(B2.url()); await spec.waitForSelector('.cg-wrap piece');
  assert.match(await spec.locator('[aria-live=polite]').first().textContent(), /watching/); ok('spectator (signed out) can open the game');
  await move(W3, 'e2', 'e4'); await waitMoves(B3, 1, 10000); await move(B3, 'e7', 'e5'); await waitMoves(W3, 2, 10000);
  await waitMoves(spec, 2, 10000); ok('spectator sees the moves');
  await spec.goto(B + '/watch'); await spec.waitForSelector('text=Live now'); await spec.waitForSelector(`text=${Bn}`, { timeout: 8000 }); ok('game is listed on the Watch page');
  console.log('     waiting ~62 s for white\'s 1-minute clock to run out...');
  await Promise.all([W3.waitForSelector('text=on time', { timeout: 90000 }), B3.waitForSelector('text=on time', { timeout: 90000 })]);
  assert.match(await B3.locator('[aria-live=polite]').first().textContent(), /You win/); ok('flag fall: white loses on time, server decided it');
  await spec.screenshot({ path: SP + '/live-watch.png' });

  const real = errors.filter((e) => !/Failed to load resource.*(40[0-9]|net::)/i.test(e));
  assert.equal(real.length, 0, 'console errors: ' + real.join(' | ')); ok('no console errors');
  console.log('\nTest accounts created:', A, Bn);
  await browser.close();
})().catch((e) => { console.error('FAIL', e.message); process.exit(1); });
