// Puzzle test that needs no account: solves puzzles by looking the answer up in the dataset. Needs a running server.
// Run with: node e2e/puzzles-anon.mjs /tmp
import { createRequire } from 'node:module';
import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseFen } from 'chessops/fen';
import { Chess } from 'chessops/chess';
import { parseUci, makeSquare, squareRank } from 'chessops/util';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const B = process.env.BASE || 'http://localhost:3111';
const SP = process.argv[2] || '/tmp';
const DATA = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'src', 'data', 'puzzles.json'), 'utf8'));
const byId = new Map(DATA.map((r) => [r[0], { id: r[0], fen: r[1], moves: r[2].split(' '), rating: r[3], themes: r[5].split(' ') }]));
/** A legal move that is not the solution and does not checkmate (so it is certainly a mistake). */
function wrongMove(z, fen) {
  const pos = Chess.fromSetup(parseFen(fen).unwrap()).unwrap();
  pos.play(parseUci(z.moves[0]));
  for (const [from, tos] of pos.allDests()) {
    for (const to of tos) {
      const piece = pos.board.get(from);
      if (piece.role === 'pawn' && (squareRank(to) === 0 || squareRank(to) === 7)) continue;
      const u = makeSquare(from) + makeSquare(to);
      if (u === z.moves[1]) continue;
      const next = pos.clone(); next.play({ from, to });
      if (next.isCheckmate()) continue;
      return u;
    }
  }
  throw new Error('no wrong move found');
}
let n = 0; const ok = (m) => console.log('OK  ', ++n, m);

