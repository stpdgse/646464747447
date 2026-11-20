// Render frames of a Bedrock animation file on a model, using Blockbench's own animator (what you see is what the export does).
// Usage: node bb-anim.js <model.bbmodel> <anim.json> <spec.json> <outdir> [port]   env: VIEW, DIST, TARGET
// spec.json: [{"anim":"fire","t":0.1,"name":"fire_a"}, ...]
const { chromium } = require('/opt/node-tools/node_modules/playwright');
const fs = require('fs'), path = require('path');
(async () => {
  const [model, animFile, specFile, outdir, port = '9222'] = process.argv.slice(2);
  fs.mkdirSync(outdir, { recursive: true });
  const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const page = browser.contexts()[0].pages().find(p => p.url().includes('index.html')) || browser.contexts()[0].pages()[0];
  await page.waitForFunction(() => window.Blockbench && window.Codecs, null, { timeout: 60000 });
  await page.evaluate(() => { for (const p of [...ModelProject.all]) p.close(true); });
  await page.evaluate(([name, content, fp]) => loadModelFile({ name, path: fp, content }), [path.basename(model), fs.readFileSync(model, 'utf8'), path.resolve(model)]);
  await page.waitForTimeout(1500);
  const n = await page.evaluate(([content, fp]) => { Animator.loadFile({ name: 'a.animation.json', path: fp, content }); return Animation.all.length; }, [fs.readFileSync(animFile, 'utf8'), path.resolve(animFile)]);
  console.log('animations loaded:', n);
  const dist = parseFloat(process.env.DIST || '55');
  const [tx, ty, tz] = (process.env.TARGET || '0,15,0').split(',').map(Number);
  const dirs = { front: [0, 0.12, -1], back: [0, 0.12, 1], side: [1, 0.12, 0], left: [-1, 0.12, 0], iso: [0.75, 0.35, -0.8], front_r: [0.5, 0.15, -1] };
  const d = dirs[process.env.VIEW || 'iso']; const nn = Math.hypot(...d);
  await page.evaluate(([pos, target]) => Preview.selected.loadAnglePreset({ projection: 'perspective', position: pos, target: target }), [[tx + d[0] / nn * dist, ty + d[1] / nn * dist, tz + d[2] / nn * dist], [tx, ty, tz]]);
  const box = await page.evaluate(() => { const r = document.querySelector('#preview canvas, .preview canvas')?.getBoundingClientRect(); return r ? { x: r.x, y: r.y, width: r.width, height: r.height } : null; });
  const spec = JSON.parse(fs.readFileSync(specFile, 'utf8'));
  for (let i = 0; i < spec.length; i++) {
    const ok = await page.evaluate((s) => {
      const a = Animation.all.find(x => x.name === 'animation.tank.' + s.anim || x.name.endsWith('.' + s.anim));
      if (!a) return false;
      a.select(); Timeline.setTime(s.t); Animator.preview(); return true;
    }, spec[i]);
    if (!ok) console.log('missing animation', spec[i].anim);
    await page.waitForTimeout(250);
    await page.screenshot({ path: path.join(outdir, `${String(i).padStart(3, '0')}_${spec[i].name}.png`), ...(box ? { clip: box } : {}) });
  }
  console.log('rendered', spec.length);
  await browser.close();
})().catch(e => { console.error('ANIM FAILED:', e.message); process.exit(1); });
