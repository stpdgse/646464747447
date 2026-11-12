// Game review, live engine and the opening explorer. Needs a running server (and the Supabase project for class stats).
// Run with: node e2e/analysis-openings.mjs /tmp
import { createRequire } from 'node:module';
import assert from 'node:assert';
const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const B = process.env.BASE || 'http://localhost:3111';
const SP = process.argv[2] || '/tmp';
let n = 0; const ok = (m) => console.log('OK  ', ++n, m);

(async () => {
  const browser = await chromium.launch();
  const p = await browser.newPage({ viewport: { width: 1280, height: 1000 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && !/favicon|Failed to load resource/.test(m.text()) && errors.push(m.text()));

  const orient = () => p.evaluate(() => (document.querySelector('.cg-wrap')?.classList.contains('orientation-black') ? 'black' : 'white'));
  async function sq(name) {
    const o = await orient(); const box = await p.locator('.cg-wrap').boundingBox();
    const f = name.charCodeAt(0) - 97, r = Number(name[1]); const col = o === 'white' ? f : 7 - f, row = o === 'white' ? 8 - r : r - 1;
    return { x: box.x + (col + 0.5) * box.width / 8, y: box.y + (row + 0.5) * box.height / 8 };
  }
  async function play(uci) { for (const s of [uci.slice(0, 2), uci.slice(2, 4)]) { const q = await sq(s); await p.mouse.click(q.x, q.y); await p.waitForTimeout(220); } }

  // ================================================================ analysis board
  await p.goto(B + '/play/board');
  await p.fill('#pgn', '[White "A"]\n[Black "B"]\n\n1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7# 1-0');
  await p.click('button:text-is("Load PGN")'); await p.waitForSelector('text=Loaded 7 moves');
  await p.click('button[aria-label="Last move"]');
  await p.waitForSelector('[aria-label=Opening]', { timeout: 10000 });
  const opening = await p.locator('[aria-label=Opening]').textContent(); ok('opening name shown for the loaded game: ' + opening.trim());
  assert.ok(await p.locator('button:text-is("Review game")').isEnabled());

  await p.click('button:text-is("Review game")');
  await p.waitForSelector('text=Analysing position', { timeout: 20000 }); ok('review starts and shows progress');
  await p.waitForSelector('[aria-label="Review summary"]', { timeout: 90000 }); ok('review completes');
  const wAcc = await p.locator('[aria-label="White accuracy"]').textContent(); const bAcc = await p.locator('[aria-label="Black accuracy"]').textContent();
  assert.ok(parseFloat(wAcc) > parseFloat(bAcc)); ok(`accuracy: White ${wAcc} vs Black ${bAcc} (Black blundered)`);
  const summary = await p.locator('[aria-label="Review summary"]').textContent();
  assert.match(summary, /1 Blunder|[1-9] Blunders?/); ok('summary counts at least one blunder');
  const nf6 = p.locator('ol[aria-label="Moves with review marks"] button[aria-label^="Nf6"]');
  assert.match(await nf6.getAttribute('aria-label'), /blunder/); assert.match(await nf6.textContent(), /\?\?/); ok('Nf6 is marked ?? in the move list');
  await nf6.click(); await p.waitForSelector('text=was a blunder', { timeout: 5000 }); ok('selecting it explains the mistake and the engine\'s preference');
  assert.match(await p.locator('[role=status]:has-text("was a blunder")').textContent(), /engine preferred/);
  assert.ok(await p.locator('svg[aria-label^="White\'s winning chances"]').count() === 1); ok('evaluation graph is shown');
  const svg = p.locator('svg[aria-label^="White\'s winning chances"]'); const box = await svg.boundingBox();
  await p.mouse.move(box.x + box.width * 0.5, box.y + 50); await p.waitForSelector('[role=status]:has-text("White ")'); ok('graph hover shows evaluation and move');
  await p.mouse.click(box.x + box.width * 0.2, box.y + 50); await p.waitForSelector('text=/viewing move [1-3] of 7/'); ok('clicking the graph jumps to that move');
  assert.ok(await p.locator('div[role=img][aria-label^="Evaluation"]').count() === 1); ok('eval bar appears with a review');
  const overflow = await p.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  assert.ok(overflow <= 0, 'page overflows horizontally by ' + overflow + 'px'); ok('the review panel fits the page (no horizontal overflow)');
  assert.match(await p.locator('[aria-label="Review summary"]').textContent(), /Inaccuracies|Inaccuracy/); ok('plurals read correctly');
  await p.screenshot({ path: SP + '/review.png', fullPage: true });
  await p.click('button:text-is("Hide review")');
  assert.equal(await p.locator('[aria-label="Review summary"]').count(), 0); ok('review can be hidden');

  // live engine
  await p.fill('#pgn', '1. e4 e5 2. Nf3'); await p.click('button:text-is("Load PGN")'); await p.waitForSelector('text=Loaded 3 moves');
  await p.check('label:has-text("Engine (Stockfish)") input');
  await p.waitForSelector('ol[aria-label="Engine lines"] li', { timeout: 30000 }); ok('live engine shows its best lines');
  const lines = await p.locator('ol[aria-label="Engine lines"] li').count(); assert.ok(lines >= 2); ok(`${lines} lines (multi-PV)`);
  await p.waitForSelector('svg.cg-shapes g', { timeout: 10000 }); ok('best-move arrow is drawn on the board');
  assert.ok(await p.locator('div[role=img][aria-label^="Evaluation"]').count() === 1); ok('eval bar shows the live evaluation');
  await p.screenshot({ path: SP + '/engine.png' });
  await p.uncheck('label:has-text("Engine (Stockfish)") input'); await p.waitForTimeout(400);
  assert.equal(await p.locator('ol[aria-label="Engine lines"]').count(), 0); ok('engine switches off');

  // hand-off from a finished game: review starts by itself
  await p.evaluate(() => { sessionStorage.setItem('rookery.import', '1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7# 1-0'); sessionStorage.setItem('rookery.review', '1'); });
  await p.goto(B + '/play/board');
  await p.waitForSelector('[aria-label="Review summary"]', { timeout: 90000 }); ok('"Review with engine" from a game opens the board and reviews it automatically');

  // ================================================================ opening explorer
  await p.goto(B + '/openings');
  await p.waitForSelector('#book'); await p.waitForSelector('table button:text-is("e4")', { timeout: 15000 });
  assert.ok((await p.locator('table').first().textContent()).includes("King's Pawn Game")); ok('the book lists 1.e4 as the King\'s Pawn Game');
  await p.waitForSelector('#class ~ div table, #class + div table, section[aria-labelledby=class] table', { timeout: 15000 });
  const classTxt = await p.locator('section[aria-labelledby=class]').textContent();
  assert.match(classTxt, /e4/); ok('class statistics from the database show what people played');
  await p.click('section[aria-labelledby=book] table button:text-is("e4")'); await p.waitForTimeout(400);
  assert.match(await p.locator('[aria-live=polite]').first().textContent(), /King's Pawn Game/); ok('clicking a book move plays it and names the position');
  const walk = ['c5', 'Nf3', 'd6', 'd4', 'cxd4', 'Nxd4', 'Nf6', 'Nc3', 'a6'];
  for (const m of walk) { await p.click(`section[aria-labelledby=book] table button:text-is("${m}")`); await p.waitForTimeout(250); }
  const header = await p.locator('[aria-live=polite]').first().textContent();
  assert.match(header, /Najdorf Variation/); ok('walking the Sicilian reaches the Najdorf: ' + header.trim().slice(0, 60));
  await p.screenshot({ path: SP + '/explorer.png', fullPage: true });
  await p.click('button:text-is("Back")'); await p.waitForTimeout(300);
  assert.doesNotMatch(await p.locator('[aria-live=polite]').first().textContent(), /Najdorf Variation \(|Najdorf Variation$/); ok('Back undoes a move');
  await p.click('button:text-is("Reset")'); await p.waitForTimeout(300);
  assert.match(await p.locator('[aria-live=polite]').first().textContent(), /Choose a first move/); ok('Reset returns to the start');
  for (const u of ['a2a3', 'a7a6', 'h2h3', 'h7h6', 'b2b3', 'b7b6', 'g2g3', 'g7g6']) await play(u);
  await p.waitForSelector('text=Out of book', { timeout: 5000 }); ok('moves off the beaten path are "Out of book"');
  await p.click('button:text-is("Analyse")'); await p.waitForURL(B + '/play/board'); await p.waitForSelector('text=Loaded your game'); ok('"Analyse" hands the line to the analysis board');

  const real = errors.filter((e) => !/Failed to load resource/.test(e));
  assert.equal(real.length, 0, 'console errors: ' + real.join(' | ')); ok('no console errors');
  await browser.close();
})().catch((e) => { console.error('FAIL', e.message); process.exit(1); });
