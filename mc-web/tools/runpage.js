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
  const b = await chromium.launch();
  const p = await b.newPage();
  p.on('console', m => console.log('[console]', m.text()));
  p.on('pageerror', e => console.log('[pageerror]', e.message));
  await p.goto('http://127.0.0.1:' + srv.address().port + '/');
  await p.waitForTimeout(secs * 1000);
  await b.close(); srv.close();
});
