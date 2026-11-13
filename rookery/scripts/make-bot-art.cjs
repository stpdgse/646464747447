// Draws the pixel-art bot portraits (Der Mann and Der Mann (Rüdenrache)) to public/bots/*.png.
// Run: node scripts/make-bot-art.cjs
const { chromium } = require("playwright");
const path = require("node:path");
const fs = require("node:fs");

const BASE = [
  "................",
  "....HHHHHHHH....",
  "...HHHHHHHHHH...",
  "...HSSSSSSSSH...",
  "...SSSSSSSSSS...",
  "...SSHHSSHHSS...",
  "...SSWESSWESS...",
  "...SSSSSSSSSS...",
  "...BBSSSSSSBB...",
  "...BBBBBBBBBB...",
  "...BBBMMMMBBB...",
  "....BBBBBBBB....",
  "....PPSSSSPP....",
  "..PPPPPPPPPPPP..",
  "..PPPPPRRPPPPP..",
  "..PPPPPPPPPPPP..",
];
const WOLF = BASE.slice();
WOLF[0] = "..HH........HH..";
WOLF[1] = "..HHHHHHHHHHHH..";
WOLF[5] = "...SHHHSSHHHSS..".slice(0, 16);
WOLF[10] = "...BBBMBBMBBB...";

const PAL = { H: "#2b2118", S: "#d9a679", B: "#4d3b30", W: "#ffffff", E: "#1b1b1b", M: "#f4f4f4", P: "#161616", R: "#d93a2f" };
const WOLF_PAL = { ...PAL, E: "#ff2a2a", W: "#ffd9d9", H: "#1a1410", M: "#ffffff" };

for (const g of [BASE, WOLF]) for (const r of g) if (r.length !== 16) throw new Error("bad row " + JSON.stringify(r));

(async () => {
  const browser = await chromium.launch();
  const out = path.join(__dirname, "../public/bots");
  fs.mkdirSync(out, { recursive: true });
  for (const [name, grid, pal, bg] of [["dermann", BASE, PAL, "#2f3b24"], ["ruedenrache", WOLF, WOLF_PAL, "#4a1414"]]) {
    const page = await browser.newPage();
    const data = await page.evaluate(({ grid, pal, bg }) => {
      const S = 12, c = document.createElement("canvas");
      c.width = c.height = 16 * S;
      const x = c.getContext("2d");
      x.fillStyle = bg; x.fillRect(0, 0, c.width, c.height);
      grid.forEach((row, y) => [...row].forEach((ch, i) => { if (pal[ch]) { x.fillStyle = pal[ch]; x.fillRect(i * S, y * S, S, S); } }));
      return c.toDataURL("image/png");
    }, { grid, pal, bg });
    fs.writeFileSync(path.join(out, name + ".png"), Buffer.from(data.split(",")[1], "base64"));
    await page.close();
  }
  await browser.close();
  console.log("bot art written to", out);
})();
