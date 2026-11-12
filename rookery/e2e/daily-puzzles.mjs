// Daily games (two players), notifications, rated puzzles and Puzzle Rush against the real Supabase project.
// Needs a running server with GAME_SERVER_SECRET set. Creates throwaway accounts e2ea*/e2eb*.
// Run with: node e2e/daily-puzzles.mjs /tmp
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
const solver = (z) => z.moves.filter((_, i) => i % 2 === 1);
function wrongMove(z) {
  const pos = Chess.fromSetup(parseFen(z.fen).unwrap()).unwrap();
  pos.play(parseUci(z.moves[0]));
  for (const [from, tos] of pos.allDests()) for (const to of tos) {
    if (pos.board.get(from).role === 'pawn' && (squareRank(to) === 0 || squareRank(to) === 7)) continue;
    const u = makeSquare(from) + makeSquare(to);
    if (u === z.moves[1]) continue;
    const next = pos.clone(); next.play({ from, to }); if (next.isCheckmate()) continue;
    return u;
  }
  throw new Error('no wrong move');
}
let n = 0; const ok = (m) => console.log('OK  ', ++n, m);
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));

async function signup(ctx, name) {
  const p = await ctx.newPage();
  await p.goto(B + '/signup'); await p.fill('#username', name); await p.fill('#password', 'e2e-test-password-1');
  await p.click('button[type=submit]'); await p.waitForURL(B + '/', { timeout: 30000 });
  return p;
}
const orient = (p) => p.evaluate(() => (document.querySelector('.cg-wrap')?.classList.contains('orientation-black') ? 'black' : 'white'));
async function sq(p, name) {
  const o = await orient(p); const box = await p.locator('.cg-wrap').boundingBox();
  const f = name.charCodeAt(0) - 97, r = Number(name[1]); const col = o === 'white' ? f : 7 - f, row = o === 'white' ? 8 - r : r - 1;
  return { x: box.x + (col + 0.5) * box.width / 8, y: box.y + (row + 0.5) * box.height / 8 };
}
async function play(p, uci) {
  for (const s of [uci.slice(0, 2), uci.slice(2, 4)]) { const q = await sq(p, s); await p.mouse.click(q.x, q.y); await p.waitForTimeout(230); }
  if (uci.length === 5) { const names = { q: 'Queen', r: 'Rook', b: 'Bishop', n: 'Knight' }; await p.click(`button[aria-label=${names[uci[4]]}]`); }
}
const moves = (p) => p.locator('ol[aria-label=Moves] button').allTextContents().catch(() => []);
const waitMoves = (p, k, ms = 20000) => p.waitForFunction((k) => document.querySelectorAll('ol[aria-label=Moves] button').length >= k, k, { timeout: ms });
const unread = async (p) => { const l = await p.locator('header button[aria-label*="notification"]').getAttribute('aria-label'); const m = /(\d+) unread/.exec(l || ''); return m ? Number(m[1]) : 0; };

