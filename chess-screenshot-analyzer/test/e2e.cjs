// End-to-end: upload a screenshot into the built app in headless Chromium and print what the user would see.
// usage: NODE_PATH=$(npm root -g) node test/e2e.cjs <screenshot> <w|b> <out-png-prefix> [url]
const { chromium } = require('playwright');
const path = require('path');

const [, , shot = 'test/fixtures/ipad-start.png', playAs = 'w', out = '/tmp/e2e', url = 'http://localhost:4173/'] = process.argv;

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1180, height: 1500 }, deviceScaleFactor: 1 });
  const errs = [];
  page.on('pageerror', (e) => errs.push('pageerror: ' + e.message));
  page.on('console', (m) => { if (m.type() === 'error') errs.push('console: ' + m.text()); });
  await page.goto(url);
  await page.waitForFunction(() => /ready|Could not/.test(document.getElementById('engineStatus').textContent), null, { timeout: 60000 });
  console.log('engine:', await page.textContent('#engineStatus'));
  const settled = () => page.waitForFunction(
    () => /Done|Stopped|Checkmate|Stalemate|Fix the position/.test(document.getElementById('msg').textContent),
    null,
    { timeout: 90000 },
  );
  const t0 = Date.now();
  await page.setInputFiles('#file', path.resolve(shot));
  await settled();
  if (playAs === 'b') {
    // The controls appear after upload; switching colour re-reads the board and re-analyses.
    await page.click('#segPlay button[data-v=b]');
    await page.waitForFunction(() => document.getElementById('fen').textContent.split(' ')[1] === 'b');
    await settled();
  }
  console.log(`took ${Date.now() - t0}ms`);
  console.log('FEN   :', await page.textContent('#fen'));
  console.log('theme :', await page.textContent('#themeNote'));
  console.log('notice:', (await page.isVisible('#notice')) ? await page.textContent('#notice') : '-');
  console.log('msg   :', await page.textContent('#msg'));
  for (const l of await page.$$('#lines .line')) console.log('  ', (await l.innerText()).replace(/\n+/g, ' | '));
  console.log('eval  :', await page.textContent('#evalText'));
  await page.screenshot({ path: out + '.png', fullPage: true });
  console.log('errors:', errs.length ? errs : 'none');
  await browser.close();
})();
