// Ground-truth forward kinematics from Blockbench's scene graph.
// Usage: node bb-probe.js <model.bbmodel> <poses.json> [port]   poses: [{"rot": {"bone": [x,y,z]}, "pos": {"bone": [x,y,z]}, "read": ["cubeName", ...]}]
const { chromium } = require('/opt/node-tools/node_modules/playwright');
const fs = require('fs'), path = require('path');
(async () => {
  const [model, poses, port = '9222'] = process.argv.slice(2);
  const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  const page = browser.contexts()[0].pages().find(p => p.url().includes('index.html')) || browser.contexts()[0].pages()[0];
  await page.waitForFunction(() => window.Blockbench && window.Codecs, null, { timeout: 60000 });
  await page.evaluate(() => { for (const p of [...ModelProject.all]) p.close(true); });
  const content = fs.readFileSync(model, 'utf8');
  await page.evaluate(([name, content, fp]) => loadModelFile({ name, path: fp, content }), [path.basename(model), content, path.resolve(model)]);
  await page.waitForTimeout(1500);
  const out = await page.evaluate((poses) => {
    const results = [];
    for (const pose of poses) {
      for (const g of Group.all) { g.rotation = [0, 0, 0]; }
      for (const [name, r] of Object.entries(pose.rot || {})) { const g = Group.all.find(x => x.name === name); if (g) g.rotation = r; }
      Canvas.updateAllBones();
      const res = {};
      for (const cn of pose.read) {
        const c = Cube.all.find(x => x.name === cn);
        const box = new THREE.Box3().setFromObject(c.mesh);
        const v = new THREE.Vector3(); box.getCenter(v);
        res[cn] = [v.x, v.y, v.z];
      }
      results.push(res);
    }
    return results;
  }, JSON.parse(fs.readFileSync(poses, 'utf8')));
  console.log(JSON.stringify(out));
  await browser.close();
})().catch(e => { console.error('PROBE FAILED:', e.message); process.exit(1); });
