// Friends, chat, clubs, forum, reports and the admin panel, against the real Supabase project.
// Phase 1:  node e2e/social.cjs setup   -> creates users e2esA/B/M and saves names to /tmp/social-users.json
// Then make M a moderator:  update public.profiles set is_moderator = true where username = '<M>';
// Phase 2:  node e2e/social.cjs run [screenshot-dir]
const { chromium } = require('playwright');
const fs = require('fs');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const F = '/tmp/social-users.json';
let n = 0;
const ok = (m) => console.log('OK  ', ++n, m);
const rnd = () => String(Math.floor(10000 + Math.random() * 89999));
const PW = 'e2e-test-password-1';

async function signup(ctx, name) {
  const p = await ctx.newPage();
  await p.goto(B + '/signup');
  await p.fill('#username', name); await p.fill('#password', PW);
  await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });
  return p;
}
async function login(ctx, name) {
  const p = await ctx.newPage();
  await p.goto(B + '/login');
  await p.fill('#username', name); await p.fill('#password', PW);
  await p.click('button[type=submit]');
  await p.waitForURL(B + '/', { timeout: 30000 });
  return p;
}

(async () => {
  const browser = await chromium.launch();
  const mk = () => browser.newContext({ viewport: { width: 1280, height: 900 } });
  if (process.argv[2] === 'setup') {
    const names = { a: 'e2esa' + rnd(), b: 'e2esb' + rnd(), m: 'e2esm' + rnd() };
    for (const k of Object.keys(names)) { const c = await mk(); await signup(c, names[k]); await c.close(); }
    fs.writeFileSync(F, JSON.stringify(names));
    console.log(JSON.stringify(names));
    return browser.close();
  }
  const SP = process.argv[3] || '/tmp';
  const names = JSON.parse(fs.readFileSync(F, 'utf8'));
  const errors = [];
  const ca = await mk(), cb = await mk(), cm = await mk();
  const a = await login(ca, names.a), b = await login(cb, names.b), m = await login(cm, names.m);
  for (const p of [a, b, m]) p.on('pageerror', (e) => errors.push(String(e)));

  // ---- friends ----
  await a.goto(B + '/community/friends');
  await a.fill('input[maxlength="20"]', names.b);
  await a.click('text=Send request');
  await a.waitForSelector('text=Friend request sent');
  ok('A sent a friend request');
  await b.goto(B + '/community/friends');
  await b.waitForSelector('text=Friend requests');
  await b.click('button:has-text("Accept")');
  await b.waitForSelector(`a:has-text("${names.a}")`);
  ok('B accepted; A appears in the friend list');
  await a.reload();
  await a.waitForSelector(`a:has-text("${names.b}")`);
  await a.waitForSelector('[aria-label=online]', { timeout: 15000 });
  ok('A sees B online (presence heartbeat works)');

  // ---- profile buttons / block ----
  await a.goto(B + '/u/' + names.m);
  await a.waitForSelector('text=Add friend');
  ok('profile shows Add friend for a stranger');

  // ---- club + chat ----
  const club = 'E2E Club ' + rnd();
  await a.goto(B + '/clubs');
  await a.fill('input[minlength="3"]', club);
  await a.click('button:has-text("Create club")');
  await a.waitForURL(/\/clubs\//, { timeout: 15000 });
  await a.waitForSelector('text=Club chat');
  ok('A created a club and landed on it');
  const clubUrl = a.url();
  await b.goto(clubUrl);
  await b.click('button:has-text("Join club")');
  await b.waitForSelector('text=Leave club');
  ok('B joined the open club');
  await a.fill('input[aria-label=Message]', 'hello from A');
  await a.click('button:has-text("Send")');
  await b.waitForSelector('text=hello from A', { timeout: 15000 });
  ok('club chat message reaches B live');

  // ---- forum ----
  await a.goto(B + '/forum/general');
  await a.fill('input[minlength="3"]', 'E2E thread ' + rnd());
  await a.fill('textarea', 'first post body');
  await a.click('button:has-text("Post thread")');
  await a.waitForURL(/\/forum\/thread\//, { timeout: 15000 });
  await a.waitForSelector('text=first post body');
  ok('A started a forum thread');
  const threadUrl = a.url();
  await b.goto(threadUrl);
  await b.fill('textarea', 'a reply from B');
  await b.click('button:has-text("Post reply")');
  await b.waitForSelector('p:has-text("a reply from B")');
  ok('B replied');
  for (let i = 0; i < 8; i++) {
    await a.reload();
    if (await a.locator('p:has-text("a reply from B")').count()) break;
    await a.waitForTimeout(1500);
  }
  await a.waitForSelector('p:has-text("a reply from B")');
  ok('A sees the reply');

  // ---- report + moderation ----
  await a.locator('li:has-text("a reply from B") >> text=Report').first().click();
  await a.selectOption('select', 'abuse');
  await a.fill('textarea[maxlength="500"]', 'e2e report');
  await a.click('button:has-text("Send report")');
  await a.waitForSelector('text=A moderator will take a look');
  ok('A reported the reply');
  await m.goto(B + '/admin');
  await m.waitForSelector('text=e2e report', { timeout: 15000 });
  ok('moderator sees the report');
  await m.click('button:has-text("Show content")');
  await m.waitForSelector('text=a reply from B');
  await m.click('li:has-text("e2e report") >> button:has-text("Mute 1h")');
  await m.waitForSelector('li:has-text("e2e report")', { state: 'detached', timeout: 15000 });
  ok('moderator muted B and resolved the report');
  await b.goto(threadUrl);
  await b.fill('textarea', 'should be blocked');
  await b.click('button:has-text("Post reply")');
  await b.waitForSelector('[role=alert]');
  await b.goto(B + '/');
  await b.waitForSelector('text=You are muted');
  ok('muted B cannot post and sees the banner');
  await m.click('button[role=tab]:has-text("Audit log")');
  await m.waitForSelector('text=mute');
  ok('audit log records the mute');
  await m.click('button[role=tab]:has-text("Users")');
  await m.fill('input[aria-label="Search username"]', names.b);
  await m.click('button:has-text("Search")');
  await m.click('button:has-text("Unmute")');
  await m.waitForSelector('button:has-text("Mute 1h")');
  ok('moderator unmuted B');

  // ---- non-mod cannot see admin ----
  await a.goto(B + '/admin');
  await a.waitForSelector('text=could not be found', { timeout: 15000 });
  assert.equal(await a.locator('[role=tablist]').count(), 0);
  ok('regular players get 404 on /admin');

  await a.goto(clubUrl); await a.screenshot({ path: SP + '/club.png' });
  await m.goto(B + '/admin'); await m.screenshot({ path: SP + '/admin.png' });
  assert.deepEqual(errors, [], 'page errors: ' + errors.join('\n'));
  ok('no page errors');
  await browser.close();
  console.log('ALL SOCIAL E2E PASSED');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
