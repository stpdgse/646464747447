// Admin bot editor. Phase 1: `node e2e/botsadmin.cjs setup` creates an account, then make it admin:
//   update public.profiles set is_admin = true where username = '<name>';
// Phase 2: `node e2e/botsadmin.cjs run`. Needs a production server on :3111.
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const F = '/tmp/botsadmin-user.json';
const PW = 'e2e-test-password-1';
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
const ok = (m) => console.log('OK  ', m);

(async () => {
  const browser = await chromium.launch();
  const mkctx = () => browser.newContext({ viewport: { width: 1280, height: 1000 } });
  if (process.argv[2] === 'setup') {
    const name = 'e2eba' + rnd();
    const c = await mkctx(); const p = await c.newPage();
    await p.goto(B + '/signup'); await p.fill('#username', name); await p.fill('#password', PW); await p.click('button[type=submit]');
    await p.waitForURL(B + '/', { timeout: 30000 });
    fs.writeFileSync(F, JSON.stringify({ name }));
    console.log(name);
    return browser.close();
  }
  const { name } = JSON.parse(fs.readFileSync(F, 'utf8'));
  const errors = [];
  const c = await mkctx(); const a = await c.newPage(); a.on('pageerror', (e) => errors.push(String(e)));
  await a.goto(B + '/login'); await a.fill('#username', name); await a.fill('#password', PW); await a.click('button[type=submit]');
  await a.waitForURL(B + '/', { timeout: 30000 });
  const botName = 'E2E Bot ' + rnd();

  await a.goto(B + '/admin?tab=bots', { waitUntil: 'networkidle' });
  await a.waitForSelector('button:has-text("New bot")');
  ok('admin sees the Bots tab with the roster');
  assert((await a.locator('li:has-text("built-in")').count()) >= 30);

  // ---- new bot with a picture, smartness and habits
  await a.click('button:has-text("New bot")');
  await a.fill('input[maxlength="40"]', botName);
  await a.fill('input[maxlength="200"]', 'Made in the admin editor.');
  await a.fill('input[maxlength="160"] >> nth=0', 'Hello from the editor!');
  await a.setInputFiles('input[type=file]', path.join(__dirname, '../public/bots/dermann.png'));
  await a.waitForSelector('section[aria-label="Looks and words"] img');
  ok('picture uploaded and previewed');
  await a.locator('input[type=range]').first().fill('4');
  await a.waitForSelector('text=Smartness: 4');
  await a.click('button:has-text("Suggest from smartness")');
  await a.locator('label:has-text("Style") input[type=checkbox]').first().check();
  await a.selectOption('section[aria-label="Playing habits"] select', 'greedy');
  await a.click('button:text-is("Create bot")');
  await a.waitForSelector('text=Saved ' + botName);
  await a.waitForSelector(`li:has-text("${botName}")`);
  ok('new bot saved and listed');

  // ---- anyone (signed out) sees it in the picker, with the picture
  const anon = await (await mkctx()).newPage(); anon.on('pageerror', (e) => errors.push(String(e)));
  await anon.goto(B + '/play/computer', { waitUntil: 'networkidle' });
  await anon.waitForSelector(`[role=radio]:has-text("${botName}")`, { timeout: 15000 });
  assert.equal(await anon.locator(`[role=radio]:has-text("${botName}") img`).count(), 1);
  ok('a signed-out visitor sees the new bot with its picture');
  await anon.click(`[role=radio]:has-text("${botName}")`);
  await anon.waitForSelector('text=Hello from the editor!');
  ok('greeting is shown');
  // and it plays
  await anon.selectOption('select >> nth=0', 'white');
  await anon.waitForFunction(() => { const x = [...document.querySelectorAll('button')].find((y) => y.textContent === 'Play'); return x && !x.disabled; }, null, { timeout: 30000 });
  await anon.click('button:text-is("Play")');
  await anon.waitForSelector('.cg-wrap piece');
  const box = await anon.locator('.cg-wrap').boundingBox();
  for (const [f, r] of [[4, 2], [4, 4]]) { await anon.mouse.click(box.x + (f + .5) * box.width / 8, box.y + (8 - r + .5) * box.height / 8); await anon.waitForTimeout(250); }
  await anon.waitForFunction(() => document.querySelectorAll('ol[aria-label=Moves] button').length >= 2, null, { timeout: 40000 });
  ok('the custom bot plays a game');

  // ---- change a built-in bot (Pawnie) and reset it
  await a.goto(B + '/admin?tab=bots', { waitUntil: 'networkidle' });
  await a.click('li:has-text("Pawnie") >> button:text-is("Edit")');
  await a.fill('input[maxlength="40"]', 'Pawnie Prime');
  await a.click('button:text-is("Save changes")');
  await a.waitForSelector('text=Saved Pawnie Prime');
  const anon2 = await (await mkctx()).newPage();
  await anon2.goto(B + '/play/computer', { waitUntil: 'networkidle' });
  await anon2.waitForSelector('[role=radio]:has-text("Pawnie Prime")', { timeout: 15000 });
  ok('an edit to a built-in bot shows for everyone');
  a.once('dialog', (d) => d.accept());
  await a.click('li:has-text("Pawnie Prime") >> button:text-is("Reset")');
  await a.waitForSelector('text=Bot reset');
  await a.waitForSelector('li:has-text("Pawnie")', { timeout: 10000 });
  assert.equal(await a.locator('li:has-text("Pawnie Prime")').count(), 0);
  ok('reset returns the built-in bot to normal');

  // ---- hide / show
  if (await a.locator('li:has-text("Sleepy Sam") >> button:text-is("Reset")').count()) { a.once('dialog', (d) => d.accept()); await a.click('li:has-text("Sleepy Sam") >> button:text-is("Reset")'); await a.waitForSelector('text=Bot reset'); } // leftover from an older run
  await a.click('li:has-text("Sleepy Sam") >> button:text-is("Hide")');
  await a.waitForSelector('li:has-text("Sleepy Sam"):has-text("hidden")', { timeout: 10000 });
  await anon2.reload({ waitUntil: 'networkidle' });
  await anon2.waitForTimeout(1500);
  assert.equal(await anon2.locator('[role=radio]:has-text("Sleepy Sam")').count(), 0);
  await a.click('li:has-text("Sleepy Sam") >> button:text-is("Show again")');
  await a.waitForSelector('li:has-text("Sleepy Sam") >> button:text-is("Hide")', { timeout: 10000 });
  await a.waitForSelector('li:has-text("Sleepy Sam") >> button:text-is("Reset")', { state: 'detached', timeout: 10000 }); // a plain hide leaves no edit behind
  ok('hiding removes a bot from the picker, showing brings it back');

  // ---- delete the custom bot
  a.once('dialog', (d) => d.accept());
  await a.click(`li:has-text("${botName}") >> button:text-is("Delete")`);
  await a.waitForSelector('text=Bot deleted');
  await a.waitForSelector(`li:has-text("${botName}")`, { state: 'detached', timeout: 10000 });
  ok('custom bot deleted');

  // ---- a regular player cannot see the tab or write
  const p2 = await (await mkctx()).newPage();
  const n2 = 'e2ebb' + rnd();
  await p2.goto(B + '/signup'); await p2.fill('#username', n2); await p2.fill('#password', PW); await p2.click('button[type=submit]');
  await p2.waitForURL(B + '/', { timeout: 30000 });
  await p2.goto(B + '/admin?tab=bots');
  await p2.waitForSelector('text=could not be found', { timeout: 15000 });
  ok('regular players cannot open the admin page');
  assert.deepEqual(errors, [], errors.join('\n'));
  await browser.close();
  console.log('BOTS ADMIN PASSED');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
