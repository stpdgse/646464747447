// Interaction test: manual square fix via the palette, unsure-square count, drag the crop box, iPad portrait layout.
// usage: NODE_PATH=$(npm root -g) node test/e2e-interact.cjs <screenshot> <out-png> [url]
const { chromium } = require('playwright');
const path = require('path');

const [, , shot = 'test/fixtures/ipad-start.png', out = '/tmp/e2e-interact.png', url = 'http://localhost:4173/'] = process.argv;

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 820, height: 1180 }, deviceScaleFactor: 1, hasTouch: true });
  const errs = [];
  page.on('pageerror', (e) => errs.push('pageerror: ' + e.message));
  await page.goto(url);
  await page.waitForFunction(() => /ready|Could not/.test(document.getElementById('engineStatus').textContent), null, { timeout: 60000 });
  const settled = () => page.waitForFunction(() => /Done|Stopped|Checkmate|Stalemate|Fix the position/.test(document.getElementById('msg').textContent), null, { timeout: 90000 });
  await page.setInputFiles('#file', path.resolve(shot));
  await settled();
  console.log('unsure squares on a clean read:', await page.$$eval('#board button.unsure', (b) => b.length));

  // 1. Fix a square: remove the e2 pawn through the palette.
  await page.click('#board button[aria-label="e2"]');
  console.log('palette open:', await page.isVisible('#palette'));
  await page.click('#palette button[value=empty]');
  await page.waitForFunction(() => document.getElementById('fen').textContent.startsWith('rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/'));
  await settled();
  console.log('after removing e2 pawn:', await page.textContent('#fen'));
  console.log('edited markers:', await page.$$eval('#board button.edited', (b) => b.length), '| undo visible:', await page.isVisible('#resetEdits'));
  const first = await page.$eval('#lines .line .mv', (e) => e.textContent);
  console.log('top move now:', first);
  await page.click('#resetEdits');
  await page.waitForFunction(() => document.getElementById('fen').textContent.includes('PPPPPPPP'));

  // 2. Opponent to move => same board, black to move.
  await page.click('#segMove button[data-v=opp]');
  await page.waitForFunction(() => document.getElementById('fen').textContent.split(' ')[1] === 'b');
  await settled();
  console.log('opponent to move, top move:', await page.$eval('#lines .line .mv', (e) => e.textContent));

  // 3. Drag the crop box a little (touch/pointer) and confirm the reading is still right.
  const box = await page.$('#box');
  const bb = await box.boundingBox();
  await page.mouse.move(bb.x + bb.width / 2, bb.y + bb.height / 2);
  await page.mouse.down();
  await page.mouse.move(bb.x + bb.width / 2 + 3, bb.y + bb.height / 2 + 2, { steps: 4 });
  await page.mouse.up();
  await page.waitForTimeout(1500);
  await settled();
  console.log('after nudging the box:', (await page.textContent('#fen')).split(' ')[0]);
  await page.screenshot({ path: out, fullPage: true });
  console.log('errors:', errs.length ? errs : 'none');
  await browser.close();
})();
