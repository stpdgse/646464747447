// Browser end-to-end test for play-vs-computer. Needs a running server and Playwright (see board.cjs).
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const SP = process.argv[2] || '/tmp';
let step = 0;
const ok = (m) => console.log('OK  ', ++step, m);

(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1280, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  p.on('requestfailed', (r) => { if (!r.url().includes('_rsc=')) errors.push('request failed: ' + r.url()); }); // prefetches cancelled by navigation are normal

  const sq = async (name, orient = 'white') => {
    const box = await p.locator('.cg-wrap').boundingBox();
    const f = name.charCodeAt(0) - 97, r = Number(name[1]);
    const col = orient === 'white' ? f : 7 - f, row = orient === 'white' ? 8 - r : r - 1;
    return { x: box.x + (col + 0.5) * box.width / 8, y: box.y + (row + 0.5) * box.height / 8 };
  };
  const click = async (n, o) => { const s = await sq(n, o); await p.mouse.click(s.x, s.y); await p.waitForTimeout(250); };
  const moveCount = async () => p.locator('ol[aria-label=Moves] button').count().catch(() => 0);
  const waitMoves = (n, ms = 20000) => p.waitForFunction((n) => document.querySelectorAll('ol[aria-label=Moves] button').length >= n, n, { timeout: ms });
  const play = async () => { await p.waitForFunction(() => { const b = [...document.querySelectorAll('button')].find((x) => x.textContent === 'Play'); return b && !b.disabled; }, null, { timeout: 30000 }); await p.click('button:text-is("Play")'); };

  await p.goto(B + '/play/computer');
  await p.waitForSelector('[role=radiogroup]');
  assert((await p.locator('[role=radio]').count()) >= 25); ok('all bots listed');

  // --- game 1: white vs Pawnie, unlimited
  await p.click('[role=radio]:has-text("Pawnie")');
  await p.selectOption('select >> nth=0', 'white');
  const t0 = Date.now();
  await play(); ok('engine loaded in the browser (' + (Date.now() - t0) + ' ms to enable Play)');
  await p.waitForSelector('.cg-wrap piece');
  await click('e2'); await click('e4');
  await waitMoves(2); ok('Pawnie replied to e4');
  assert.match(await p.locator('[aria-live=polite]').first().textContent(), /Your move/);

  // takeback undoes the reply and my move
  await p.click('button:text-is("Takeback")'); await p.waitForTimeout(400);
  assert.equal(await moveCount(), 0); ok('takeback undid both moves');
  await click('d2'); await click('d4'); await waitMoves(2); ok('can play on after a takeback');

  // draw offer is declined early in the game
  await p.click('button:text-is("Offer draw")');
  await p.waitForSelector('text=declines the draw offer'); ok('early draw offer declined');

  // resign -> result, rematch, analyse
  await p.click('button:text-is("Resign")');
  await p.waitForSelector('text=Pawnie wins'); ok('resign ends the game');
  await p.screenshot({ path: SP + '/computer-over.png' });
  await p.click('button:text-is("Analyse")');
  await p.waitForURL(B + '/play/board');
  await p.waitForSelector('text=Loaded your game');
  assert.ok((await p.locator('ol[aria-label=Moves] button').count()) >= 2); ok('analysis board received the game');

  // --- game 2: black vs Stockfish, bot moves first, timed 3+2
  await p.goto(B + '/play/computer'); await p.waitForSelector('[role=radiogroup]');
  await p.click('[role=radio]:has-text("Stockfish")');
  await p.selectOption('select >> nth=0', 'black');
  await p.selectOption('select >> nth=2', '3+2');
  await play();
  await waitMoves(1, 20000); ok('Stockfish (white) opened the game');
  await p.waitForSelector('[aria-label="You clock"]');
  const clockTxt = async () => (await p.locator('[aria-label="You clock"]').textContent()).trim();
  assert.equal(await clockTxt(), '3:00'); ok('3+2 clock shows 3:00');
  const fenMove = await p.locator('ol[aria-label=Moves] button').first().textContent();
  assert.match(fenMove, /^[a-hNBRQKO]/); ok('bot opening move is readable SAN: ' + fenMove);
  // play a reply (c7-c5 or e7-e5 are both legal vs most openings; try a few safe ones)
  const replies = [['e7', 'e5'], ['c7', 'c5'], ['d7', 'd5'], ['g8', 'f6'], ['e7', 'e6']];
  for (const [f, t] of replies) { await click(f, 'black'); await click(t, 'black'); if ((await moveCount()) >= 2) break; await p.keyboard.press('Escape'); }
  assert.ok((await moveCount()) >= 2); ok('played a reply as black');
  await waitMoves(3); ok('clock running after both sides moved');
  const c1 = await clockTxt(); await p.waitForTimeout(2200); const c2 = await clockTxt();
  console.log('     clock', c1, '->', c2);
  assert.notEqual(c1, c2); ok('your clock counts down');

  // --- game 3: Chess960 vs Cam
  await p.goto(B + '/play/computer'); await p.waitForSelector('[role=radiogroup]');
  await p.click('[role=radio]:has-text("Casual Cam")');
  await p.selectOption('select >> nth=0', 'white');
  await p.selectOption('select >> nth=1', 'chess960');
  await play(); await p.waitForSelector('.cg-wrap piece');
  const back = await p.evaluate(() => [...document.querySelectorAll('.cg-wrap piece.white:not(.ghost)')].length);
  assert.equal(back, 16); ok('Chess960 board has 16 white pieces');
  // find a legal opening move by trying pawn pushes
  for (const f of 'abcdefgh') { await click(f + '2'); await click(f + '4'); if ((await moveCount()) >= 1) break; }
  await waitMoves(2); ok('Chess960: bot answered a legal move');

  await p.screenshot({ path: SP + '/computer-960.png' });
  const real = errors.filter((e) => !/favicon/.test(e));
  assert.equal(real.length, 0, 'errors: ' + real.join(' | ')); ok('no console errors or failed requests');
  await b.close();
})().catch((e) => { console.error('FAIL', e.message); process.exit(1); });
