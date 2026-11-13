// Move feedback under the board while playing the computer. Needs a production server on :3111.
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const LABELS = ['Brilliant', 'Great', 'Best', 'Good', 'Inaccuracy', 'Mistake', 'Blunder'];
(async () => {
  const browser = await chromium.launch();
  const p = await (await browser.newContext({ viewport: { width: 1280, height: 1000 } })).newPage();
  const errors = []; p.on('pageerror', (e) => errors.push(String(e)));
  await p.goto(B + '/play/computer', { waitUntil: 'networkidle' });
  await p.click('[role=radio]:has-text("Der Mann")');
  await p.selectOption('select >> nth=0', 'white');
  await p.waitForFunction(() => { const x = [...document.querySelectorAll('button')].find((y) => y.textContent === 'Play'); return x && !x.disabled; }, null, { timeout: 30000 });
  await p.click('button:text-is("Play")');
  await p.waitForSelector('.cg-wrap piece');
  await p.waitForSelector('[aria-label="Move feedback"]:has-text("how good it was")');
  console.log('OK   the feedback bar is under the board and invites a move');
  const mv = async (f1, r1, f2, r2) => { for (const [f, r] of [[f1, r1], [f2, r2]]) { const box = await p.locator('.cg-wrap').boundingBox(); await p.mouse.click(box.x + (f + .5) * box.width / 8, box.y + (8 - r + .5) * box.height / 8); await p.waitForTimeout(250); } };
  const chip = async (since) => {
    await p.waitForFunction(({ labels, since }) => { const t = document.querySelector('[aria-label="Move feedback"]')?.textContent || ''; return labels.some((l) => t.includes(l)) && (!since || !t.includes(since)); }, { labels: LABELS, since }, { timeout: 30000 });
    return p.locator('[aria-label="Move feedback"]').textContent();
  };
  await mv(4, 2, 4, 4); // e4
  const t1 = await chip();
  assert(/e4/.test(t1), t1);
  console.log('OK   after 1.e4 the bar says:', t1.replace(/\s+/g, ' ').trim().slice(0, 80));
  await p.waitForFunction(() => document.querySelectorAll('ol[aria-label=Moves] button').length >= 2, null, { timeout: 40000 });
  // a bad move: bring the king out
  await mv(4, 1, 4, 2); // Ke2
  await p.waitForFunction(() => /Ke2/.test(document.querySelector('[aria-label="Move feedback"]')?.textContent || ''), null, { timeout: 30000 });
  const t2 = await p.locator('[aria-label="Move feedback"]').textContent();
  assert(LABELS.some((l) => t2.includes(l)), t2);
  console.log('OK   after Ke2 the bar says:', t2.replace(/\s+/g, ' ').trim().slice(0, 100));
  // toggle off and on
  await p.click('button:has-text("Turn off feedback")');
  await p.waitForSelector('text=Move feedback is off');
  await p.click('button:has-text("Turn on feedback")');
  console.log('OK   feedback can be turned off and on');
  assert.deepEqual(errors, [], errors.join('\n'));
  await p.locator('[aria-label="Move feedback"]').screenshot({ path: '/tmp/feedback-bar.png' });
  await browser.close();
  console.log('FEEDBACK PASSED');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
