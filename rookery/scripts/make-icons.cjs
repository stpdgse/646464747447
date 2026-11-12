// Renders the app icons (PNG) from one SVG using the Chromium that Playwright already provides.
// Run: node scripts/make-icons.cjs   (writes public/icons/*)
const { chromium } = require("playwright");
const { mkdirSync } = require("node:fs");
const path = require("node:path");

const rook = (bg, pad) => `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <rect width="512" height="512" fill="${bg}"/>
  <g transform="translate(256 256) scale(${pad}) translate(-256 -256)" fill="#7fa650">
    <polygon points="156,118 200,118 200,156 232,156 232,118 280,118 280,156 312,156 312,118 356,118 356,200 332,200 344,328 366,328 366,362 386,362 386,396 126,396 126,362 146,362 146,328 168,328 180,200 156,200"/>
  </g>
</svg>`;

const out = path.join(__dirname, "../public/icons/");
mkdirSync(out, { recursive: true });
(async () => {
const browser = await chromium.launch();
const shots = [
  ["icon-192.png", 192, "#161512", 0.9],
  ["icon-512.png", 512, "#161512", 0.9],
  ["apple-touch-icon.png", 180, "#161512", 0.9],
  ["maskable-512.png", 512, "#161512", 0.62], // keeps the rook inside the maskable safe zone
];
for (const [name, size, bg, pad] of shots) {
  const page = await browser.newPage({ viewport: { width: size, height: size } });
  await page.setContent(`<body style="margin:0">${rook(bg, pad).replace('width="512" height="512"', `width="${size}" height="${size}"`)}</body>`);
  await page.screenshot({ path: out + name, omitBackground: false });
  await page.close();
}
await browser.close();
console.log("icons written to", out);
})();
