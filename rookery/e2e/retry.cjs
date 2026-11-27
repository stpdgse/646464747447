// A legal but wrong move must not freeze the board: you can try again straight away. Needs a production server on :3111.
const { chromium } = require('playwright');
const assert = require('assert');
const fs = require('fs');
const path = require('path');
const B = process.env.BASE || 'http://localhost:3111';
const DATA = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'src', 'data', 'puzzles.json'), 'utf8'));
const byId = new Map(DATA.map((r) => [r[0], { id: r[0], fen: r[1], moves: r[2].split(' ') }]));
const ok = (m) => console.log('OK  ', m);

(async () => {
  const { parseFen } = await import('chessops/fen');
  const { Chess } = await import('chessops/chess');
  const { parseUci, makeSquare, squareRank } = await import('chessops/util');
  /** A legal move that is not the solution and does not checkmate, so it is certainly a mistake. */
  const wrongMove = (z) => {
    const pos = Chess.fromSetup(parseFen(z.fen).unwrap()).unwrap();
    pos.play(parseUci(z.moves[0]));
    for (const [from, tos] of pos.allDests()) for (const to of tos) {
      const piece = pos.board.get(from);
      if (piece.role === 'pawn' && (squareRank(to) === 0 || squareRank(to) === 7)) continue;
      const u = makeSquare(from) + makeSquare(to);
      if (u === z.moves[1]) continue;
      const next = pos.clone(); next.play({ from, to });
      if (next.isCheckmate()) continue;
      return u;
    }
    throw new Error('no wrong move found');
  };

  const browser = await chromium.launch();
  const errors = [];
  const p = await browser.newPage({ viewport: { width: 1280, height: 1000 } });
  p.on('pageerror', (e) => errors.push(String(e)));
  const sq = async (name) => {
    const box = await p.locator('.cg-wrap').boundingBox();
    const black = await p.evaluate(() => document.querySelector('.cg-wrap').classList.contains('orientation-black'));
    const f = name.charCodeAt(0) - 97, r = Number(name[1]);
    const col = black ? 7 - f : f, row = black ? r - 1 : 8 - r;
    return { x: box.x + (col + 0.5) * box.width / 8, y: box.y + (row + 0.5) * box.height / 8 };
  };
  const play = async (uci) => {
    for (const s of [uci.slice(0, 2), uci.slice(2, 4)]) { const q = await sq(s); await p.mouse.click(q.x, q.y); await p.waitForTimeout(250); }
    if (uci.length === 5) await p.click('button[aria-label=Queen]');
  };

  // ---- puzzle trainer: wrong move, then the right one at once (no hint, no reload)
  await p.goto(B + '/puzzles/train');
  await p.waitForSelector('[data-puzzle-id][data-phase=solving]', { timeout: 20000 });
  await p.waitForTimeout(800);
  let z = byId.get(await p.getAttribute('[data-puzzle-id]', 'data-puzzle-id'));
  await play(wrongMove(z));
  await p.waitForSelector('text=not the move');
  ok('a legal but wrong puzzle move is refused with a message');
  await p.waitForTimeout(400);
  const sol = z.moves.filter((_, i) => i % 2 === 1);
  for (const mv of sol) { await play(mv); await p.waitForTimeout(900); }
  await p.waitForSelector('text=Puzzle finished', { timeout: 15000 });
  ok('the board still accepts moves after a wrong one: the puzzle can be finished');

  // ---- the same puzzle again, from the start
  await p.click('button:text-is("Try this puzzle again")');
  await p.waitForSelector('[data-phase=solving]', { timeout: 10000 });
  assert.equal(await p.getAttribute('[data-puzzle-id]', 'data-puzzle-id'), z.id);
  await p.waitForTimeout(500);
  for (const mv of sol) { await play(mv); await p.waitForTimeout(900); }
  await p.waitForSelector('text=Puzzle solved!', { timeout: 15000 });
  ok('"Try this puzzle again" restarts the same puzzle and it can be solved cleanly');

  // ---- lesson: wrong move, then the right one at once
  await p.goto(B + '/learn/lessons/basics', { waitUntil: 'networkidle' });
  await p.click('button:has-text("Start the exercises")');
  await p.waitForSelector('[data-phase=solving]');
  await play('d4d5');
  await p.waitForSelector('text=Not quite');
  await play('d4d7');
  await p.waitForSelector('text=Correct!', { timeout: 10000 });
  ok('lesson: the right move works straight after a wrong one');
  await p.click('button:text-is("Try again")');
  await p.waitForSelector('[data-phase=solving]');
  await play('d4d7');
  await p.waitForSelector('text=Correct!', { timeout: 10000 });
  ok('lesson: "Try again" replays an exercise');

  assert.deepEqual(errors, [], errors.join('\n'));
  await browser.close();
  console.log('RETRY PASSED');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
