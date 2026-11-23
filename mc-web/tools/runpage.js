// usage: node runpage.js <dir> [seconds]   serves dir, opens index.html headless, prints console output
const http = require('http'), fs = require('fs'), path = require('path');
const { chromium } = require('playwright');
const dir = process.argv[2], secs = +(process.argv[3] || 10);
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' };
const srv = http.createServer((q, r) => {
  const f = path.join(dir, decodeURIComponent(q.url.split('?')[0]).replace(/^\/$/, '/index.html'));
  if (!fs.existsSync(f)) { r.writeHead(404); return r.end(); }
  r.writeHead(200, { 'Content-Type': types[path.extname(f)] || 'application/octet-stream',
    'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp' });
  fs.createReadStream(f).pipe(r);
}).listen(0, async () => {
  const b = await chromium.launch({ args: ['--disable-dev-shm-usage', '--no-sandbox', '--enable-unsafe-swiftshader', '--use-angle=swiftshader'].concat(process.env.JSFLAGS ? ['--js-flags=' + process.env.JSFLAGS] : []) });
  const p = await b.newPage();
  p.on('console', m => console.log('[console]', m.text()));
  p.on('pageerror', e => console.log('[pageerror]', e.message));
  p.on('crash', () => console.log('[PAGE CRASHED]'));
  await p.goto('http://127.0.0.1:' + srv.address().port + '/' + (process.env.QUERY || ''));
  await p.waitForTimeout(secs * 1000);
  // STEPS: JSON list of {wait, move:[x,y], click:[x,y], key, type, shot, hold:[key,ms]} replayed against the page
  for (const st of JSON.parse(process.env.STEPS || '[]')) {
    if (st.wait) await p.waitForTimeout(st.wait);
    if (st.move) await p.mouse.move(st.move[0], st.move[1]);
    if (st.click) { await p.mouse.move(st.click[0], st.click[1]); await p.mouse.down(); await p.waitForTimeout(80); await p.mouse.up(); }
    if (st.key) await p.keyboard.press(st.key);
    if (st.type) await p.keyboard.type(st.type, { delay: 40 });
    if (st.hold) { await p.keyboard.down(st.hold[0]); await p.waitForTimeout(st.hold[1]); await p.keyboard.up(st.hold[0]); }
    if (st.shot) await p.screenshot({ path: st.shot });
  }
  if (process.argv[4]) await p.screenshot({ path: process.argv[4] });
  await b.close(); srv.close();
});