(async () => {
  const browser = await chromium.launch();
  const p = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && !/favicon/.test(m.text()) && errors.push(m.text()));

  async function boxFor(orient) { const b = await p.locator('.cg-wrap').boundingBox(); return { b, orient }; }
  const orient = () => p.evaluate(() => (document.querySelector('.cg-wrap')?.classList.contains('orientation-black') ? 'black' : 'white'));
  async function sq(name) {
    const o = await orient(); const box = await p.locator('.cg-wrap').boundingBox();
    const f = name.charCodeAt(0) - 97, r = Number(name[1]); const col = o === 'white' ? f : 7 - f, row = o === 'white' ? 8 - r : r - 1;
    return { x: box.x + (col + 0.5) * box.width / 8, y: box.y + (row + 0.5) * box.height / 8 };
  }
  async function play(uci) {
    for (const s of [uci.slice(0, 2), uci.slice(2, 4)]) { const q = await sq(s); await p.mouse.click(q.x, q.y); await p.waitForTimeout(220); }
    if (uci.length === 5) { const names = { q: 'Queen', r: 'Rook', b: 'Bishop', n: 'Knight' }; await p.click(`button[aria-label=${names[uci[4]]}]`); }
  }
  const phase = () => p.getAttribute('[data-puzzle-id]', 'data-phase');
  const puzzleId = async () => { await p.waitForSelector('[data-puzzle-id][data-phase=solving]', { timeout: 15000 }); return p.getAttribute('[data-puzzle-id]', 'data-puzzle-id'); };

  // ---- trainer, signed out
  await p.goto(B + '/puzzles/train');
  let id = await puzzleId(); let pz = byId.get(id);
  assert.ok(pz, 'puzzle exists in dataset'); ok(`a puzzle loads (id ${id}, rating ${pz.rating})`);
  const solverMoves = (z) => z.moves.filter((_, i) => i % 2 === 1);

  // wrong move first: snaps back and shows a message
  await p.waitForTimeout(700); // let the opening move's animation finish (captured pieces fade out)
  const fen0 = await p.locator('.cg-wrap piece:not(.ghost)').count();
  const oppSq = pz.moves[0].slice(2, 4);
  await play(oppSq + (oppSq === 'a1' ? 'a2' : 'a1')); await p.waitForTimeout(300); // the opponent's piece cannot be moved by the solver
  assert.ok((await p.locator('.cg-wrap piece:not(.ghost)').count()) === fen0); ok('an illegal/wrong move does nothing');

  for (const mv of solverMoves(pz)) { await play(mv); await p.waitForTimeout(900); }
  await p.waitForSelector('text=Puzzle solved!', { timeout: 15000 }); ok('playing the real solution solves the puzzle');
  assert.ok(await p.locator('text=Log in').count() > 0); ok('signed-out players are invited to log in for ratings');
  await p.screenshot({ path: SP + '/puzzle-solved.png' });

  // next puzzle with a theme filter
  await p.selectOption('select', 'mateIn1');
  id = await puzzleId(); pz = byId.get(id);
  assert.ok(pz.themes.includes('mateIn1')); ok('theme filter returns a mate-in-1 puzzle');
  // wrong move that is legal but incorrect: strike -> still finishable, shows message
  await p.click('button:text-is("Show solution")');
  await p.waitForSelector('text=Puzzle finished', { timeout: 20000 }); ok('"Show solution" plays it out and marks the puzzle finished');

  // skip and analyse
  await p.click('button:text-is("Next puzzle")'); id = await puzzleId();
  await p.click('button:text-is("Skip")'); await p.waitForTimeout(300); ok('skip loads another puzzle');
  await puzzleId();

  // ---- daily puzzle: same for everyone, stable across loads
  await p.goto(B + '/puzzles/daily'); const d1 = await puzzleId();
  const p2 = await browser.newPage(); await p2.goto(B + '/puzzles/daily'); await p2.waitForSelector('[data-puzzle-id][data-phase=solving]');
  const d2 = await p2.getAttribute('[data-puzzle-id]', 'data-puzzle-id');
  assert.equal(d1, d2); ok('the daily puzzle is the same for everyone');
  assert.ok((await p.textContent('body')).includes('Everyone gets the same one')); ok('daily page explains itself');

  // ---- puzzle rush, signed out
  await p.goto(B + '/puzzles/rush'); await p.waitForSelector('button:text-is("Start")');
  await p.click('button:text-is("Start")');
  await p.waitForSelector('[data-puzzle-id]', { timeout: 15000 });
  let solved = 0;
  for (let i = 0; i < 3; i++) {
    await p.waitForTimeout(700);
    id = await p.getAttribute('[data-puzzle-id]', 'data-puzzle-id'); pz = byId.get(id);
    for (const mv of solverMoves(pz)) { await play(mv); await p.waitForTimeout(550); }
    solved++; await p.waitForTimeout(700);
  }
  const scoreText = await p.locator('text=Solved').locator('xpath=..').textContent();
  assert.match(scoreText, /3/); ok('rush counts solved puzzles (score 3)');
  // three strikes ends the run
  for (let i = 0; i < 3; i++) {
    await p.waitForTimeout(900);
    const strikesBefore = await p.locator('[aria-label$="of 3 strikes"]').getAttribute('aria-label');
    id = await p.getAttribute('[data-puzzle-id]', 'data-puzzle-id'); pz = byId.get(id);
    await play(wrongMove(pz, pz.fen)); await p.waitForTimeout(1000);
  }
  await p.waitForSelector('text=Time\'s up', { timeout: 15000 }); ok('three strikes end the run');
  const final = await p.locator('p.text-6xl').textContent(); assert.equal(final.trim(), '3'); ok('final score shown');
  assert.ok(await p.locator('text=Log in').count() > 0); ok('signed-out score is not saved (invited to log in)');
  await p.screenshot({ path: SP + '/rush-over.png' });

  const real = errors.filter((e) => !/Failed to load resource/.test(e));
  assert.equal(real.length, 0, 'console errors: ' + real.join(' | ')); ok('no console errors');
  await browser.close();
})().catch((e) => { console.error('FAIL', e.message); process.exit(1); });
