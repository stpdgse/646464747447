// Pack each piece set (12 PNGs of 150x150) into one 1800x150 sprite: public/pieces/<theme>.png
// Order: wp wn wb wr wq wk bp bn bb br bq bk
import fs from 'node:fs';
import path from 'node:path';
import { PNG } from 'pngjs';

const SRC = path.resolve(import.meta.dirname, '../assets/pieces-src');
const OUT = path.resolve(import.meta.dirname, '../public/pieces');
const CODES = ['wp', 'wn', 'wb', 'wr', 'wq', 'wk', 'bp', 'bn', 'bb', 'br', 'bq', 'bk'];
fs.mkdirSync(OUT, { recursive: true });
for (const theme of fs.readdirSync(SRC)) {
  if (!fs.statSync(path.join(SRC, theme)).isDirectory()) continue;
  const sheet = new PNG({ width: 150 * 12, height: 150 });
  CODES.forEach((code, i) => {
    const p = PNG.sync.read(fs.readFileSync(path.join(SRC, theme, code + '.png')));
    if (p.width !== 150 || p.height !== 150) throw new Error(`${theme}/${code} is ${p.width}x${p.height}`);
    PNG.bitblt(p, sheet, 0, 0, 150, 150, i * 150, 0);
  });
  fs.writeFileSync(path.join(OUT, theme + '.png'), PNG.sync.write(sheet));
}
console.log('sprites written to', OUT);
