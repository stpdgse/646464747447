// Lessons and endgames. Needs a production server on :3111.
const { chromium } = require('playwright');
const assert = require('assert');
const puzzles = require('../src/data/puzzles.json');
const B = process.env.BASE || 'http://localhost:3111';
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
const ok = (m) => console.log('OK  ', m);
const FILES = 'abcdefgh';

(async () => {
  const browser = await chromium.launch();
  const errors = [];
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 1000 } });
  const p = await ctx.newPage(); p.on('pageerror', (e) => errors.push(String(e)));
  const name = 'e2el' + rnd();
  await p.goto(B + '/signup'); await p.fill('#username', name); await p.fill('#password', 'e2e-test-password-1'); await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });

  const sqXY = async (sq) => {
    const box = await p.locator('.cg-wrap').boundingBox();
    const black = await p.evaluate(() => document.querySelector('.cg-wrap').classList.contains('orientation-black'));
    const f = FILES.indexOf(sq[0]), r = Number(sq[1]);
    const col = black ? 7 - f : f, row = black ? r - 1 : 8 - r;
    return { x: box.x + (col + .5) * box.width / 8, y: box.y + (row + .5) * box.height / 8 };
  };
  const move = async (uci) => { for (const s of [uci.slice(0, 2), uci.slice(2, 4)]) { const q = await sqXY(s); await p.mouse.click(q.x, q.y); await p.waitForTimeout(250); } };

  await p.goto(B + '/learn', { waitUntil: 'networkidle' });
  assert((await p.locator('a[href^="/learn/lessons/"]').count()) >= 12);
  await p.waitForSelector('a[href="/learn/endgames"]');
  ok('the Learn page lists the lessons and the endgames');

  // ---- basics: the 8 hand-made exercises
  await p.goto(B + '/learn/lessons/basics', { waitUntil: 'networkidle' });
  await p.click('button:has-text("Start the exercises")');
  await p.waitForSelector('.cg-wrap piece');
  // a wrong move first
  await move('d4d5');
  await p.waitForSelector('text=Not quite');
  ok('a wrong move is refused with a friendly message');
  await p.click('button:text-is("Hint")');
  const steps = ['d4d7', 'a2d5', 'e2d4', 'd1h5', 'e1d1', 'e7e8', 'e1g1', 'e5d6'];
  for (let i = 0; i < steps.length; i++) {
    await p.waitForSelector('[data-phase=solving]', { timeout: 10000 });
    await move(steps[i]);
    if (steps[i] === 'e7e8') await p.click('button[aria-label=Queen], button:has-text("Queen")');
    await p.waitForSelector('text=Correct!', { timeout: 10000 });
    await p.click('button:text-is("Next exercise"), button:text-is("Finish")');
  }
  await p.waitForSelector('text=Lesson complete!', { timeout: 10000 });
  ok('all 8 basics exercises solved and the lesson completes');

  // ---- a puzzle lesson, solved with the real solutions
  await p.goto(B + '/learn/lessons/mate-in-1', { waitUntil: 'networkidle' });
  await p.click('button:has-text("Start the exercises")');
  for (let n = 0; n < 8; n++) {
    await p.waitForSelector('[data-phase=solving]', { timeout: 15000 });
    const id = await p.locator('[data-item]').first().getAttribute('data-item');
    const row = puzzles.find((r) => r[0] === id);
    assert(row, 'puzzle ' + id);
    const sol = row[2].split(' ')[1];
    await move(sol);
    if (sol.length === 5) await p.click('button[aria-label=Queen], button:has-text("Queen")'); // promotion
    await p.waitForSelector('text=Correct!', { timeout: 10000 });
    await p.click('button:text-is("Next exercise"), button:text-is("Finish")');
  }
  await p.waitForSelector('text=Lesson complete!', { timeout: 10000 });
  ok('a puzzle lesson: eight real mate-in-one puzzles solved');

  // ---- progress is saved
  await p.goto(B + '/learn', { waitUntil: 'networkidle' });
  assert((await p.locator('a[href="/learn/lessons/basics"]:has-text("Complete")').count()) === 1);
  assert((await p.locator('a[href="/learn/lessons/mate-in-1"]:has-text("Complete")').count()) === 1);
  ok('progress is saved: both lessons show as complete on the Learn page');

  // ---- an endgame against the engine
  await p.goto(B + '/learn/endgames', { waitUntil: 'networkidle' });
  assert((await p.locator('a[href^="/learn/endgames/"]').count()) >= 9);
  await p.goto(B + '/learn/endgames/kq-k', { waitUntil: 'networkidle' });
  await p.waitForSelector('text=Your move', { timeout: 30000 });
  await move('d1a4');
  await p.waitForFunction(() => { const b = [...document.querySelectorAll('button')].find((x) => x.textContent === 'Take back'); return b && !b.disabled; }, null, { timeout: 10000 });
  await p.waitForTimeout(1500); // the engine's reply (about a quarter of a second) lands
  await p.waitForSelector('text=Your move', { timeout: 20000 });
  ok('endgame: the engine answers your move');
  await p.click('button:text-is("Take back")');
  await p.click('button:text-is("Start over")');
  ok('take back and start over work');
  assert.deepEqual(errors, [], errors.join('\n'));
  await browser.close();
  console.log('LEARN PASSED (' + name + ')');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
