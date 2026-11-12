// "A class of 30" load test against the real Supabase project and a running production build.
//   SUPABASE_URL=... SUPABASE_KEY=<publishable key> BASE=http://localhost:3111 node e2e/load.cjs [users=30]
// Part 1: page throughput (autocannon). Part 2: N users sign up, join one club, chat at once while one listens over Realtime.
// Creates N throwaway accounts named e2elNNNNN_i (delete them afterwards, see README).
const autocannon = require('autocannon');
const { createClient } = require('@supabase/supabase-js');
const B = process.env.BASE || 'http://localhost:3111';
const URL_ = process.env.SUPABASE_URL, KEY = process.env.SUPABASE_KEY;
if (!URL_ || !KEY) { console.error('Set SUPABASE_URL and SUPABASE_KEY (the publishable key).'); process.exit(2); }
const N = Number(process.argv[2] || 30);
const pct = (xs, p) => { const s = [...xs].sort((a, b) => a - b); return s.length ? Math.round(s[Math.min(s.length - 1, Math.floor(p * s.length))]) : 0; };
const timed = async (xs, fn) => { const t = performance.now(); const r = await fn(); xs.push(performance.now() - t); return r; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let failed = false;
const check = (cond, msg) => { console.log(cond ? 'OK  ' : 'FAIL', msg); if (!cond) failed = true; };

(async () => {
  // ---- part 1: pages ----
  for (const path of ['/', '/play', '/leaderboard', '/forum', '/puzzles']) {
    const r = await autocannon({ url: B + path, connections: N, duration: 8 });
    const bad = r.non2xx + r.errors + r.timeouts;
    console.log(`${path.padEnd(13)} ${Math.round(r.requests.average)} req/s  p50 ${r.latency.p50} ms  p99 ${r.latency.p99} ms  errors ${bad}`);
    check(bad === 0 && r.latency.p99 < 5000, `${path}: no errors, p99 under 5 s`);
  }

  // ---- part 2: realtime chat with N users ----
  const tag = String(Math.floor(10000 + Math.random() * 89999));
  const mkc = () => createClient(URL_, KEY, { auth: { persistSession: false, autoRefreshToken: false } });
  const users = [];
  const tSign = [];
  await Promise.all(Array.from({ length: N }, async (_, i) => {
    const c = mkc(); const username = `e2el${tag}_${i}`;
    const { data, error } = await timed(tSign, () => c.auth.signUp({ email: `${username}@users.rookery.invalid`, password: 'e2e-test-password-1', options: { data: { username } } }));
    if (error || !data.session) { console.error('signup failed', username, error?.message); failed = true; return; }
    users[i] = { c, id: data.user.id, username };
  }));
  const us = users.filter(Boolean);
  check(us.length === N, `${N} users signed up (p50 ${pct(tSign, 0.5)} ms, p95 ${pct(tSign, 0.95)} ms)`);
  if (us.length < 2) process.exit(1);

  const owner = us[0];
  const slug = `e2e-load-${tag}`;
  const { error: ce } = await owner.c.from('clubs').insert({ slug, name: `Load ${tag}`, description: 'load test', owner_id: owner.id, open: true });
  check(!ce, 'owner created the club ' + (ce?.message ?? ''));
  const { data: club } = await owner.c.from('clubs').select('id').eq('slug', slug).single();

  const tJoin = [], tPresence = [], tSend = [], tRead = [];
  await Promise.all(us.slice(1).map((u) => timed(tJoin, () => u.c.from('club_members').insert({ club_id: club.id, user_id: u.id, role: 'member', status: 'active' }))));
  check(true, `all joined (p95 ${pct(tJoin, 0.95)} ms)`);

  // one listener measures how long a message takes to arrive over Realtime
  const listener = us[1];
  const sentAt = new Map(); const lat = []; let received = 0;
  const ch = listener.c.channel('load-' + tag).on('postgres_changes', { event: 'INSERT', schema: 'public', table: 'club_messages', filter: `club_id=eq.${club.id}` }, (p) => {
    received++; const t = sentAt.get(p.new.body); if (t) lat.push(performance.now() - t);
  });
  await new Promise((res) => ch.subscribe((s) => s === 'SUBSCRIBED' && res()));

  const PER = 5;
  await Promise.all(us.map(async (u, i) => {
    await timed(tPresence, () => u.c.from('presence').upsert({ user_id: u.id, last_seen_at: new Date().toISOString() }, { onConflict: 'user_id' }));
    for (let k = 0; k < PER; k++) {
      await sleep(Math.random() * 1500);
      const body = `m${i}-${k}-${tag}`;
      sentAt.set(body, performance.now());
      const { error } = await timed(tSend, () => u.c.from('club_messages').insert({ club_id: club.id, user_id: u.id, body }));
      if (error) { console.error('send failed', error.message); failed = true; }
      await timed(tRead, () => u.c.from('club_messages').select('id, body, author:profiles(username)').eq('club_id', club.id).order('created_at', { ascending: false }).limit(60));
    }
  }));
  await sleep(4000);
  const total = us.length * PER;
  console.log(`messages sent ${total}, received over Realtime ${received}`);
  console.log(`send p50 ${pct(tSend, 0.5)} ms p95 ${pct(tSend, 0.95)} ms · read p50 ${pct(tRead, 0.5)} ms p95 ${pct(tRead, 0.95)} ms · presence p95 ${pct(tPresence, 0.95)} ms · realtime delivery p50 ${pct(lat, 0.5)} ms p95 ${pct(lat, 0.95)} ms`);
  check(received >= total * 0.98, 'Realtime delivered at least 98% of messages');
  check(pct(tSend, 0.95) < 2000 && pct(tRead, 0.95) < 2000, 'writes and reads stay under 2 s at p95');
  const { count } = await listener.c.from('club_messages').select('id', { count: 'exact', head: true }).eq('club_id', club.id);
  check(count === total, `database has all ${total} messages (${count})`);
  await listener.c.removeChannel(ch);
  console.log('Test accounts: e2el' + tag + '_0 .. _' + (N - 1) + '   club slug: ' + slug);
  process.exit(failed ? 1 : 0);
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
