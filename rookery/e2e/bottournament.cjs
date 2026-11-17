// Bot tournaments: one player starts a knockout, a visitor watches, the host leaves, another viewer takes over, a champion is crowned.
// Needs a production server on :3111. Takes a few minutes (real games between real engines).
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
const ok = (m) => console.log('OK  ', m);
const PW = 'e2e-test-password-1';
async function signup(ctx, name) {
  const p = await ctx.newPage();
  await p.goto(B + '/signup'); await p.fill('#username', name); await p.fill('#password', PW); await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });
  return p;
}
(async () => {
  const browser = await chromium.launch();
  const mk = () => browser.newContext({ viewport: { width: 1280, height: 1100 } });
  const errors = [];
  const a = await signup(await mk(), 'e2ebt' + rnd()), bUser = 'e2ebu' + rnd();
  const b = await signup(await mk(), bUser);
  const v = await (await mk()).newPage();
  for (const p of [a, b, v]) p.on('pageerror', (e) => errors.push(String(e)));

  // ---- create
  await a.goto(B + '/bot-tournaments', { waitUntil: 'networkidle' });
  const tname = 'E2E Cup ' + rnd();
  await a.fill('input[minlength="3"]', tname);
  await a.selectOption('select >> nth=1', '4'); // 4 bots
  await a.waitForSelector('text=Choose the bots');
  assert((await a.locator('button:text-is("Create tournament")').isDisabled()), 'cannot create with no bots');
  ok('the form refuses to create a tournament until the right number of bots is picked');
  for (const label of ['≈4000', '≈2800', '≈450', '≈51']) await a.click(`label:has-text("${label}")`);
  await a.click('button:text-is("Create tournament")');
  await a.waitForURL(/\/bot-tournaments\/[0-9a-f-]{36}/, { timeout: 30000 });
  const url = a.url();
  await a.waitForSelector('text=You are running this tournament', { timeout: 30000 });
  await a.waitForSelector('h3:text-is("Semi-finals")');
  await a.waitForSelector('h3:text-is("Final")');
  ok('the creator lands on the tournament and starts running it; the bracket shows semi-finals and a final');
  await a.waitForSelector('.cg-wrap piece');
  await a.waitForFunction(() => document.querySelectorAll('ol[aria-label=Moves] button').length >= 4, null, { timeout: 60000 });
  ok('the first game is being played on the board');

  // ---- limits
  await b.goto(B + '/bot-tournaments', { waitUntil: 'networkidle' });
  assert((await b.locator(`a:has-text("${tname}")`).count()) >= 1, 'running list');
  ok('another player sees it in the Running now list');
  const a2 = await (await mk()).newPage();
  // a second tournament by the same person is refused (use a fresh page, same account via cookies)
  const cookies = await a.context().cookies();
  await a2.context().addCookies(cookies);
  await a2.goto(B + '/bot-tournaments', { waitUntil: 'networkidle' });
  await a2.click('button:text-is("Random bots")');
  await a2.selectOption('select >> nth=1', '4'); // changing the size clears the picks
  await a2.click('button:text-is("Random bots")');
  await a2.click('button:text-is("Create tournament")');
  await a2.waitForSelector('text=already have a tournament running', { timeout: 15000 });
  ok('a second tournament at the same time is refused with a clear message (the spam limit)');
  await a2.close();

  // ---- a visitor watches
  await v.goto(B + '/watch', { waitUntil: 'networkidle' });
  await v.waitForSelector(`section[aria-labelledby=bottour] a:has-text("${tname}")`, { timeout: 20000 });
  await v.click(`section[aria-labelledby=bottour] a:has-text("${tname}")`);
  await v.waitForSelector('h3:text-is("Final")');
  await v.waitForSelector('text=Live: running in', { timeout: 20000 });
  assert.equal(await v.locator('button:text-is("Run this tournament")').count(), 0);
  ok('a signed-out visitor finds it on the Watch tab and sees it running live in the host\'s browser');
  await v.waitForSelector('button:text-matches("^G1 (1-0|0-1|1/2-1/2)$")', { timeout: 240000 });
  ok('the bracket records the first finished game');

  // ---- the host leaves; another viewer takes over
  await a.goto(B + '/', { waitUntil: 'networkidle' });
  await b.goto(url, { waitUntil: 'networkidle' });
  await b.waitForSelector('button:text-is("Run this tournament")', { timeout: 60000 });
  const before = await b.locator('button:text-matches("^G\\\\d (1-0|0-1|1/2-1/2)$")').count();
  await b.click('button:text-is("Run this tournament")');
  await b.waitForSelector('text=You are running this tournament', { timeout: 30000 });
  ok('when the host leaves, another viewer can take over and keeps the tournament going');

  // ---- to the end
  await b.waitForSelector('text=Champion', { timeout: 600000 });
  const champ = (await b.locator('[role=status]:has-text("Champion") p.text-2xl').textContent()).trim();
  assert(champ.length > 1);
  ok('the tournament ends with a champion: ' + champ);
  await v.waitForSelector('text=Champion', { timeout: 30000 });
  ok('and the visitor sees the champion too');
  await v.goto(B + '/bot-tournaments', { waitUntil: 'networkidle' });
  await v.waitForSelector('h2:text-is("Bot league")');
  await v.waitForSelector('table:has-text("Elo")');
  ok('the Bot league (Elo) table lists the bots');
  assert.deepEqual(errors, [], errors.join('\n'));
  await browser.close();
  console.log('BOT TOURNAMENT PASSED (' + tname + ')');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
