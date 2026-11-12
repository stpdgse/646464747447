// Browser end-to-end test for the analysis board. Needs a running server and Playwright:
//   npm run build && npm start -- -p 3111   (in one terminal)
//   node e2e/board.cjs /tmp                  (screenshots go to the given folder)
// Set BASE to test another URL, e.g. BASE=https://your-site.vercel.app
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const SP = process.argv[2];
let step = 0;
const ok = (m) => console.log('OK  ', ++step, m);

(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1280, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));

  await p.goto(B + '/play/board');
  await p.waitForSelector('.cg-wrap piece');
  assert.equal(await p.locator('.cg-wrap piece:not(.ghost)').count(), 32); ok('board renders 32 pieces');

  const bb = await p.locator('.rk-board').boundingBox();
  const side = await p.locator('[aria-live=polite]').first().boundingBox();
  assert.ok(bb.x + bb.width <= side.x + 0.5, 'board overlaps side panel: ' + (bb.x + bb.width) + ' > ' + side.x); ok('desktop: board does not overlap side panel');
  const sq = async (name, orient = 'white') => {
    const box = await p.locator('.cg-wrap').boundingBox();
    const f = name.charCodeAt(0) - 97, r = Number(name[1]);
    const col = orient === 'white' ? f : 7 - f, row = orient === 'white' ? 8 - r : r - 1;
    return { x: box.x + (col + 0.5) * box.width / 8, y: box.y + (row + 0.5) * box.height / 8 };
  };
  const click = async (n, o) => { const s = await sq(n, o); await p.mouse.click(s.x, s.y); await p.waitForTimeout(250); };
  const drag = async (a, c, o) => {
    const s = await sq(a, o), t = await sq(c, o);
    await p.mouse.move(s.x, s.y); await p.mouse.down(); await p.mouse.move((s.x + t.x) / 2, (s.y + t.y) / 2, { steps: 4 });
    await p.mouse.move(t.x, t.y, { steps: 4 }); await p.mouse.up(); await p.waitForTimeout(300);
  };
  const fen = async () => (await p.locator('section[aria-label="Position (FEN)"] code').textContent()).trim();
  const moves = async () => (await p.locator('ol[aria-label=Moves] button').allTextContents());
  const status = async () => (await p.locator('[aria-live=polite]').first().textContent()).trim();

  // click-click move, then drag move
  await click('e2'); await click('e4');
  await p.waitForFunction(() => document.querySelector('ol[aria-label=Moves]'));
  assert.deepEqual(await moves(), ['e4']); ok('click-click move e2-e4');
  await drag('e7', 'e5');
  assert.deepEqual(await moves(), ['e4', 'e5']); ok('drag move e7-e5');
  assert.match(await status(), /White to move/);

  // illegal drag is ignored
  await drag('a2', 'a5');
  assert.deepEqual(await moves(), ['e4', 'e5']); ok('illegal move snaps back');

  // text input
  await p.fill('#move-text', 'Ke4'); await p.click('button:has-text("Play")');
  await p.waitForSelector('[role=alert]:has-text("not a legal move")'); ok('bad text move explained');
  await p.fill('#move-text', 'Nf3'); await p.click('button:has-text("Play")');
  await p.fill('#move-text', 'Nc6'); await p.click('button:has-text("Play")');
  await p.fill('#move-text', 'Bc4'); await p.click('button:has-text("Play")');
  await p.fill('#move-text', 'Nf6'); await p.click('button:has-text("Play")');
  assert.equal((await moves()).length, 6); ok('typed SAN moves');

  // castling by dragging the king two squares
  await drag('e1', 'g1');
  assert.equal((await moves()).at(-1), 'O-O');
  assert.match(await fen(), /RNBQ1RK1/); ok('castling by dragging king');
  await p.waitForTimeout(400);
  await p.screenshot({ path: SP + '/board-mid.png' });

  // keyboard navigation
  await p.locator('body').click({ position: { x: 5, y: 5 } });
  await p.keyboard.press('ArrowLeft');
  assert.match(await status(), /viewing move 6 of 7/); await p.keyboard.press('ArrowUp');
  assert.match(await status(), /viewing move 0 of 7/);
  assert.match(await fen(), /^rnbqkbnr\/pppppppp\/8\/8\/8\/8\/PPPPPPPP\/RNBQKBNR/); ok('arrow keys step through history');
  await p.keyboard.press('ArrowDown');

  // flip
  await p.keyboard.press('f'); await p.waitForTimeout(300);
  await p.keyboard.press('f'); ok('flip key');

  // promotion via FEN
  await p.fill('#fen', '8/P6k/8/8/8/8/8/K7 w - - 0 1'); await p.click('button:has-text("Load")');
  await click('a7'); await click('a8');
  await p.waitForSelector('[role=dialog]'); ok('promotion dialog opens');
  await p.screenshot({ path: SP + '/board-promo.png' });
  await p.click('button[aria-label=Knight]');
  assert.match(await fen(), /^N7\/7k/); ok('promoted to a knight');

  // promotion cancel snaps back
  await p.fill('#fen', '8/P6k/8/8/8/8/8/K7 w - - 0 1'); await p.click('button:has-text("Load")');
  await click('a7'); await click('a8'); await p.waitForSelector('[role=dialog]');
  await p.keyboard.press('Escape');
  await p.waitForSelector('[role=dialog]', { state: 'detached' });
  assert.match(await fen(), /^8\/P6k/); ok('cancelled promotion restores the pawn');

  // fool's mate -> checkmate banner
  await p.selectOption('#variant', 'standard'); await p.click('button:has-text("Start")');
  for (const m of ['f3', 'e5', 'g4', 'Qh4']) { await p.fill('#move-text', m); await p.click('button:has-text("Play")'); }
  assert.match(await status(), /Black wins by checkmate/); ok('checkmate detected');
  await p.screenshot({ path: SP + '/board-mate.png' });

  // chess960
  await p.selectOption('#variant', 'chess960'); await p.fill('#id960', '0'); await p.click('button:has-text("Start")');
  assert.match(await fen(), /^bbqnnrkr\/pppppppp\/8\/8\/8\/8\/PPPPPPPP\/BBQNNRKR w (KQkq|FDfd)/); ok('Chess960 position 0');
  await p.fill('#id960', '960'); await p.click('button:has-text("Start")');
  await p.waitForSelector('[role=alert]:has-text("0 to 959")'); ok('Chess960 range validated');
  await p.fill('#id960', ''); await p.click('button:has-text("Start")'); await p.waitForTimeout(500);
  assert.equal(await p.locator('.cg-wrap piece:not(.ghost)').count(), 32); ok('random Chess960 start');
  await p.screenshot({ path: SP + '/board-960.png' });

  // PGN import
  await p.selectOption('#variant', 'standard');
  await p.fill('#pgn', '[White "A"]\n\n1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 1/2-1/2');
  await p.click('button:has-text("Load PGN")');
  await p.waitForSelector('[role=status]:has-text("Loaded 10 moves")');
  assert.equal((await moves()).length, 10); ok('PGN import');
  await p.fill('#pgn', '1. e4 e5 2. Ke4'); await p.click('button:has-text("Load PGN")');
  await p.waitForSelector('[role=alert]:has-text("Ke4")'); ok('bad PGN explained');

  // coordinates toggle
  await p.click('button:has-text("Appearance")');
  assert.ok(await p.locator('.cg-wrap coords').count() > 0, 'coords present by default'); ok('coordinates shown by default');
  await p.locator('label:has-text("Show coordinates") input').uncheck();
  assert.equal(await p.locator('.cg-wrap coords').count(), 0); ok('coordinates can be hidden');
  await p.locator('label:has-text("Show coordinates") input').check();
  await p.click('button:has-text("Appearance")');

  // themes
  await p.click('button:has-text("Appearance")');
  await p.click('button[aria-label=Brown]');
  assert.equal(await p.locator('.rk-board').evaluate((e) => getComputedStyle(e).getPropertyValue('--sq-light').trim()), '#f0d9b5'); ok('board theme switches');
  await p.click('button[aria-label=Merida]');
  const bg = await p.locator('.cg-wrap piece.knight.white').first().evaluate((e) => getComputedStyle(e).backgroundImage);
  assert.match(bg, /pieces\/merida\/wN\.svg/); ok('piece set switches');
  await p.reload(); await p.waitForSelector('.cg-wrap piece');
  const bg2 = await p.locator('.cg-wrap piece.knight.white').first().evaluate((e) => getComputedStyle(e).backgroundImage);
  assert.match(bg2, /merida/); ok('theme persists after reload');
  await p.screenshot({ path: SP + '/board-themed.png' });

  // phone layout
  const m = await b.newPage({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true });
  await m.goto(B + '/play/board'); await m.waitForSelector('.cg-wrap piece');
  const box = await m.locator('.cg-wrap').boundingBox();
  assert.ok(box.width > 340 && Math.abs(box.width - box.height) < 2, `board square on phone: ${box.width}x${box.height}`); ok('phone: board is square and fills width');
  assert.ok(await m.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), 'no horizontal scroll'); ok('phone: no horizontal scroll');
  await m.screenshot({ path: SP + '/board-phone.png' });

  const real = errors.filter((e) => !/favicon/.test(e));
  assert.equal(real.length, 0, 'console errors: ' + real.join(' | ')); ok('no console errors');
  await b.close();
})().catch((e) => { console.error('FAIL', e.message); process.exit(1); });
