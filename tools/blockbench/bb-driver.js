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
  fs.writeFileSync(path.join(outdir, 'result.json'), JSON.stringify(results, null, 1));
  await browser.close();
})().catch(e => { console.error('DRIVER FAILED:', e.message); process.exit(1); });
