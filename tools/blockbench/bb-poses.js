// Render a model in a list of poses from one camera angle.
// Usage: node bb-poses.js <model.bbmodel> <poses.json> <outdir> [port]   env: VIEW=front|side|iso|back, DIST, TARGET=x,y,z
// poses.json: [{"name": "...", "rot": {"bone": [x,y,z]}}, ...]  (rotations are ABSOLUTE bone rotations)
const { chromium } = require('/opt/node-tools/node_modules/playwright');
const fs = require('fs'), path = require('path');
(async () => {
  const [model, poses, outdir, port = '9222'] = process.argv.slice(2);
  fs.mkdirSync(outdir, { recursive: true });
  const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const page = browser.contexts()[0].pages().find(p => p.url().includes('index.html')) || browser.contexts()[0].pages()[0];
  await page.waitForFunction(() => window.Blockbench && window.Codecs, null, { timeout: 60000 });
  await page.evaluate(() => { for (const p of [...ModelProject.all]) p.close(true); });
  await page.evaluate(([name, content, fp]) => loadModelFile({ name, path: fp, content }), [path.basename(model), fs.readFileSync(model, 'utf8'), path.resolve(model)]);
  await page.waitForTimeout(1500);
  const dist = parseFloat(process.env.DIST || '55');
  const [tx, ty, tz] = (process.env.TARGET || '0,15,0').split(',').map(Number);
  const dirs = { front: [0, 0.12, -1], back: [0, 0.12, 1], side: [1, 0.12, 0], left: [-1, 0.12, 0], iso: [0.75, 0.35, -0.8], front_r: [0.5, 0.15, -1] };
  const d = dirs[process.env.VIEW || 'iso']; const n = Math.hypot(...d);
  await page.evaluate(([pos, target]) => Preview.selected.loadAnglePreset({ projection: 'perspective', position: pos, target: target }), [[tx + d[0] / n * dist, ty + d[1] / n * dist, tz + d[2] / n * dist], [tx, ty, tz]]);
  const list = JSON.parse(fs.readFileSync(poses, 'utf8'));
  const box = await page.evaluate(() => { const r = document.querySelector('#preview canvas, .preview canvas')?.getBoundingClientRect(); return r ? { x: r.x, y: r.y, width: r.width, height: r.height } : null; });
  for (let i = 0; i < list.length; i++) {
    await page.evaluate((pose) => {
      for (const g of Group.all) g.rotation = [0, 0, 0];
      for (const [name, r] of Object.entries(pose.rot || {})) { const g = Group.all.find(x => x.name === name); if (g) g.rotation = r; }
      Canvas.updateAllBones();
    }, list[i]);
    await page.waitForTimeout(250);
    await page.screenshot({ path: path.join(outdir, `${String(i).padStart(3, '0')}_${list[i].name || 'pose'}.png`), ...(box ? { clip: box } : {}) });
  }
  console.log('rendered', list.length, 'poses');
  await browser.close();
})().catch(e => { console.error('POSES FAILED:', e.message); process.exit(1); });
