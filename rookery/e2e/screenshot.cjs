// Press S on a game page -> the board image lands on the clipboard. Needs a running server on :3111.
const { chromium } = require('playwright');
const fs = require('fs');
const assert = require('assert');
const B = process.env.BASE || 'http://localhost:3111';
const SP = process.argv[2] || '/tmp';
(async () => {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 900 }, permissions: ['clipboard-read', 'clipboard-write'] });
  const p = await ctx.newPage();
  const errors = []; p.on('pageerror', (e) => errors.push(String(e)));
  await p.goto(B + '/play/computer', { waitUntil: 'networkidle' });
  await p.click('[role=radio]:has-text("Pawnie")');
  await p.waitForFunction(() => { const b = [...document.querySelectorAll('button')].find((x) => x.textContent === 'Play'); return b && !b.disabled; }, null, { timeout: 30000 });
  await p.click('button:text-is("Play")');
  await p.waitForSelector('.cg-wrap piece');
  // make a move so there is a last-move highlight
  const box = await p.locator('.cg-wrap').boundingBox();
  const sq = (f, r) => ({ x: box.x + (f + 0.5) * box.width / 8, y: box.y + (8 - r + 0.5) * box.height / 8 });
  const orientBlack = await p.evaluate(() => document.querySelector('.cg-wrap')?.classList.contains('orientation-black'));
  if (!orientBlack) { for (const s of [sq(4, 2), sq(4, 4)]) { await p.mouse.click(s.x, s.y); await p.waitForTimeout(250); } }
  await p.waitForTimeout(1500);
  await p.keyboard.press('s');
  await p.waitForSelector('text=Board copied', { timeout: 10000 });
  const info = await p.evaluate(async () => {
    const items = await navigator.clipboard.read();
    const blob = await items[0].getType('image/png');
    const bmp = await createImageBitmap(blob);
    const c = document.createElement('canvas'); c.width = c.height = 64; const x = c.getContext('2d'); x.drawImage(bmp, 0, 0, 64, 64);
    const d = x.getImageData(0, 0, 64, 64).data; let nonBlank = 0; for (let i = 0; i < d.length; i += 4) if (d[i + 3] > 0) nonBlank++;
    const url = URL.createObjectURL(blob);
    const buf = Array.from(new Uint8Array(await blob.arrayBuffer()));
    return { type: blob.type, w: bmp.width, h: bmp.height, size: blob.size, nonBlank, buf };
  });
  fs.writeFileSync(SP + '/board-clip.png', Buffer.from(info.buf));
  assert.equal(info.type, 'image/png'); assert(info.w >= 900 && info.w === info.h, 'square image'); assert(info.nonBlank > 3000);
  console.log('OK   clipboard has a', info.w + 'x' + info.h, 'PNG,', info.size, 'bytes');
  // typing S in an input must not trigger it
  await p.goto(B + '/play/online', { waitUntil: 'networkidle' });
  await p.keyboard.press('s');
  await p.waitForTimeout(400);
  assert.equal(await p.locator('text=Board copied').count(), 0);
  console.log('OK   no board on lobby page, nothing happens');
  assert.deepEqual(errors, []);
  await browser.close();
  console.log('SCREENSHOT KEY PASSED');
})().catch((e) => { console.error('FAIL', e); process.exit(1); });
