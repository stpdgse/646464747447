// Drive a running Blockbench over CDP: open a .bbmodel, screenshot it, export formats.
// Usage: node bb-driver.js <model.bbmodel> <outdir> [port]
const { chromium } = require('/opt/node-tools/node_modules/playwright');
const fs = require('fs'), path = require('path');

(async () => {
  const [model, outdir, port = '9222'] = process.argv.slice(2);
  fs.mkdirSync(outdir, { recursive: true });
  const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const page = browser.contexts()[0].pages().find(p => p.url().includes('index.html')) || browser.contexts()[0].pages()[0];
  await page.waitForFunction(() => window.Blockbench && window.Project !== undefined && window.Codecs, null, { timeout: 60000 });
  await page.evaluate(() => { for (const p of [...ModelProject.all]) p.close(true); });
  const content = fs.readFileSync(model, 'utf8');
  await page.evaluate(([name, content, fp]) => loadModelFile({ name, path: fp, content }), [path.basename(model), content, path.resolve(model)]);
  await page.waitForTimeout(1500);
  const info = await page.evaluate(() => ({
    name: Project.name, format: Format.id, cubes: Cube.all.length, groups: Group.all.length,
    textures: Texture.all.length, errors: Blockbench.errors || 0,
  }));
  console.log('loaded:', JSON.stringify(info));
  const results = { info, exports: {} };
  // export the model through Blockbench's own codecs
  for (const id of ['project', 'java_block', 'bedrock', 'obj']) {
    try {
      const out = await page.evaluate(async id => {
        const c = Codecs[id]; if (!c) return null;
        const r = c.compile(); return typeof r === 'string' ? r : JSON.stringify(r);
      }, id);
      if (out) { const f = path.join(outdir, `export_${id}.txt`); fs.writeFileSync(f, out); results.exports[id] = out.length; }
    } catch (e) { results.exports[id] = 'ERR ' + e.message.split('\n')[0]; }
  }
  console.log('exports:', JSON.stringify(results.exports));
  await page.screenshot({ path: path.join(outdir, 'screenshot.png') });
  // optional: render the viewport from several angles (VIEWS=front,side,back,iso, DIST=<units>, TARGET=x,y,z)
  if (process.env.VIEWS) {
    const dist = parseFloat(process.env.DIST || '60');
    const [tx, ty, tz] = (process.env.TARGET || '0,16,0').split(',').map(Number);
    const dirs = { front: [0, 0.15, -1], back: [0, 0.15, 1], side: [1, 0.15, 0], left: [-1, 0.15, 0], iso: [0.7, 0.45, -0.8], top: [0, 1, -0.05] };
    for (const v of process.env.VIEWS.split(',')) {
      const d = dirs[v]; if (!d) continue;
      const n = Math.hypot(...d);
      await page.evaluate(([pos, target]) => {
        Preview.selected.loadAnglePreset({ projection: 'perspective', position: pos, target: target });
      }, [[tx + d[0] / n * dist, ty + d[1] / n * dist, tz + d[2] / n * dist], [tx, ty, tz]]);
      await page.waitForTimeout(500);
      const box = await page.evaluate(() => { const r = document.querySelector('#preview canvas, .preview canvas')?.getBoundingClientRect(); return r ? { x: r.x, y: r.y, width: r.width, height: r.height } : null; });
      await page.screenshot({ path: path.join(outdir, `view_${v}.png`), ...(box ? { clip: box } : {}) });
    }
  }
  fs.writeFileSync(path.join(outdir, 'result.json'), JSON.stringify(results, null, 1));
  await browser.close();
})().catch(e => { console.error('DRIVER FAILED:', e.message); process.exit(1); });
