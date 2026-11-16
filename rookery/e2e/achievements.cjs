// Badges: finishing the basics lesson earns badges, a toast announces them, and the profile shows them. Needs a production server on :3111.
const { chromium } = require('playwright');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
const ok = (m) => console.log('OK  ', m);
const FILES = 'abcdefgh';
(async () => {
  const browser = await chromium.launch();
  const errors = [];
  const p = await (await browser.newContext({ viewport: { width: 1280, height: 1000 } })).newPage(); p.on('pageerror', (e) => errors.push(String(e)));
  const name = 'e2eab' + rnd();
  await p.goto(B + '/signup'); await p.fill('#username', name); await p.fill('#password', 'e2e-test-password-1'); await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });
  const move = async (uci) => { for (const s of [uci.slice(0, 2), uci.slice(2, 4)]) { const box = await p.locator('.cg-wrap').boundingBox(); const f = FILES.indexOf(s[0]), r = Number(s[1]); await p.mouse.click(box.x + (f + .5) * box.width / 8, box.y + (8 - r + .5) * box.height / 8); await p.waitForTimeout(250); } };

  await p.goto(B + '/u/' + name, { waitUntil: 'networkidle' });
  await p.waitForSelector('h2:text-is("Achievements")');
  await p.waitForSelector('text=0 of 36 badges earned');
  ok('a new player has the badge wall with nothing earned');

  await p.goto(B + '/learn/lessons/basics', { waitUntil: 'networkidle' });
  await p.click('button:has-text("Start the exercises")');
  await p.waitForSelector('.cg-wrap piece');
  for (const mv of ['d4d7', 'a2d5', 'e2d4', 'd1h5', 'e1d1', 'e7e8', 'e1g1', 'e5d6']) {
    await p.waitForSelector('[data-phase=solving]', { timeout: 10000 });
    await move(mv);
    if (mv === 'e7e8') await p.click('button[aria-label=Queen]');
    await p.waitForSelector('text=Correct!', { timeout: 10000 });
    await p.click('button:text-is("Next exercise"), button:text-is("Finish")');
  }
  await p.waitForSelector('text=Lesson complete!');
  await p.waitForSelector('text=Achievement unlocked', { timeout: 20000 });
  const toast = await p.locator('[role=status]:has-text("Achievement unlocked")').first().textContent();
  assert(/Student|Knows the moves/.test(toast), toast);
  ok('a toast announces the new badge: ' + toast.replace(/\s+/g, ' ').slice(0, 70));

  await p.goto(B + '/u/' + name, { waitUntil: 'networkidle' });
  await p.waitForSelector('li:has-text("Student") >> text=Earned');
  await p.waitForSelector('li:has-text("Knows the moves") >> text=Earned');
  assert((await p.locator('li:has-text("Founder")').textContent()).includes('Start a club'));
  ok('the profile shows earned badges with their date, and locked ones with what to do');
  const v = await (await browser.newContext()).newPage();
  await v.goto(B + '/u/' + name, { waitUntil: 'networkidle' });
  await v.waitForSelector('li:has-text("Knows the moves") >> text=Earned');
  ok('anyone can see them');
  assert.deepEqual(errors, [], errors.join('\n'));
  await browser.close();
  console.log('ACHIEVEMENTS PASSED (' + name + ')');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