(async () => {
  const browser = await chromium.launch();
  const A = 'e2ea' + rnd(), Bn = 'e2eb' + rnd();
  const ctxA = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const ctxB = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const errors = [];
  const watch = (p, w) => { p.on('pageerror', (e) => errors.push(w + ': ' + e.message)); p.on('console', (m) => m.type() === 'error' && !/favicon|Failed to load resource|WebSocket|realtime/i.test(m.text()) && errors.push(w + ': ' + m.text())); };
  const pa = await signup(ctxA, A); watch(pa, 'A');
  const pb = await signup(ctxB, Bn); watch(pb, 'B');
  ok(`signed up ${A} and ${Bn}`);

  // ---------------------------------------------------------------- daily game
  await pa.goto(B + '/play/daily'); await pb.goto(B + '/play/daily');
  await pa.waitForSelector('h2:text-is("Start a daily game")');
  await pa.click('button:text-is("Post an open offer")');
  await pa.waitForSelector('text=Offer posted'); ok('A posts an open daily offer');
  await pb.waitForSelector(`li:has-text("${A}") button:text-is("Play")`, { timeout: 15000 }); ok('B sees the offer under Open daily offers');
  assert.ok(await pb.locator(`li:has-text("${A}")`).textContent().then((t) => /3 days per move/.test(t))); ok('offer shows 3 days per move');
  await pb.click(`li:has-text("${A}") button:text-is("Play")`);
  await pb.waitForURL(/\/game\//, { timeout: 15000 });
  await pb.waitForSelector('.cg-wrap piece');
  const gameUrl = pb.url();
  assert.match(await pb.locator('[aria-live=polite]').first().textContent(), /Daily, 3 days per move/); ok('game is labelled Daily, 3 days per move');
  assert.equal(await pb.locator('[aria-label$="clock"]').count(), 0); ok('daily games have no ticking clock');
  const note = await pb.locator('[aria-label$="deadline"]').first().textContent(); assert.match(note, /(2d 2\d|3d 0)h left/); ok('deadline shown for the side to move: ' + note);

  // A joins the game from "Your games"
  await pa.goto(B + '/play/daily');
  await pa.waitForSelector('h2:has-text("Your games")'); await pa.waitForSelector(`a:has-text("vs ${Bn}")`, { timeout: 15000 }); ok('A sees the new game under Your games');
  await pa.click(`a:has-text("vs ${Bn}")`); await pa.waitForURL(gameUrl); await pa.waitForSelector('.cg-wrap piece');
  const [oa] = [await orient(pa)];
  const [W, Bk, wName] = oa === 'white' ? [pa, pb, A] : [pb, pa, Bn];

  // notification for white (game started) shown in the bell
  await W.goto(B + '/play/daily'); await W.waitForSelector('header button[aria-label*="notification"]', { timeout: 15000 });
  await W.waitForFunction(() => /unread/.test(document.querySelector('header button[aria-label*="notification"]')?.getAttribute('aria-label') || ''), null, { timeout: 15000 });
  ok('white has an unread "your move" notification in the bell');
  await W.click('header button[aria-label*="notification"]'); await W.waitForSelector('text=Your move against');
  await W.screenshot({ path: SP + '/daily-bell.png' });
  await W.goto(gameUrl); await W.waitForSelector('.cg-wrap piece'); await Bk.waitForSelector('.cg-wrap piece');

  await play(W, 'e2e4'); await waitMoves(Bk, 1, 15000); ok('black sees white\'s daily move');
  await Bk.goto(B + '/'); await Bk.waitForFunction(() => /unread/.test(document.querySelector('header button[aria-label*="notification"]')?.getAttribute('aria-label') || ''), null, { timeout: 15000 });
  ok('black is notified that it is their move');
  await Bk.goto(gameUrl); await Bk.waitForSelector('.cg-wrap piece');
  await play(Bk, 'e7e5'); await waitMoves(W, 2, 15000);
  await W.click('button:text-is("Offer draw")'); await Bk.waitForSelector('text=offers a draw', { timeout: 15000 });
  await Bk.click('button:text-is("Accept")');
  await Promise.all([W.waitForSelector('text=by agreement', { timeout: 15000 }), Bk.waitForSelector('text=by agreement', { timeout: 15000 })]);
  await W.waitForSelector('[aria-label="Rating changes"] li', { timeout: 10000 });
  assert.match(await W.locator('[aria-label="Rating changes"]').textContent(), /1500 → 1500/); ok('rated daily draw: both stay at 1500 (a draw between equals)');
  await W.screenshot({ path: SP + '/daily-over.png' });

  await pa.goto(B + '/u/' + A); await pa.waitForSelector('h2:text-is("Ratings")');
  assert.ok(await pa.locator('p:text-is("Daily")').count() === 1); ok('profile shows a separate Daily rating');
  await pa.goto(B + '/leaderboard'); await pa.click('button[role=tab]:text-is("Daily")'); await pa.waitForSelector(`table >> text=${A}`, { timeout: 10000 }); ok('Daily leaderboard lists the player');

  // ---------------------------------------------------------------- rated puzzles (A)
  await pa.goto(B + '/puzzles/train');
  const pid = async (p) => { await p.waitForSelector('[data-puzzle-id][data-phase=solving]', { timeout: 15000 }); return p.getAttribute('[data-puzzle-id]', 'data-puzzle-id'); };
  let id = await pid(pa); let pz = byId.get(id);
  await pa.waitForTimeout(500);
  assert.match(await pa.locator('[aria-live=polite]').first().textContent(), /Rated/); ok('signed-in puzzles are rated');
  for (const mv of solver(pz)) { await play(pa, mv); await pa.waitForTimeout(900); }
  await pa.waitForSelector('text=Puzzle solved!', { timeout: 15000 });
  await pa.waitForSelector('[aria-label="Rating change"]', { timeout: 10000 });
  const up = await pa.locator('[aria-label="Rating change"]').textContent();
  assert.match(up, /Puzzle rating 1500 → 1\d\d\d \(\+\d+\)/); ok('solving raises the puzzle rating: ' + up.trim());
  await pa.screenshot({ path: SP + '/puzzle-rated.png' });

  await pa.click('button:text-is("Next puzzle")'); id = await pid(pa); pz = byId.get(id);
  await pa.waitForTimeout(500);
  await play(pa, wrongMove(pz)); await pa.waitForSelector('text=not the move'); ok('a wrong move shows a hint and snaps back');
  await pa.click('button:text-is("Show solution")'); await pa.waitForSelector('text=Puzzle finished', { timeout: 25000 });
  await pa.waitForSelector('[aria-label="Rating change"]', { timeout: 10000 });
  const down = await pa.locator('[aria-label="Rating change"]').textContent();
  assert.match(down, /−\d+\)/); ok('a miss lowers it: ' + down.trim());

  await pa.goto(B + '/puzzles');
  await pa.waitForSelector('text=Your puzzles'); assert.ok(await pa.locator('text=Solved').count() > 0 && await pa.locator('text=Missed').count() > 0); ok('puzzle home lists solved and missed puzzles');

  // a puzzle only counts once: reload the same one and solve again -> no rating change
  await pa.goto(B + '/puzzles/daily'); id = await pid(pa); pz = byId.get(id);
  for (const mv of solver(pz)) { await play(pa, mv); await pa.waitForTimeout(900); }
  await pa.waitForSelector('text=Puzzle solved!'); await pa.waitForSelector('[aria-label="Rating change"]', { timeout: 10000 }); ok('daily puzzle can be rated');
  await pa.goto(B + '/puzzles/daily'); await pa.waitForSelector('[data-puzzle-id]');
  assert.ok((await pa.textContent('body')).includes("You've already done it")); ok('the daily puzzle remembers you did it');
  await pa.goto(B + '/u/' + A); await pa.waitForSelector('p:text-is("Puzzles")'); assert.ok(await pa.locator('role=tab[name="Puzzles"]').count() === 1); ok('profile shows a Puzzles rating and history tab');

  // ---------------------------------------------------------------- Puzzle Rush (signed in)
  await pa.goto(B + '/puzzles/rush'); await pa.click('button:text-is("Start")'); await pa.waitForSelector('[data-puzzle-id]', { timeout: 15000 });
  for (let i = 0; i < 2; i++) { await pa.waitForTimeout(900); id = await pa.getAttribute('[data-puzzle-id]', 'data-puzzle-id'); pz = byId.get(id); for (const mv of solver(pz)) { await play(pa, mv); await pa.waitForTimeout(550); } await pa.waitForTimeout(700); }
  for (let i = 0; i < 3; i++) { await pa.waitForTimeout(900); id = await pa.getAttribute('[data-puzzle-id]', 'data-puzzle-id'); pz = byId.get(id); await play(pa, wrongMove(pz)); await pa.waitForTimeout(1000); }
  await pa.waitForSelector("text=Time's up", { timeout: 15000 });
  await pa.waitForSelector('text=Score saved', { timeout: 15000 }); ok('a verified Rush score is saved');
  await pa.goto(B + '/leaderboard'); await pa.waitForSelector('h2:text-is("Puzzle Rush: best scores")'); await pa.waitForSelector(`ol >> text=${A}`); ok('Rush score appears on the leaderboard');
  await pa.screenshot({ path: SP + '/leaderboard-rush.png', fullPage: true });

  const real = errors.filter((e) => !/Failed to load resource/.test(e));
  assert.equal(real.length, 0, 'console errors: ' + real.join(' | ')); ok('no console errors');
  console.log('\nTest accounts created:', A, Bn);
  await browser.close();
})().catch((e) => { console.error('FAIL', e.message); process.exit(1); });
