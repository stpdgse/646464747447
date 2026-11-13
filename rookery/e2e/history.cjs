// Bot game history on the profile. Needs a production server on :3111.
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
const ok = (m) => console.log('OK  ', m);
(async () => {
  const browser = await chromium.launch();
  const errors = [];
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 1000 } });
  const p = await ctx.newPage(); p.on('pageerror', (e) => errors.push(String(e)));
  const name = 'e2eh' + rnd();
  await p.goto(B + '/signup'); await p.fill('#username', name); await p.fill('#password', 'e2e-test-password-1'); await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });

  const playAndResign = async (bot, isPublic) => {
    await p.evaluate(() => localStorage.removeItem('rookery.botgame.v1'));
    await p.goto(B + '/play/computer', { waitUntil: 'networkidle' });
    await p.click(`[role=radio]:has-text("${bot}")`);
    await p.selectOption('select >> nth=0', 'white');
    if (!isPublic) await p.uncheck('input[type=checkbox] >> nth=1');
    await p.waitForFunction(() => { const x = [...document.querySelectorAll('button')].find((y) => y.textContent === 'Play'); return x && !x.disabled; }, null, { timeout: 30000 });
    await p.click('button:text-is("Play")');
    await p.waitForSelector('.cg-wrap piece');
    for (const [f, r] of [[4, 2], [4, 4]]) { const bx = await p.locator('.cg-wrap').boundingBox(); await p.mouse.click(bx.x + (f + .5) * bx.width / 8, bx.y + (8 - r + .5) * bx.height / 8); await p.waitForTimeout(250); }
    await p.waitForFunction(() => document.querySelectorAll('ol[aria-label=Moves] button').length >= 2, null, { timeout: 40000 });
    await p.waitForTimeout(1500);
    await p.click('button:text-is("Resign")');
    await p.waitForSelector('text=wins');
    await p.waitForTimeout(2500); // let the final result reach the database
  };
  await playAndResign('Der Mann', true);
  await playAndResign('Pawnie', false);
  ok('played two games and resigned both (one public, one private)');

  await p.goto(B + '/u/' + name, { waitUntil: 'networkidle' });
  await p.waitForSelector('h2:text-is("Against the computer")');
  await p.waitForSelector('li:has-text("Pawnie")');
  await p.waitForSelector('li:has-text("Der Mann")');
  assert((await p.locator('li:has-text("Pawnie")').textContent()).includes('private'));
  assert((await p.locator('li:has-text("Pawnie")').textContent()).includes('Loss'));
  ok('the owner sees both games with results, and the private one is marked');
  assert((await p.locator('section[aria-labelledby=botgames] >> text=/0 won · 0 drawn · 2 lost/').count()) === 1);
  ok('the record line adds up');

  await p.click('li:has-text("Der Mann") >> button:text-is("Review")');
  await p.waitForURL('**/play/board');
  await p.waitForSelector('.cg-wrap piece');
  await p.waitForFunction(() => document.querySelectorAll('ol[aria-label=Moves] button').length >= 2, null, { timeout: 15000 });
  ok('Review opens the game on the analysis board');

  // another visitor: only the public game
  const v = await (await browser.newContext()).newPage();
  await v.goto(B + '/u/' + name, { waitUntil: 'networkidle' });
  await v.waitForSelector('li:has-text("Der Mann")');
  assert.equal(await v.locator('li:has-text("Pawnie")').count(), 0);
  ok('a visitor sees the public game only, never the private one');
  assert.deepEqual(errors, [], errors.join('\n'));
  await browser.close();
  console.log('HISTORY PASSED (' + name + ')');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
